use querydown_parser::ast::*;

use crate::{
    errors::msg,
    schema::links::Link,
    sql::expr::build::*,
    sql::tree::{CtePurpose, SqlExpr},
};

use super::{
    comparisons::convert_comparison,
    constants::{VAR_FALSE, VAR_INFINITY, VAR_NOW, VAR_NULL, VAR_TRUE},
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
        .map(|expr| convert_expr(expr, scope))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(cmp::condition_set(conditions, &condition_set.conjunction))
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
