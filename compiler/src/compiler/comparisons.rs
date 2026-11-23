use querydown_parser::ast::*;

use crate::{
    compiler::expr::{convert_definition_with_env, convert_expr},
    errors::msg,
    schema::ValueType,
    sql::{
        expr::build::*,
        tree::{CtePurpose, SqlExpr},
        RegexPattern,
    },
    Error,
};

use super::{
    constants::LINKED_RECORD_COMPARISON_NAME,
    paths::{clarify_path, ClarifiedPathTail},
    scope::{BindingContext, Expansion, Scope},
    temporal::{now_operand, reconcile, TemporalZone},
    typing::infer_type,
};

pub fn convert_comparison(c: Comparison, scope: &mut Scope) -> Result<SqlExpr, Error> {
    use ComparisonSide::{Expansion as CmpExpansion, Expr as CmpExpr, Range as CmpRange};

    // An expansion on the left distributes the whole comparison across its entries. Each entry is
    // then compared just as it would be if it were written alone on the left, so it may name a
    // custom comparison or a linked record. (With an expansion on the right too, this makes the
    // left one the outer of the two.)
    let left = match c.left {
        CmpExpansion(conditions) => {
            let exprs = conditions
                .entries
                .into_iter()
                .map(|entry| {
                    let comparison = Comparison {
                        left: CmpExpr(entry),
                        operator: c.operator,
                        right: c.right.clone(),
                    };
                    convert_comparison(comparison, scope)
                })
                .collect::<Result<Vec<_>, _>>()?;
            return Ok(cmp::condition_set(exprs, &conditions.conjunction));
        }
        left => left,
    };

    // A custom comparison is invoked when the left-hand side is a path naming a registered custom
    // comparison. When one matches, its body is expanded in place of the whole comparison.
    if scope.has_custom_comparisons() {
        if let ComparisonSide::Expr(Expr {
            kind: ExprKind::Path(parts),
            ..
        }) = &left
        {
            let prefixed = scope.prefixed(parts);
            if let Some((head_parts, def)) = resolve_custom_comparison(&prefixed, scope)? {
                return convert_custom_comparison(def, head_parts, c.operator, c.right, scope);
            }
            // Failing an explicitly-named custom comparison, a comparison whose left-hand side is a
            // path to a single linked record can defer to that record's table-wide linked-record
            // comparison, e.g. `author:alice` becoming `author.username:alice`.
            if let Some((head_parts, def)) = resolve_linked_record_comparison(&prefixed, scope)? {
                return convert_custom_comparison(def, head_parts, c.operator, c.right, scope);
            }
        }
    }

    let mut simple = |l: &Expr, r: &Expr| convert_simple_comparison(l, c.operator, r, scope);

    match (left, c.right) {
        // Two expressions
        (CmpExpr(left), CmpExpr(right)) => simple(&left, &right),

        // Expression vs expansion
        (CmpExpr(ref left), CmpExpansion(conditions)) => conditions
            .entries
            .iter()
            .map(|right| simple(left, right))
            .collect::<Result<Vec<_>, _>>()
            .map(|exprs| cmp::condition_set(exprs, &conditions.conjunction)),

        // Range vs Expr
        (CmpRange(range), CmpExpr(expr)) | (CmpExpr(expr), CmpRange(range)) => {
            if !matches!(c.operator, Operator::Eq | Operator::EqCi | Operator::Match) {
                return Err(msg::compare_range_without_eq());
            }
            convert_range_comparison(&expr, &range, scope)
        }

        // Range vs Expansion
        (CmpRange(r), CmpExpansion(conditions)) => {
            if !matches!(c.operator, Operator::Eq | Operator::EqCi | Operator::Match) {
                return Err(msg::compare_range_without_eq());
            }
            conditions
                .entries
                .iter()
                .map(|expr| convert_range_comparison(expr, &r, scope))
                .collect::<Result<Vec<_>, _>>()
                .map(|exprs| cmp::condition_set(exprs, &conditions.conjunction))
        }

        // Two ranges
        (CmpRange(_), CmpRange(_)) => Err(msg::compare_two_ranges()),

        // A left expansion was distributed above.
        (CmpExpansion(_), _) => unreachable!("a left expansion is distributed before this"),
    }
}

/// Whether `expr` is a literal that isn't a condition: text, a number, a date, or a duration.
pub(super) fn is_literal_value(expr: &Expr) -> bool {
    matches!(
        expr.kind,
        ExprKind::String(_)
            | ExprKind::FormatString(_)
            | ExprKind::Number(_)
            | ExprKind::Date(_)
            | ExprKind::Duration(_)
    )
}

/// Whether a condition set evidently holds values (as the expansion `[open reopened]` does) rather
/// than conditions: whether any of its entries is a literal value, negated or not.
fn holds_values(condition_set: &ConditionSet) -> bool {
    condition_set.entries.iter().any(|entry| {
        let mut entry = entry;
        while let ExprKind::Not(inner) = &entry.kind {
            entry = inner;
        }
        is_literal_value(entry)
    })
}

fn convert_simple_comparison(
    left: &Expr,
    operator: Operator,
    right: &Expr,
    scope: &mut Scope,
) -> Result<SqlExpr, Error> {
    use Operator::*;

    // A few literals have a special meaning in a comparison: zero, `@null`, a duration, and an
    // empty string. They keep that meaning when they arrive through a constant or a parameter, so
    // the checks below look at what each side stands for.
    let left_form = scope.see_through_variables(left);
    let right_form = scope.see_through_variables(right);

    // Only a set written directly in a comparison is an expansion. One that arrives some other way
    // (through a constant, say) is a single boolean value, which is surely not what was meant when
    // it lists values.
    for (side, form) in [(left, &left_form), (right, &right_form)] {
        if matches!(&form.kind, ExprKind::ConditionSet(set) if holds_values(set)) {
            return Err(msg::condition_set_as_value().at(side.loc));
        }
    }

    // The match operator (`:`) and the case-insensitive equality operator (`:=`, `EqCi`) both behave
    // like `Eq` for all the special cases below; they only diverge from strict equality for plain
    // text value comparisons (handled in the final dispatch).
    let eq_like = matches!(operator, Eq | EqCi | Match);

    if left_form.is_zero() && eq_like {
        return convert_expression_vs_zero(right, ComparisonVsZero::Eq, scope);
    }
    if left_form.is_zero() && operator == Lt {
        return convert_expression_vs_zero(right, ComparisonVsZero::Gt, scope);
    }
    if right_form.is_zero() && eq_like {
        return convert_expression_vs_zero(left, ComparisonVsZero::Eq, scope);
    }
    if right_form.is_zero() && operator == Gt {
        return convert_expression_vs_zero(left, ComparisonVsZero::Gt, scope);
    }

    if right_form.is_null() && eq_like {
        return convert_expr(left.to_owned(), scope).map(cmp::is_null);
    }
    if left_form.is_null() && eq_like {
        return convert_expr(right.to_owned(), scope).map(cmp::is_null);
    }

    let left_type = infer_type(left, scope);
    let right_type = infer_type(right, scope);

    // Date/duration magic: comparing a date-like value on the left against a duration on the right
    // means "within that duration of now" — i.e. `ABS(now - date) <op> duration` — which reads
    // intuitively whether the date lies in the past or the future. The reversed order (duration on
    // the left, date on the right) is deliberately left alone, producing the usual database type
    // error just as a direct date-vs-duration comparison does today.
    //
    // Both sides are recognized by their inferred types, so the date may be any expression that
    // works out to one (`created_at%max`), and likewise the duration (`1w + 1d`, or a constant).
    if left_type.is_temporal() && right_type == ValueType::Duration {
        if let Some(build_cmp) = numeric_comparison(operator) {
            let date_zone = TemporalZone::of(&left_type);
            let date = convert_expr(left.to_owned(), scope)?;
            let duration = convert_expr(right.to_owned(), scope)?;
            // Reconcile `@now` and the date so the subtraction is valid on dialects that can't mix a
            // zoned and a naive temporal value.
            let (now_sql, now_zone) = now_operand(scope.options.dialect.as_ref());
            let (now, date) = reconcile(
                (now_sql, now_zone),
                (date, date_zone),
                scope.options.dialect.as_ref(),
            );
            let elapsed = math::abs(date_time::extract_epoch(math::subtract(now, date)));
            let limit = date_time::extract_epoch(duration);
            return Ok(build_cmp(elapsed, limit));
        }
    }

    let left_converted = convert_expr(left.to_owned(), scope)?;
    let right_converted = convert_expr(right.to_owned(), scope)?;

    // Reconcile the operands when a comparison mixes a zoned and a naive temporal value (e.g.
    // `created_at < @now|minus(6y)`), so the comparison is valid on dialects that can't mix the
    // two. This is a no-op for non-temporal comparisons and for dialects that reconcile natively.
    let left_zone = TemporalZone::of(&left_type);
    let right_zone = TemporalZone::of(&right_type);
    let (left_converted, right_converted) = reconcile(
        (left_converted, left_zone),
        (right_converted, right_zone),
        scope.options.dialect.as_ref(),
    );

    match &operator {
        Eq => Ok(cmp::eq(left_converted, right_converted)),
        // Case-insensitive equality (`:=`): when the left-hand side is known to be text, compare
        // case-insensitively (mirroring the normalization the match operator applies); otherwise
        // fall back to strict equality.
        EqCi => {
            if left_type == ValueType::Text {
                Ok(scope
                    .options
                    .dialect
                    .text_eq(left_converted, right_converted))
            } else {
                Ok(cmp::eq(left_converted, right_converted))
            }
        }
        Gt => Ok(cmp::gt(left_converted, right_converted)),
        Gte => Ok(cmp::gte(left_converted, right_converted)),
        Lt => Ok(cmp::lt(left_converted, right_converted)),
        Lte => Ok(cmp::lte(left_converted, right_converted)),
        Like => Ok(cmp::like(left_converted, right_converted)),
        RegexMatch(flags) => {
            // A pattern that is a string literal (or a variable bound to one, e.g. a custom
            // comparison's parameter) is kept apart, so that the dialect can write flags into it.
            let pattern = match right_form.kind {
                ExprKind::String(s) => RegexPattern::Literal(s),
                _ => RegexPattern::Expr(right_converted),
            };
            Ok(scope
                .options
                .dialect
                .match_regex(left_converted, pattern, flags))
        }
        // The match operator applies type-aware matching: case-insensitive "contains" when the
        // left-hand side is known to be text, falling back to exact equality otherwise. An empty
        // string literal on the right is a special case: "contains" would match every row, so we
        // use strict equality instead, making `description:""` find rows with an empty description
        // (mirroring `description:=""`).
        Match => {
            let right_is_empty_string =
                matches!(&right_form.kind, ExprKind::String(s) if s.is_empty());
            if !right_is_empty_string && left_type == ValueType::Text {
                Ok(scope
                    .options
                    .dialect
                    .text_contains(left_converted, right_converted))
            } else {
                Ok(cmp::eq(left_converted, right_converted))
            }
        }
    }
}

/// Maps a comparison operator to the SQL builder for comparing two numeric (epoch-second) operands,
/// used by the date/duration magic. Operators that don't define an ordering have no mapping, so the
/// magic doesn't apply to them.
fn numeric_comparison(operator: Operator) -> Option<fn(SqlExpr, SqlExpr) -> SqlExpr> {
    use Operator::*;
    match operator {
        Eq | EqCi | Match => Some(cmp::eq),
        Gt => Some(cmp::gt),
        Gte => Some(cmp::gte),
        Lt => Some(cmp::lt),
        Lte => Some(cmp::lte),
        Like | RegexMatch(_) => None,
    }
}

fn convert_range_comparison(
    expr: &Expr,
    range: &Range,
    scope: &mut Scope,
) -> Result<SqlExpr, Error> {
    let lower_op = match range.lower.exclusivity {
        Exclusivity::Inclusive => Operator::Gte,
        Exclusivity::Exclusive => Operator::Gt,
    };
    let lower = convert_simple_comparison(expr, lower_op, &range.lower.expr, scope)?;

    let upper_op = match range.upper.exclusivity {
        Exclusivity::Inclusive => Operator::Lte,
        Exclusivity::Exclusive => Operator::Lt,
    };
    let upper = convert_simple_comparison(expr, upper_op, &range.upper.expr, scope)?;

    Ok(cmp::condition_set([lower, upper], &Conjunction::And))
}

#[derive(Clone, Copy)]
enum ComparisonVsZero {
    Eq,
    Gt,
}

impl From<ComparisonVsZero> for Operator {
    fn from(cmp: ComparisonVsZero) -> Self {
        match cmp {
            ComparisonVsZero::Eq => Operator::Eq,
            ComparisonVsZero::Gt => Operator::Gt,
        }
    }
}

impl From<ComparisonVsZero> for CtePurpose {
    fn from(cmp: ComparisonVsZero) -> Self {
        match cmp {
            ComparisonVsZero::Eq => CtePurpose::Exclusion,
            ComparisonVsZero::Gt => CtePurpose::Inclusion,
        }
    }
}

fn convert_expression_vs_zero(
    expr: &Expr,
    cmp: ComparisonVsZero,
    scope: &mut Scope,
) -> Result<SqlExpr, Error> {
    let fallback = |scope: &mut Scope| {
        let op = match cmp {
            ComparisonVsZero::Eq => cmp::eq,
            ComparisonVsZero::Gt => cmp::gt,
        };
        Ok(op(convert_expr(expr.to_owned(), scope)?, value::zero()))
    };
    let ExprKind::Path(path_parts) = &expr.kind else {
        return fallback(scope);
    };
    // Resolve the path from the scope's current table, as `convert_path` does, so that an existence
    // check inside a scoped condition set (`author{++#comments}`) starts from the scoped table
    // rather than the base table.
    let prefixed_parts = scope.prefixed(path_parts);
    let Ok(clarified_path) = clarify_path(prefixed_parts, scope) else {
        return fallback(scope);
    };
    let Some(ClarifiedPathTail::ChainToMany((chain, None, _))) = clarified_path.tail else {
        return fallback(scope);
    };
    let join_result = scope.join_chain_to_many(&clarified_path.head, chain, None, cmp.into());
    let Ok(pk) = join_result else {
        return fallback(scope);
    };
    match cmp {
        ComparisonVsZero::Eq => Ok(cmp::is_null(pk)),
        ComparisonVsZero::Gt => Ok(cmp::is_not_null(pk)),
    }
}

/// If `parts` names a custom comparison, returns the head of the path (the parts leading to the
/// table hosting the comparison) together with the matched definition. A real column of the same
/// name takes precedence, in which case this returns `None`.
fn resolve_custom_comparison(
    parts: &[PathPart],
    scope: &Scope,
) -> Result<Option<(Vec<PathPart>, CustomComparisonDef)>, Error> {
    // Only a trailing plain column can name a custom comparison.
    let Some((
        PathPart {
            kind: PathPartKind::Column(name),
            ..
        },
        head_parts,
    )) = parts.split_last()
    else {
        return Ok(None);
    };
    // Determine the table hosting the comparison. Only the base table (empty head) or a single
    // related record reachable via the head can host one.
    let table = if head_parts.is_empty() {
        scope.get_base_table()
    } else {
        let clarified = clarify_path(head_parts.to_vec(), scope)?;
        match (clarified.head, clarified.tail) {
            (Some(chain_to_one), None) => scope
                .schema
                .tables
                .get(&chain_to_one.get_ending_table_id())
                .unwrap(),
            _ => return Ok(None),
        }
    };
    // A real column of the same name takes precedence over a custom comparison.
    if scope
        .options
        .resolve_identifier(&table.column_lookup, name)
        .is_some()
    {
        return Ok(None);
    }
    Ok(scope
        .get_custom_comparison(table.id, name)
        .map(|def| (head_parts.to_vec(), def.clone())))
}

/// If `parts` names a single linked record — a chain of to-one links ending at a table, with no
/// trailing column — whose target table defines a [`LINKED_RECORD_COMPARISON_NAME`] custom
/// comparison, returns the full path together with the matched definition. The path becomes the
/// prefix under which the comparison's body is evaluated, so `author:alice` expands the `users`
/// table's linked-record comparison as though written `author.<body>`.
fn resolve_linked_record_comparison(
    parts: &[PathPart],
    scope: &Scope,
) -> Result<Option<(Vec<PathPart>, CustomComparisonDef)>, Error> {
    // Only a path that resolves to a single linked record qualifies. A path ending in a column (or a
    // to-many chain, or an empty path) is handled by the ordinary comparison logic. A path that
    // fails to clarify is likewise left to the ordinary logic to surface the error.
    let Ok(clarified) = clarify_path(parts.to_vec(), scope) else {
        return Ok(None);
    };
    let table = match (clarified.head, clarified.tail) {
        (Some(chain_to_one), None) => scope
            .schema
            .tables
            .get(&chain_to_one.get_ending_table_id())
            .unwrap(),
        _ => return Ok(None),
    };
    Ok(scope
        .get_custom_comparison(table.id, LINKED_RECORD_COMPARISON_NAME)
        .map(|def| (parts.to_vec(), def.clone())))
}

/// Expands a custom comparison: the right-hand side value is bound to the definition's parameter and
/// the (possibly operator-switched) body is compiled in place of the comparison.
fn convert_custom_comparison(
    def: CustomComparisonDef,
    head_parts: Vec<PathPart>,
    call_operator: Operator,
    right: ComparisonSide,
    scope: &mut Scope,
) -> Result<SqlExpr, Error> {
    // The value belongs to the call, so its context is recorded before the comparison's expansion
    // is entered.
    let context = scope.context();
    let _guard = scope.enter_expansion(Expansion::CustomComparison {
        table: def.table.to_string(),
        name: def.name.clone(),
    })?;
    let body = switched_body(&def, call_operator)?;
    match right {
        ComparisonSide::Expr(value) => {
            convert_custom_comparison_body(&def.param, value, &context, body, &head_parts, scope)
        }
        // An expansion on the right binds the parameter to each entry in turn, combining the
        // results with the expansion's conjunction (e.g. `comment:["a" "b"]`).
        ComparisonSide::Expansion(conditions) => {
            let exprs = conditions
                .entries
                .into_iter()
                .map(|value| {
                    convert_custom_comparison_body(
                        &def.param,
                        value,
                        &context,
                        body.clone(),
                        &head_parts,
                        scope,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(cmp::condition_set(exprs, &conditions.conjunction))
        }
        ComparisonSide::Range(_) => Err(msg::custom_comparison_with_range(&def.name)),
    }
}

/// Compiles a custom comparison's `body`, relative to the table that hosts the comparison (which
/// `head_parts` leads to), with its parameter bound to `value`. The value means what it meant where
/// the comparison was called, which `context` records: a column it refers to is one of the table
/// the call was made on, not of the host table, nor of any table the body goes on to reach.
fn convert_custom_comparison_body(
    param: &str,
    value: Expr,
    context: &BindingContext,
    body: Expr,
    head_parts: &[PathPart],
    scope: &mut Scope,
) -> Result<SqlExpr, Error> {
    let env = scope.bind(
        &scope.globals(),
        param.to_string(),
        value,
        scope.env(),
        context.clone(),
    );
    convert_definition_with_env(body, head_parts.to_vec(), env, scope)
}

/// Returns the custom comparison's body, with its `:` (match) operators rewritten to `call_operator`
/// if the comparison is being called with a different operator than it was defined with. Operator
/// switching is only permitted when the definition uses `:` and every comparison in the body also
/// uses `:`.
fn switched_body(def: &CustomComparisonDef, call_operator: Operator) -> Result<Expr, Error> {
    if call_operator == def.operator {
        return Ok(def.body.clone());
    }
    if def.operator == Operator::Match && expr_all_match(&def.body) {
        let mut body = def.body.clone();
        rewrite_match_operator(&mut body, call_operator);
        return Ok(body);
    }
    Err(msg::custom_comparison_operator_mismatch(&def.name))
}

/// Whether every comparison anywhere within `expr` uses the match (`:`) operator.
fn expr_all_match(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Comparison(c) => {
            matches!(c.operator, Operator::Match)
                && side_all_match(&c.left)
                && side_all_match(&c.right)
        }
        ExprKind::Not(e) => expr_all_match(e),
        ExprKind::Product(a, b)
        | ExprKind::Quotient(a, b)
        | ExprKind::Sum(a, b)
        | ExprKind::Difference(a, b) => expr_all_match(a) && expr_all_match(b),
        ExprKind::ConditionSet(cs) => cs.entries.iter().all(expr_all_match),
        ExprKind::ScopedConditionSet(s) => {
            s.path.iter().all(pathpart_all_match)
                && s.condition_set.entries.iter().all(expr_all_match)
        }
        ExprKind::HasQuantity(h) => h.path_parts.iter().all(pathpart_all_match),
        ExprKind::Case(c) => {
            c.variants
                .iter()
                .all(|v| expr_all_match(&v.condition) && expr_all_match(&v.value))
                && expr_all_match(&c.fallback)
        }
        ExprKind::Call(c) => c.args.iter().all(expr_all_match),
        ExprKind::FormatString(parts) => parts.iter().filter_map(|p| p.expr()).all(expr_all_match),
        ExprKind::Path(parts) => parts.iter().all(pathpart_all_match),
        ExprKind::Window(w) => {
            w.args.iter().all(expr_all_match)
                && w.partition_by.iter().all(expr_all_match)
                && w.order_by.iter().all(|s| expr_all_match(&s.expr))
        }
        ExprKind::AnonymousFunctionCall(c) => {
            c.args.iter().all(expr_all_match)
                && c.body.assignments.iter().all(|a| expr_all_match(&a.expr))
                && expr_all_match(&c.body.expr)
        }
        ExprKind::FunctionArg(f) => {
            f.body.assignments.iter().all(|a| expr_all_match(&a.expr))
                && expr_all_match(&f.body.expr)
        }
        // A scalar subquery is self-contained; it does not participate in operator switching.
        ExprKind::Subquery(_) => true,
        ExprKind::Number(_)
        | ExprKind::Date(_)
        | ExprKind::Duration(_)
        | ExprKind::String(_)
        | ExprKind::Variable(_) => true,
    }
}

fn side_all_match(side: &ComparisonSide) -> bool {
    match side {
        ComparisonSide::Expr(e) => expr_all_match(e),
        ComparisonSide::Expansion(cs) => cs.entries.iter().all(expr_all_match),
        ComparisonSide::Range(r) => expr_all_match(&r.lower.expr) && expr_all_match(&r.upper.expr),
    }
}

fn pathpart_all_match(part: &PathPart) -> bool {
    match &part.kind {
        PathPartKind::TableWithMany(t) => t.condition_set.entries.iter().all(expr_all_match),
        PathPartKind::Column(_) | PathPartKind::TableWithOne(_) => true,
    }
}

/// Rewrites every match (`:`) operator within `expr` to `new_operator`, recursing through the same
/// structure that [`expr_all_match`] inspects.
fn rewrite_match_operator(expr: &mut Expr, new_operator: Operator) {
    match &mut expr.kind {
        ExprKind::Comparison(c) => {
            if matches!(c.operator, Operator::Match) {
                c.operator = new_operator;
            }
            rewrite_side(&mut c.left, new_operator);
            rewrite_side(&mut c.right, new_operator);
        }
        ExprKind::Not(e) => rewrite_match_operator(e, new_operator),
        ExprKind::Product(a, b)
        | ExprKind::Quotient(a, b)
        | ExprKind::Sum(a, b)
        | ExprKind::Difference(a, b) => {
            rewrite_match_operator(a, new_operator);
            rewrite_match_operator(b, new_operator);
        }
        ExprKind::ConditionSet(cs) => cs
            .entries
            .iter_mut()
            .for_each(|e| rewrite_match_operator(e, new_operator)),
        ExprKind::ScopedConditionSet(s) => {
            s.path
                .iter_mut()
                .for_each(|p| rewrite_pathpart(p, new_operator));
            s.condition_set
                .entries
                .iter_mut()
                .for_each(|e| rewrite_match_operator(e, new_operator));
        }
        ExprKind::HasQuantity(h) => h
            .path_parts
            .iter_mut()
            .for_each(|p| rewrite_pathpart(p, new_operator)),
        ExprKind::Case(c) => {
            for v in c.variants.iter_mut() {
                rewrite_match_operator(&mut v.condition, new_operator);
                rewrite_match_operator(&mut v.value, new_operator);
            }
            rewrite_match_operator(&mut c.fallback, new_operator);
        }
        ExprKind::Call(c) => c
            .args
            .iter_mut()
            .for_each(|e| rewrite_match_operator(e, new_operator)),
        ExprKind::FormatString(parts) => parts
            .iter_mut()
            .filter_map(|p| p.expr_mut())
            .for_each(|e| rewrite_match_operator(e, new_operator)),
        ExprKind::Path(parts) => parts
            .iter_mut()
            .for_each(|p| rewrite_pathpart(p, new_operator)),
        ExprKind::Window(w) => {
            w.args
                .iter_mut()
                .for_each(|e| rewrite_match_operator(e, new_operator));
            w.partition_by
                .iter_mut()
                .for_each(|e| rewrite_match_operator(e, new_operator));
            w.order_by
                .iter_mut()
                .for_each(|s| rewrite_match_operator(&mut s.expr, new_operator));
        }
        ExprKind::AnonymousFunctionCall(c) => {
            c.args
                .iter_mut()
                .for_each(|e| rewrite_match_operator(e, new_operator));
            c.body
                .assignments
                .iter_mut()
                .for_each(|a| rewrite_match_operator(&mut a.expr, new_operator));
            rewrite_match_operator(&mut c.body.expr, new_operator);
        }
        ExprKind::FunctionArg(f) => {
            f.body
                .assignments
                .iter_mut()
                .for_each(|a| rewrite_match_operator(&mut a.expr, new_operator));
            rewrite_match_operator(&mut f.body.expr, new_operator);
        }
        // A scalar subquery is self-contained; its inner operators are not rewritten.
        ExprKind::Subquery(_) => {}
        ExprKind::Number(_)
        | ExprKind::Date(_)
        | ExprKind::Duration(_)
        | ExprKind::String(_)
        | ExprKind::Variable(_) => {}
    }
}

fn rewrite_side(side: &mut ComparisonSide, new_operator: Operator) {
    match side {
        ComparisonSide::Expr(e) => rewrite_match_operator(e, new_operator),
        ComparisonSide::Expansion(cs) => cs
            .entries
            .iter_mut()
            .for_each(|e| rewrite_match_operator(e, new_operator)),
        ComparisonSide::Range(r) => {
            rewrite_match_operator(&mut r.lower.expr, new_operator);
            rewrite_match_operator(&mut r.upper.expr, new_operator);
        }
    }
}

fn rewrite_pathpart(part: &mut PathPart, new_operator: Operator) {
    if let PathPartKind::TableWithMany(t) = &mut part.kind {
        t.condition_set
            .entries
            .iter_mut()
            .for_each(|e| rewrite_match_operator(e, new_operator));
    }
}
