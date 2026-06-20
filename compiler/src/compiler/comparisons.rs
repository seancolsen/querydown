use querydown_parser::ast::*;

use crate::{
    compiler::constants::VAR_NOW,
    compiler::expr::{convert_expr, convert_expr_with_path_prefix},
    errors::msg,
    schema::ValueType,
    sql::{
        expr::build::*,
        tree::{CtePurpose, SqlExpr},
        RegExFlags,
    },
};

use super::{
    paths::{clarify_path, ClarifiedPathTail},
    scope::Scope,
};

pub fn convert_comparison(c: Comparison, scope: &mut Scope) -> Result<SqlExpr, String> {
    use ComparisonSide::{Expansion as CmpExpansion, Expr as CmpExpr, Range as CmpRange};

    // A custom comparison is invoked when the left-hand side is a path naming a registered custom
    // comparison. When one matches, its body is expanded in place of the whole comparison.
    if scope.has_custom_comparisons() {
        if let ComparisonSide::Expr(Expr::Path(parts)) = &c.left {
            let prefixed: Vec<PathPart> = scope
                .path_prefix
                .iter()
                .cloned()
                .chain(parts.iter().cloned())
                .collect();
            if let Some((head_parts, def)) = resolve_custom_comparison(&prefixed, scope)? {
                return convert_custom_comparison(def, head_parts, c.operator, c.right, scope);
            }
        }
    }

    let mut simple = |l: &Expr, r: &Expr| convert_simple_comparison(l, c.operator, r, scope);

    match (c.left, c.right) {
        // Two expressions
        (CmpExpr(left), CmpExpr(right)) => simple(&left, &right),

        // Expression vs expansion
        (CmpExpr(ref left), CmpExpansion(conditions)) => conditions
            .entries
            .iter()
            .map(|right| simple(left, right))
            .collect::<Result<Vec<_>, _>>()
            .map(|exprs| cmp::condition_set(exprs, &conditions.conjunction)),
        (CmpExpansion(conditions), CmpExpr(ref right)) => conditions
            .entries
            .iter()
            .map(|left| simple(left, right))
            .collect::<Result<Vec<_>, _>>()
            .map(|exprs| cmp::condition_set(exprs, &conditions.conjunction)),

        // Dual expansion
        (CmpExpansion(left_conditions), CmpExpansion(right_conditions)) => {
            let mut outer_entries = vec![];
            for left in left_conditions.entries.iter() {
                let mut inner_entries = vec![];
                for right in right_conditions.entries.iter() {
                    inner_entries.push(simple(left, right)?);
                }
                outer_entries.push(cmp::condition_set(
                    inner_entries,
                    &right_conditions.conjunction,
                ));
            }
            Ok(cmp::condition_set(
                outer_entries,
                &left_conditions.conjunction,
            ))
        }

        // Range vs Expr
        (CmpRange(range), CmpExpr(expr)) | (CmpExpr(expr), CmpRange(range)) => {
            if !matches!(c.operator, Operator::Eq | Operator::Match) {
                return Err(msg::compare_range_without_eq());
            }
            convert_range_comparison(&expr, &range, scope)
        }

        // Range vs Expansion
        (CmpExpansion(conditions), CmpRange(r)) | (CmpRange(r), CmpExpansion(conditions)) => {
            if !matches!(c.operator, Operator::Eq | Operator::Match) {
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
    }
}

fn convert_simple_comparison(
    left: &Expr,
    operator: Operator,
    right: &Expr,
    scope: &mut Scope,
) -> Result<SqlExpr, String> {
    use Operator::*;

    // The match operator (`:`) behaves like `Eq` for all the special cases below; it only diverges
    // from equality for plain text value comparisons (handled in the final dispatch).
    let eq_like = matches!(operator, Eq | Match);

    if left.is_zero() && eq_like {
        return convert_expression_vs_zero(right, ComparisonVsZero::Eq, scope);
    }
    if left.is_zero() && operator == Lt {
        return convert_expression_vs_zero(right, ComparisonVsZero::Gt, scope);
    }
    if right.is_zero() && eq_like {
        return convert_expression_vs_zero(left, ComparisonVsZero::Eq, scope);
    }
    if right.is_zero() && operator == Gt {
        return convert_expression_vs_zero(left, ComparisonVsZero::Gt, scope);
    }

    if right.is_null() && eq_like {
        return convert_expr(left.to_owned(), scope).map(cmp::is_null);
    }
    if left.is_null() && eq_like {
        return convert_expr(right.to_owned(), scope).map(cmp::is_null);
    }

    // Date/duration magic: comparing a date-like value on the left against a duration literal on the
    // right means "within that duration of now" — i.e. `ABS(now - date) <op> duration` — which reads
    // intuitively whether the date lies in the past or the future. The reversed order (duration on
    // the left, date on the right) is deliberately left alone, producing the usual database type
    // error just as a direct date-vs-duration comparison does today.
    if is_date_like(left, scope) && matches!(right, Expr::Duration(_)) {
        if let Some(build_cmp) = numeric_comparison(operator) {
            let date = convert_expr(left.to_owned(), scope)?;
            let duration = convert_expr(right.to_owned(), scope)?;
            let elapsed = math::abs(date_time::extract_epoch(math::subtract(func::now(), date)));
            let limit = date_time::extract_epoch(duration);
            return Ok(build_cmp(elapsed, limit));
        }
    }

    let left_converted = convert_expr(left.to_owned(), scope)?;
    let right_converted = convert_expr(right.to_owned(), scope)?;

    let match_regex = |a: SqlExpr, b: SqlExpr, scope: &mut Scope| {
        let flags = RegExFlags {
            is_case_sensitive: false,
        };
        scope.options.dialect.match_regex(a, b, true, &flags)
    };

    match &operator {
        Eq => Ok(cmp::eq(left_converted, right_converted)),
        Gt => Ok(cmp::gt(left_converted, right_converted)),
        Gte => Ok(cmp::gte(left_converted, right_converted)),
        Lt => Ok(cmp::lt(left_converted, right_converted)),
        Lte => Ok(cmp::lte(left_converted, right_converted)),
        Like => Ok(cmp::like(left_converted, right_converted)),
        RegexMatch => Ok(match_regex(left_converted, right_converted, scope)),
        // The match operator applies type-aware matching: case-insensitive "contains" when the
        // left-hand side is known to be text, falling back to exact equality otherwise.
        Match => {
            if classify_value_type(left, scope) == ValueType::Text {
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

/// Best-effort, shallow classification of an expression's value type, used to decide how the match
/// operator (`:`) behaves. Only direct column references (optionally reached through to-one join
/// chains) are classified; anything else returns [`ValueType::Unknown`], which the caller treats as
/// a fall back to exact equality.
fn classify_value_type(expr: &Expr, scope: &Scope) -> ValueType {
    let Expr::Path(parts) = expr else {
        return ValueType::Unknown;
    };
    // Mirror `convert_path`'s handling of the scope's path prefix so we classify the same column
    // that gets compiled.
    let prefixed_parts: Vec<PathPart> = scope
        .path_prefix
        .iter()
        .cloned()
        .chain(parts.iter().cloned())
        .collect();
    let Ok(clarified) = clarify_path(prefixed_parts, scope) else {
        return ValueType::Unknown;
    };
    let column_type = |table: &crate::schema::Table, column_name: &str| -> ValueType {
        scope
            .options
            .resolve_identifier(&table.column_lookup, column_name)
            .and_then(|id| table.columns.get(id))
            .map(|column| column.r#type.clone())
            .unwrap_or(ValueType::Unknown)
    };
    match (clarified.head, clarified.tail) {
        (None, Some(ClarifiedPathTail::Column(column_name))) => {
            column_type(scope.get_base_table(), &column_name)
        }
        (Some(chain_to_one), Some(ClarifiedPathTail::Column(column_name))) => {
            match scope.schema.tables.get(&chain_to_one.get_ending_table_id()) {
                Some(table) => column_type(table, &column_name),
                None => ValueType::Unknown,
            }
        }
        _ => ValueType::Unknown,
    }
}

/// Whether `expr` is statically known to be a date or datetime value: a date literal, the `@now`
/// constant, or a column path typed as a date/time. Used to recognize the date side of the
/// date/duration magic in [`convert_simple_comparison`].
fn is_date_like(expr: &Expr, scope: &Scope) -> bool {
    match expr {
        Expr::Date(_) => true,
        Expr::Variable(name) => name == VAR_NOW,
        Expr::Path(_) => matches!(
            classify_value_type(expr, scope),
            ValueType::Date | ValueType::Time
        ),
        _ => false,
    }
}

/// Maps a comparison operator to the SQL builder for comparing two numeric (epoch-second) operands,
/// used by the date/duration magic. Operators that don't define an ordering have no mapping, so the
/// magic doesn't apply to them.
fn numeric_comparison(operator: Operator) -> Option<fn(SqlExpr, SqlExpr) -> SqlExpr> {
    use Operator::*;
    match operator {
        Eq | Match => Some(cmp::eq),
        Gt => Some(cmp::gt),
        Gte => Some(cmp::gte),
        Lt => Some(cmp::lt),
        Lte => Some(cmp::lte),
        Like | RegexMatch => None,
    }
}

fn convert_range_comparison(
    expr: &Expr,
    range: &Range,
    scope: &mut Scope,
) -> Result<SqlExpr, String> {
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
) -> Result<SqlExpr, String> {
    let fallback = |scope: &mut Scope| {
        let op = match cmp {
            ComparisonVsZero::Eq => cmp::eq,
            ComparisonVsZero::Gt => cmp::gt,
        };
        Ok(op(convert_expr(expr.to_owned(), scope)?, value::zero()))
    };
    let Expr::Path(path_parts) = &expr else {
        return fallback(scope);
    };
    let Ok(clarified_path) = clarify_path(path_parts.to_owned(), scope) else {
        return fallback(scope);
    };
    let Some(ClarifiedPathTail::ChainToMany((chain, None))) = clarified_path.tail else {
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
) -> Result<Option<(Vec<PathPart>, CustomComparisonDef)>, String> {
    // Only a trailing plain column can name a custom comparison.
    let Some((PathPart::Column(name), head_parts)) = parts.split_last() else {
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
        .get_custom_comparison(&table.name, name)
        .map(|def| (head_parts.to_vec(), def.clone())))
}

/// Expands a custom comparison: the right-hand side value is bound to the definition's parameter and
/// the (possibly operator-switched) body is compiled in place of the comparison.
fn convert_custom_comparison(
    def: CustomComparisonDef,
    head_parts: Vec<PathPart>,
    call_operator: Operator,
    right: ComparisonSide,
    scope: &mut Scope,
) -> Result<SqlExpr, String> {
    let body = switched_body(&def, call_operator)?;
    match right {
        ComparisonSide::Expr(value) => {
            convert_custom_comparison_body(&def.param, value, body, &head_parts, scope)
        }
        // An expansion on the right binds the parameter to each entry in turn, combining the
        // results with the expansion's conjunction (e.g. `comment:..["a" "b"]`).
        ComparisonSide::Expansion(conditions) => {
            let exprs = conditions
                .entries
                .into_iter()
                .map(|value| {
                    convert_custom_comparison_body(
                        &def.param,
                        value,
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

fn convert_custom_comparison_body(
    param: &str,
    value: Expr,
    body: Expr,
    head_parts: &[PathPart],
    scope: &mut Scope,
) -> Result<SqlExpr, String> {
    let bindings = vec![(param.to_string(), value)];
    scope.with_variable_bindings(bindings, |scope| {
        convert_expr_with_path_prefix(body, head_parts.to_vec(), scope)
    })
}

/// Returns the custom comparison's body, with its `:` (match) operators rewritten to `call_operator`
/// if the comparison is being called with a different operator than it was defined with. Operator
/// switching is only permitted when the definition uses `:` and every comparison in the body also
/// uses `:`.
fn switched_body(def: &CustomComparisonDef, call_operator: Operator) -> Result<Expr, String> {
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
    match expr {
        Expr::Comparison(c) => {
            matches!(c.operator, Operator::Match)
                && side_all_match(&c.left)
                && side_all_match(&c.right)
        }
        Expr::Not(e) => expr_all_match(e),
        Expr::Product(a, b) | Expr::Quotient(a, b) | Expr::Sum(a, b) | Expr::Difference(a, b) => {
            expr_all_match(a) && expr_all_match(b)
        }
        Expr::ConditionSet(cs) => cs.entries.iter().all(expr_all_match),
        Expr::HasQuantity(h) => h.path_parts.iter().all(pathpart_all_match),
        Expr::Case(c) => {
            c.variants
                .iter()
                .all(|v| expr_all_match(&v.condition) && expr_all_match(&v.value))
                && expr_all_match(&c.fallback)
        }
        Expr::Call(c) => c.args.iter().all(expr_all_match),
        Expr::Path(parts) => parts.iter().all(pathpart_all_match),
        Expr::Window(w) => {
            w.args.iter().all(expr_all_match)
                && w.partition_by.iter().all(expr_all_match)
                && w.order_by.iter().all(|s| expr_all_match(&s.expr))
        }
        Expr::AnonymousFunctionCall(c) => {
            c.args.iter().all(expr_all_match)
                && c.body.assignments.iter().all(|a| expr_all_match(&a.expr))
                && expr_all_match(&c.body.expr)
        }
        Expr::Number(_)
        | Expr::Date(_)
        | Expr::Duration(_)
        | Expr::String(_)
        | Expr::Variable(_) => true,
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
    match part {
        PathPart::TableWithMany(t) => t.condition_set.entries.iter().all(expr_all_match),
        PathPart::Column(_) | PathPart::TableWithOne(_) => true,
    }
}

/// Rewrites every match (`:`) operator within `expr` to `new_operator`, recursing through the same
/// structure that [`expr_all_match`] inspects.
fn rewrite_match_operator(expr: &mut Expr, new_operator: Operator) {
    match expr {
        Expr::Comparison(c) => {
            if matches!(c.operator, Operator::Match) {
                c.operator = new_operator;
            }
            rewrite_side(&mut c.left, new_operator);
            rewrite_side(&mut c.right, new_operator);
        }
        Expr::Not(e) => rewrite_match_operator(e, new_operator),
        Expr::Product(a, b) | Expr::Quotient(a, b) | Expr::Sum(a, b) | Expr::Difference(a, b) => {
            rewrite_match_operator(a, new_operator);
            rewrite_match_operator(b, new_operator);
        }
        Expr::ConditionSet(cs) => cs
            .entries
            .iter_mut()
            .for_each(|e| rewrite_match_operator(e, new_operator)),
        Expr::HasQuantity(h) => h
            .path_parts
            .iter_mut()
            .for_each(|p| rewrite_pathpart(p, new_operator)),
        Expr::Case(c) => {
            for v in c.variants.iter_mut() {
                rewrite_match_operator(&mut v.condition, new_operator);
                rewrite_match_operator(&mut v.value, new_operator);
            }
            rewrite_match_operator(&mut c.fallback, new_operator);
        }
        Expr::Call(c) => c
            .args
            .iter_mut()
            .for_each(|e| rewrite_match_operator(e, new_operator)),
        Expr::Path(parts) => parts
            .iter_mut()
            .for_each(|p| rewrite_pathpart(p, new_operator)),
        Expr::Window(w) => {
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
        Expr::AnonymousFunctionCall(c) => {
            c.args
                .iter_mut()
                .for_each(|e| rewrite_match_operator(e, new_operator));
            c.body
                .assignments
                .iter_mut()
                .for_each(|a| rewrite_match_operator(&mut a.expr, new_operator));
            rewrite_match_operator(&mut c.body.expr, new_operator);
        }
        Expr::Number(_)
        | Expr::Date(_)
        | Expr::Duration(_)
        | Expr::String(_)
        | Expr::Variable(_) => {}
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
    if let PathPart::TableWithMany(t) = part {
        t.condition_set
            .entries
            .iter_mut()
            .for_each(|e| rewrite_match_operator(e, new_operator));
    }
}
