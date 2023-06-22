use querydown_parser::ast::{
    Comparison, ConditionSet, Expr, HasQuantity, Operator, PathPart, Quantity,
};

use crate::{
    errors::msg,
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
    }
}

fn convert_variable(variable: &str, _: &Scope) -> Result<SqlExpr, String> {
    let sql = match variable {
        VAR_NOW => func::now(),
        VAR_INFINITY => value::infinity(),
        VAR_TRUE => value::true_(),
        VAR_FALSE => value::false_(),
        VAR_NULL => value::null(),
        // TODO handle user-defined variables from scope
        name => return Err(msg::unknown_variable(name)),
    };
    Ok(SqlExpr::atom(sql.to_string()))
}

fn convert_path(parts: Vec<PathPart>, scope: &mut Scope) -> Result<SqlExpr, String> {
    let clarified_path = clarify_path(parts, scope)?;
    match (clarified_path.head, clarified_path.tail) {
        (None, ClarifiedPathTail::Column(column_name)) => {
            let table_name = scope.get_base_table().name.clone();
            Ok(scope.table_column_expr(&table_name, &column_name))
        }
        (Some(chain_to_one), ClarifiedPathTail::Column(column_name)) => {
            let table_name = scope.join_chain_to_one(&chain_to_one);
            Ok(scope.table_column_expr(&table_name, &column_name))
        }
        (_, ClarifiedPathTail::ChainToMany((_, Some(column_name)))) => Err(
            msg::path_to_many_with_column_name_and_no_agg_fn(&column_name),
        ),
        (head, ClarifiedPathTail::ChainToMany((chain_to_many, None))) => {
            scope.join_chain_to_many(&head, chain_to_many, None, CtePurpose::AggregateValue)
        }
    }
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

fn convert_has_quantity(has_quantity: HasQuantity, scope: &mut Scope) -> Result<SqlExpr, String> {
    let operator = match has_quantity.quantity {
        Quantity::AtLeastOne => Operator::Gt,
        Quantity::Zero => Operator::Eq,
    };
    let comparison = Comparison {
        left: Expr::Path(has_quantity.path_parts),
        operator,
        right: Expr::zero(),
        is_expand_left: false,
        is_expand_right: false,
    };
    convert_comparison(comparison, scope)
}
