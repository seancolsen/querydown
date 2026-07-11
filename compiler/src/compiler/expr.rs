use querydown_parser::ast::*;

use crate::{
    errors::msg,
    schema::{links::Link, Table, ValueType},
    sql::expr::build::*,
    sql::tree::{CtePurpose, SqlExpr},
};

use super::{
    comparisons::convert_comparison,
    constants::{
        DEFAULT_TEXT_SEARCH_COMPARISON_NAME, VAR_FALSE, VAR_INFINITY, VAR_NOW, VAR_NULL, VAR_TRUE,
    },
    functions::{convert_anonymous_function_call, convert_call},
    paths::{clarify_path, ClarifiedPathTail},
    scope::Scope,
    temporal::{now_operand, reconcile, TemporalZone},
    typing::infer_type,
};

/// Convert a Querydown expression to an SQL expression
pub fn convert_expr(expr: Expr, scope: &mut Scope) -> Result<SqlExpr, String> {
    match expr {
        Expr::Number(n) => Ok(SqlExpr::atom(n)),
        Expr::Date(d) => Ok(SqlExpr::atom(scope.options.dialect.date(&d))),
        Expr::Duration(d) => Ok(SqlExpr::atom(scope.options.dialect.duration(&d))),
        Expr::String(s) => Ok(SqlExpr::atom(scope.options.dialect.quote_string(&s))),
        Expr::Variable(v) => convert_variable(&v, scope),
        Expr::Path(p) => convert_path(p, scope),
        Expr::ConditionSet(cs) => convert_condition_set(cs, scope),
        Expr::ScopedConditionSet(s) => convert_scoped_condition_set(s, scope),
        Expr::HasQuantity(h) => convert_has_quantity(h, scope),
        Expr::Case(c) => convert_case(c, scope),
        Expr::Call(c) => convert_call(c, scope),
        Expr::Product(a, b) => Ok(math::multiply(
            convert_expr(*a, scope)?,
            convert_expr(*b, scope)?,
        )),
        Expr::Quotient(a, b) => Ok(math::divide(
            convert_expr(*a, scope)?,
            convert_expr(*b, scope)?,
        )),
        Expr::Sum(a, b) => Ok(math::add(
            convert_expr(*a, scope)?,
            convert_expr(*b, scope)?,
        )),
        Expr::Difference(a, b) => {
            // Subtracting two temporal values whose zone-ness differs (e.g. `@now` minus a naive
            // `timestamp` column) needs reconciling for dialects that can't mix the two.
            let left_zone = TemporalZone::of(&infer_type(&a, scope));
            let right_zone = TemporalZone::of(&infer_type(&b, scope));
            let left = convert_expr(*a, scope)?;
            let right = convert_expr(*b, scope)?;
            let (left, right) = reconcile(
                (left, left_zone),
                (right, right_zone),
                scope.options.dialect.as_ref(),
            );
            Ok(math::subtract(left, right))
        }
        Expr::Comparison(c) => convert_comparison(*c, scope),
        Expr::Not(e) => Ok(cond::not(convert_expr(*e, scope)?)),
        Expr::Window(w) => super::window::convert_window(w, scope),
        Expr::AnonymousFunctionCall(c) => convert_anonymous_function_call(*c, scope),
        Expr::Subquery(q) => super::convert_subquery(*q, scope),
    }
}

fn convert_variable(variable: &str, scope: &mut Scope) -> Result<SqlExpr, String> {
    let sql = match variable {
        VAR_NOW => now_operand(scope.options.dialect.as_ref()).0,
        VAR_INFINITY => value::infinity(),
        VAR_TRUE => value::true_(),
        VAR_FALSE => value::false_(),
        VAR_NULL => value::null(),
        // A user-defined variable (constant or function parameter): inline its bound expression.
        name => {
            return match scope.get_variable(name) {
                Some(expr) => convert_expr(expr.clone(), scope),
                None => Err(msg::unknown_variable(name)),
            }
        }
    };
    Ok(SqlExpr::atom(sql.to_string()))
}

fn convert_path(parts: Vec<PathPart>, scope: &mut Scope) -> Result<SqlExpr, String> {
    let prefixed_parts: Vec<PathPart> = scope.path_prefix.iter().cloned().chain(parts).collect();
    // If the path names a computed column, substitute its definition, evaluated relative to the
    // table that hosts it (the head of the path).
    if let Some((head_parts, expr)) = resolve_computed_column(&prefixed_parts, scope)? {
        return convert_expr_with_path_prefix(expr, head_parts, scope);
    }
    let clarified_path = clarify_path(prefixed_parts, scope)?;
    match (clarified_path.head, clarified_path.tail) {
        (None, None) => Ok(SqlExpr::empty()),
        (None, Some(ClarifiedPathTail::Column(column_name))) => {
            let table_name = scope.get_base_table().name.clone();
            Ok(scope.table_column_expr(&table_name, &column_name))
        }
        (Some(chain_to_one), None) => {
            let (truncated_chain_to_one_opt, last_link) = chain_to_one.with_last_link_broken_off();
            let table_name = match truncated_chain_to_one_opt {
                Some(truncated_chain_to_one) => scope.join_chain_to_one(&truncated_chain_to_one),
                None => scope.get_base_table().name.clone(),
            };
            let column_reference = last_link.get_start();
            let column_name = scope.schema.get_referenced_column_name(&column_reference);
            Ok(scope.table_column_expr(&table_name, &column_name))
        }
        (Some(chain_to_one), Some(ClarifiedPathTail::Column(column_name))) => {
            let table_name = scope.join_chain_to_one(&chain_to_one);
            Ok(scope.table_column_expr(&table_name, &column_name))
        }
        (_, Some(ClarifiedPathTail::ChainToMany((_, Some(column_name), _)))) => Err(
            msg::path_to_many_with_column_name_and_no_agg_fn(&column_name),
        ),
        (head, Some(ClarifiedPathTail::ChainToMany((chain_to_many, None, _)))) => {
            scope.join_chain_to_many(&head, chain_to_many, None, CtePurpose::AggregateValue)
        }
    }
}

/// If `parts` names a computed column, returns the head of the path (the parts that lead to the
/// table hosting the computed column) together with the computed column's expression. A real column
/// of the same name always takes precedence, in which case this returns `None`.
pub(super) fn resolve_computed_column(
    parts: &[PathPart],
    scope: &Scope,
) -> Result<Option<(Vec<PathPart>, Expr)>, String> {
    // Only a trailing plain column can name a computed column.
    let Some((PathPart::Column(name), head_parts)) = parts.split_last() else {
        return Ok(None);
    };
    // Determine the table hosting the computed column. Only the base table (empty head) or a single
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
    // A real column of the same name takes precedence over a computed column.
    if scope
        .options
        .resolve_identifier(&table.column_lookup, name)
        .is_some()
    {
        return Ok(None);
    }
    Ok(scope
        .get_computed_column(&table.name, name)
        .map(|expr| (head_parts.to_vec(), expr.clone())))
}

/// Converts an expression with `path_prefix` temporarily set, restoring the previous prefix
/// afterward (which, unlike [`Scope::with_path_prefix`], supports nesting).
pub(super) fn convert_expr_with_path_prefix(
    expr: Expr,
    path_prefix: Vec<PathPart>,
    scope: &mut Scope,
) -> Result<SqlExpr, String> {
    let saved = std::mem::replace(&mut scope.path_prefix, path_prefix);
    let result = convert_expr(expr, scope);
    scope.path_prefix = saved;
    result
}

/// Compiles a scoped condition set (`issue{title:dashboard}`) by evaluating its condition set with
/// the scope's path prefix extended by the scoped path. Every entry — a comparison, a bare default
/// text search, a nested scope, and so on — is then compiled exactly as it would be at the top level
/// of the related table, because path resolution ([`convert_path`]) and default text search
/// ([`convert_default_text_search`]) both consult that prefix.
///
/// The prefix is *extended* rather than replaced so that nested scopes compose:
/// `issue{project{name:x}}` scopes `name:x` to `issue.project`.
fn convert_scoped_condition_set(
    scoped: ScopedConditionSet,
    scope: &mut Scope,
) -> Result<SqlExpr, String> {
    let extended_prefix: Vec<PathPart> = scope
        .path_prefix
        .iter()
        .cloned()
        .chain(scoped.path)
        .collect();
    convert_expr_with_path_prefix(
        Expr::ConditionSet(scoped.condition_set),
        extended_prefix,
        scope,
    )
}

pub fn convert_condition_set(
    condition_set: ConditionSet,
    scope: &mut Scope,
) -> Result<SqlExpr, String> {
    let conditions = condition_set
        .entries
        .into_iter()
        .map(|entry| convert_condition_set_entry(entry, scope))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(cmp::condition_set(conditions, &condition_set.conjunction))
}

/// Compiles a single boolean condition-set entry.
///
/// A bare string standing alone as a boolean condition is a default text search term. The parser
/// produces this for an unquoted bare word as well as a quoted string (see the `condition_entry`
/// parser); a backtick-quoted identifier remains an `Expr::Path` and so is treated as an ordinary
/// column reference here.
///
/// A `!`-prefixed search term (`!backend`) arrives as an `Expr::Not` wrapping the string. The `Not`
/// layers are peeled recursively so the search interpretation still reaches the inner term (and
/// repeated negation like `!!backend` works). Every other entry — including `Not` of a non-search
/// expression such as a negated column or condition set — falls through to `convert_expr` unchanged.
fn convert_condition_set_entry(entry: Expr, scope: &mut Scope) -> Result<SqlExpr, String> {
    match entry {
        Expr::String(term) => convert_default_text_search(term, scope),
        Expr::Not(inner) => Ok(cond::not(convert_condition_set_entry(*inner, scope)?)),
        entry => convert_expr(entry, scope),
    }
}

/// Compiles a default text search for `term` against the scope's current table — the base table, or,
/// when a [scoped condition set](convert_scoped_condition_set) is being compiled, the related table
/// the scope points at. If that table has a `__querydown_default_text_search` custom comparison
/// defined, that comparison configures the search. Otherwise the search is an `OR` across all of the
/// table's text-like columns, each matched against `term` with the type-aware match operator
/// (case-insensitive "contains").
///
/// Every column reference and custom-comparison lookup produced here flows back through the ordinary
/// comparison machinery, which applies the scope's `path_prefix`, so the search lands on the scoped
/// table's columns (e.g. `issue{dashboard}` searches `issue.title`, `issue.description`, …).
fn convert_default_text_search(term: String, scope: &mut Scope) -> Result<SqlExpr, String> {
    let table = get_prefix_table(scope)?;
    let table_name = table.name.clone();

    // A custom comparison with the reserved name lets the schema author configure the search.
    if scope
        .get_custom_comparison(&table_name, DEFAULT_TEXT_SEARCH_COMPARISON_NAME)
        .is_some()
    {
        let comparison = Comparison {
            left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column(
                DEFAULT_TEXT_SEARCH_COMPARISON_NAME.to_string(),
            )])),
            operator: Operator::Match,
            right: ComparisonSide::Expr(Expr::String(term)),
        };
        return convert_comparison(comparison, scope);
    }

    // Otherwise, search every text-like column of the table, in column order.
    let mut text_columns: Vec<(usize, String)> = table
        .columns
        .values()
        .filter(|column| column.r#type == ValueType::Text)
        .map(|column| (column.id, column.name.clone()))
        .collect();
    text_columns.sort_by_key(|(id, _)| *id);
    if text_columns.is_empty() {
        return Err(msg::no_default_text_search_columns(&table_name));
    }
    let entries = text_columns
        .into_iter()
        .map(|(_, column_name)| {
            Expr::Comparison(Box::new(Comparison {
                left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column(column_name)])),
                operator: Operator::Match,
                right: ComparisonSide::Expr(Expr::String(term.clone())),
            }))
        })
        .collect();
    convert_condition_set(ConditionSet::via_or(entries), scope)
}

/// The table that the scope's current `path_prefix` resolves to: the base table when there is no
/// prefix, or the single related record the prefix leads to. Used to scope a default text search to
/// the right table. Errors if the prefix does not lead to exactly one related record (e.g. it ends
/// at a column, or traverses a to-many relationship).
fn get_prefix_table<'a>(scope: &'a Scope) -> Result<&'a Table, String> {
    if scope.path_prefix.is_empty() {
        return Ok(scope.get_base_table());
    }
    let clarified = clarify_path(scope.path_prefix.clone(), scope)?;
    match (clarified.head, clarified.tail) {
        (Some(chain_to_one), None) => Ok(scope
            .schema
            .tables
            .get(&chain_to_one.get_ending_table_id())
            .unwrap()),
        _ => Err(msg::scope_is_not_a_single_related_record()),
    }
}

fn convert_case(case: Case, scope: &mut Scope) -> Result<SqlExpr, String> {
    let mut variants = Vec::with_capacity(case.variants.len());
    for variant in case.variants {
        let condition = convert_expr(variant.condition, scope)?;
        let value = convert_expr(variant.value, scope)?;
        variants.push((condition, value));
    }
    let fallback = convert_expr(*case.fallback, scope)?;
    Ok(cond::case(variants, fallback))
}

fn convert_has_quantity(has_quantity: HasQuantity, scope: &mut Scope) -> Result<SqlExpr, String> {
    let operator = match has_quantity.quantity {
        Quantity::AtLeastOne => Operator::Gt,
        Quantity::Zero => Operator::Eq,
    };
    let comparison = Comparison {
        left: ComparisonSide::Expr(Expr::Path(has_quantity.path_parts)),
        operator,
        right: ComparisonSide::Expr(Expr::zero()),
    };
    convert_comparison(comparison, scope)
}
