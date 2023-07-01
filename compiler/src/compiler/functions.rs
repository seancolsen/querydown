use std::collections::HashMap;

use itertools::Itertools;
use querydown_parser::ast::{Call, Expr, FunctionDimension};

use crate::{
    compiler::{
        expr::convert_expr,
        paths::{clarify_path, AggregateExprTemplate, ClarifiedPathTail},
        scope::Scope,
    },
    errors::msg::{self, unknown_aggregate_function, unknown_scalar_function},
    sql::expr::build::{agg::*, cond::*, date_time::*, func::*, math::*, strings::*},
    sql::tree::{CtePurpose, SqlExpr},
};

pub fn convert_call(call: Call, scope: &mut Scope) -> Result<SqlExpr, String> {
    match call.dimension {
        FunctionDimension::Scalar => convert_scalar_call(&call.name, call.args, scope),
        FunctionDimension::Aggregate => convert_aggregate_call(&call.name, call.args, scope),
    }
}

fn convert_scalar_call(name: &str, e: Vec<Expr>, s: &mut Scope) -> Result<SqlExpr, String> {
    let func = s
        .get_scalar_function(name)
        .ok_or_else(|| unknown_scalar_function(name))?;
    func(e, s)
}

fn convert_aggregate_call(name: &str, e: Vec<Expr>, s: &mut Scope) -> Result<SqlExpr, String> {
    let func = s
        .get_aggregate_function(name)
        .ok_or_else(|| unknown_aggregate_function(name))?;
    func(e, s)
}

pub type FuncMap = HashMap<String, Func>;
pub type Func = fn(Vec<Expr>, &mut Scope) -> Result<SqlExpr, String>;

/// Get the first item out of an Iterator, ensuring it has no more
fn iter_one<T>(items: impl IntoIterator<Item = T>) -> Option<T> {
    items.into_iter().exactly_one().ok()
}

/// Get the first two items out of an Iterator, ensuring it has no more
fn iter_two<T>(items: impl IntoIterator<Item = T>) -> Option<(T, T)> {
    let mut iter = items.into_iter();
    let a = iter.next()?;
    let b = iter.next()?;
    if iter.next().is_some() {
        None
    } else {
        Some((a, b))
    }
}

/// Used for a scalar function that takes all arguments as a vector.
fn args_v(
    args: Vec<Expr>,
    scope: &mut Scope,
    f: fn(Vec<SqlExpr>) -> SqlExpr,
) -> Result<SqlExpr, String> {
    let mut sql_args = Vec::<SqlExpr>::new();
    for arg in args {
        sql_args.push(convert_expr(arg, scope)?);
    }
    Ok(f(sql_args))
}

/// Used for a scalar function that takes one argument
fn args_1(
    args: Vec<Expr>,
    scope: &mut Scope,
    f: fn(SqlExpr) -> SqlExpr,
) -> Result<SqlExpr, String> {
    let arg0 = iter_one(args).ok_or_else(msg::expected_one_arg)?;
    let a = convert_expr(arg0, scope)?;
    Ok(f(a))
}

/// Used for a scalar function that takes two arguments
fn args_2(
    args: Vec<Expr>,
    scope: &mut Scope,
    f: fn(SqlExpr, SqlExpr) -> SqlExpr,
) -> Result<SqlExpr, String> {
    let (a, b) = iter_two(args).ok_or_else(msg::expected_two_args)?;
    Ok(f(convert_expr(a, scope)?, convert_expr(b, scope)?))
}

pub fn get_standard_scalar_functions() -> FuncMap {
    #[rustfmt::skip]
    let templates: [(&str, Func); 24] = [
        ("abs",         |e, s| args_1(e, s, abs)),
        ("age",         |e, s| args_1(e, s, |a| subtract(now(), a))),
        ("ago",         |e, s| args_1(e, s, |a| subtract(now(), a))),
        ("away",        |e, s| args_1(e, s, |a| add(now(), a))),
        ("ceil",        |e, s| args_1(e, s, ceil)),
        ("days",        |e, s| args_1(e, s, days)),
        ("divide",      |e, s| args_2(e, s, divide)),
        ("else",        |e, s| args_1(e, s, coalesce)),
        ("floor",       |e, s| args_1(e, s, floor)),
        ("hours",       |e, s| args_1(e, s, hours)),
        ("keep_above",  |e, s| args_v(e, s, greatest)),
        ("keep_below",  |e, s| args_v(e, s, least)),
        ("length",      |e, s| args_1(e, s, char_length)),
        ("lowercase",   |e, s| args_1(e, s, lower)),
        ("max",         |e, s| args_v(e, s, greatest)),
        ("min",         |e, s| args_v(e, s, least)),
        ("minus",       |e, s| args_2(e, s, subtract)),
        ("minutes",     |e, s| args_1(e, s, minutes)),
        ("mod",         |e, s| args_2(e, s, modulo)),
        ("not",         |e, s| args_1(e, s, not)),
        ("plus",        |e, s| args_2(e, s, add)),
        ("seconds",     |e, s| args_1(e, s, seconds)),
        ("times",       |e, s| args_2(e, s, multiply)),
        ("uppercase",   |e, s| args_1(e, s, upper)),
    ];
    templates
        .into_iter()
        .map(|(s, f)| (s.to_string(), f))
        .collect()
}

/// Used for an aggregate function that takes one argument
fn agg_1(
    args: Vec<Expr>,
    scope: &mut Scope,
    agg_wrapper: fn(SqlExpr) -> SqlExpr,
) -> Result<SqlExpr, String> {
    let arg0 = iter_one(args).ok_or_else(msg::expected_one_arg)?;
    let Expr::Path(path_parts) = arg0 else {
        return Err(msg::aggregate_fn_applied_to_a_non_path());
    };
    let clarified_path = clarify_path(path_parts, scope)?;
    let Some(ClarifiedPathTail::ChainToMany((chain_to_many, column_name_opt))) = clarified_path.tail else {
        return Err(msg::aggregate_fn_applied_to_path_to_one());
    };
    let Some(column_name) = column_name_opt else {
        return Err(msg::aggregate_fn_applied_to_a_path_without_a_column());
    };
    let aggregate_expr_template = AggregateExprTemplate::new(column_name, agg_wrapper);
    scope.join_chain_to_many(
        &clarified_path.head,
        chain_to_many,
        Some(aggregate_expr_template),
        CtePurpose::AggregateValue,
    )
}

pub fn get_standard_aggregate_functions() -> FuncMap {
    #[rustfmt::skip]
    let templates: [(&str, Func); 9] = [
        ("all_true", |e, s| agg_1(e, s, bool_and)),
        ("any_true", |e, s| agg_1(e, s, bool_or)),
        ("avg",      |e, s| agg_1(e, s, avg)),
        ("count",    |e, s| agg_1(e, s, count)),
        ("distinct", |e, s| agg_1(e, s, count_distinct)),
        ("list",     |e, s| agg_1(e, s, string_agg)),
        ("max",      |e, s| agg_1(e, s, max)),
        ("min",      |e, s| agg_1(e, s, min)),
        ("sum",      |e, s| agg_1(e, s, sum)),
    ];
    templates
        .into_iter()
        .map(|(s, f)| (s.to_string(), f))
        .collect()
}
