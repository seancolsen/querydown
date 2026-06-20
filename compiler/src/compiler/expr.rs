use querydown_parser::ast::*;

use crate::{
    errors::msg,
    schema::{links::Link, ValueType},
    sql::expr::build::*,
    sql::tree::{CtePurpose, SqlExpr},
};

use super::{
    comparisons::convert_comparison,
    constants::{
        DEFAULT_TEXT_SEARCH_COMPARISON_NAME, VAR_FALSE, VAR_INFINITY, VAR_NOW, VAR_NULL, VAR_TRUE,
    },
    functions::convert_call,
    paths::{clarify_path, ClarifiedPathTail},
    scope::Scope,
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
        Expr::Difference(a, b) => Ok(math::subtract(
            convert_expr(*a, scope)?,
            convert_expr(*b, scope)?,
        )),
        Expr::Comparison(c) => convert_comparison(*c, scope),
        Expr::Not(e) => Ok(cond::not(convert_expr(*e, scope)?)),
        Expr::Window(w) => super::window::convert_window(w, scope),
    }
}

fn convert_variable(variable: &str, scope: &mut Scope) -> Result<SqlExpr, String> {
    let sql = match variable {
        VAR_NOW => func::now(),
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
        (_, Some(ClarifiedPathTail::ChainToMany((_, Some(column_name))))) => Err(
            msg::path_to_many_with_column_name_and_no_agg_fn(&column_name),
        ),
        (head, Some(ClarifiedPathTail::ChainToMany((chain_to_many, None)))) => {
            scope.join_chain_to_many(&head, chain_to_many, None, CtePurpose::AggregateValue)
        }
    }
}

/// If `parts` names a computed column, returns the head of the path (the parts that lead to the
/// table hosting the computed column) together with the computed column's expression. A real column
/// of the same name always takes precedence, in which case this returns `None`.
fn resolve_computed_column(
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

pub fn convert_condition_set(
    condition_set: ConditionSet,
    scope: &mut Scope,
) -> Result<SqlExpr, String> {
    let conditions = condition_set
        .entries
        .into_iter()
        .map(|expr| convert_condition_entry(expr, scope))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(cmp::condition_set(conditions, &condition_set.conjunction))
}

/// Converts a single entry of a condition set. This is almost always [`convert_expr`], but it also
/// recognizes a bare **default text search** term: a standalone string (or bare alphanumeric word)
/// that carries no comparison operator. Such an entry is a low-friction way to search across many of
/// the base table's columns at once (see the "Default text search" section of the language guide).
fn convert_condition_entry(expr: Expr, scope: &mut Scope) -> Result<SqlExpr, String> {
    match default_text_search_term(&expr, scope) {
        Some(term) => convert_default_text_search(term, scope),
        None => convert_expr(expr, scope),
    }
}

/// If `expr`, appearing as a boolean condition, should be treated as a default text search term,
/// returns that term. A quoted string literal always qualifies. A bare word (parsed as a
/// single-column path) qualifies only when it begins with a letter, contains only alphanumeric
/// characters, and does not resolve to a real or computed column on the base table — in which case
/// it would otherwise be a column reference.
fn default_text_search_term(expr: &Expr, scope: &Scope) -> Option<String> {
    match expr {
        Expr::String(s) => Some(s.clone()),
        Expr::Path(parts) => {
            let [PathPart::Column(name)] = parts.as_slice() else {
                return None;
            };
            let is_bare_word = name.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
                && name.chars().all(|c| c.is_ascii_alphanumeric());
            if is_bare_word && !resolves_to_column(name, scope) {
                Some(name.clone())
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Whether `name` resolves to a real column or a computed column on the scope's base table.
fn resolves_to_column(name: &str, scope: &Scope) -> bool {
    let base_table = scope.get_base_table();
    scope
        .options
        .resolve_identifier(&base_table.column_lookup, name)
        .is_some()
        || scope.get_computed_column(&base_table.name, name).is_some()
}

/// Compiles a default text search for `term` against the scope's base table. If the base table has a
/// `__querydown_default_text_search` custom comparison defined, that comparison configures the
/// search. Otherwise the search is an `OR` across all of the base table's text-like columns, each
/// matched against `term` with the type-aware match operator (case-insensitive "contains").
fn convert_default_text_search(term: String, scope: &mut Scope) -> Result<SqlExpr, String> {
    let base_table_name = scope.get_base_table().name.clone();

    // A custom comparison with the reserved name lets the schema author configure the search.
    if scope
        .get_custom_comparison(&base_table_name, DEFAULT_TEXT_SEARCH_COMPARISON_NAME)
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

    // Otherwise, search every text-like column of the base table, in column order.
    let base_table = scope.get_base_table();
    let mut text_columns: Vec<(usize, String)> = base_table
        .columns
        .values()
        .filter(|column| column.r#type == ValueType::Text)
        .map(|column| (column.id, column.name.clone()))
        .collect();
    text_columns.sort_by_key(|(id, _)| *id);
    if text_columns.is_empty() {
        return Err(msg::no_default_text_search_columns(&base_table_name));
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
