use std::collections::HashMap;

use itertools::Itertools;
use querydown_parser::ast::{Call, Expr, FunctionDimension, SortExpr};

use crate::{
    compiler::{
        expr::convert_expr,
        paths::{
            clarify_path, render_order_by, AggWrapper, AggregateExprTemplate, ClarifiedPath,
            ClarifiedPathTail,
        },
        scope::Scope,
    },
    errors::msg::{self, unknown_aggregate_function, unknown_scalar_function},
    sql::expr::build::{self, agg::*, cond::*, date_time::*, func::*, math::*, strings::*},
    sql::tree::{CtePurpose, SqlExpr},
};

pub fn convert_call(call: Call, scope: &mut Scope) -> Result<SqlExpr, String> {
    match call.dimension {
        FunctionDimension::Scalar => convert_scalar_call(&call.name, call.args, scope),
        FunctionDimension::Aggregate => {
            convert_aggregate_call(&call.name, call.args, call.order_by, scope)
        }
    }
}

fn convert_scalar_call(name: &str, e: Vec<Expr>, s: &mut Scope) -> Result<SqlExpr, String> {
    let func = s
        .get_scalar_function(name)
        .ok_or_else(|| unknown_scalar_function(name))?;
    func(e, s)
}

fn convert_aggregate_call(
    name: &str,
    e: Vec<Expr>,
    order_by: Vec<SortExpr>,
    s: &mut Scope,
) -> Result<SqlExpr, String> {
    // A standalone aggregate with no argument, e.g. `%count`, which is shorthand for `count(*)`.
    // Only `count` is meaningful without an argument.
    if e.is_empty() {
        if name == "count" {
            return Ok(build::agg::count_star());
        }
        return Err(msg::aggregate_fn_without_argument(name));
    }
    let func = s
        .get_aggregate_function(name)
        .ok_or_else(|| unknown_aggregate_function(name))?;
    func(e, order_by, s)
}

pub type ScalarFuncMap = HashMap<String, ScalarFunc>;
pub type ScalarFunc = fn(Vec<Expr>, &mut Scope) -> Result<SqlExpr, String>;

pub type AggregateFuncMap = HashMap<String, AggregateFunc>;
pub type AggregateFunc = fn(Vec<Expr>, Vec<SortExpr>, &mut Scope) -> Result<SqlExpr, String>;

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

pub fn get_standard_scalar_functions() -> ScalarFuncMap {
    #[rustfmt::skip]
    let templates: [(&str, ScalarFunc); 30] = [
        ("abs",         |e, s| args_1(e, s, abs)),
        ("age",         |e, s| args_1(e, s, |a| subtract(now(), a))),
        ("ago",         |e, s| args_1(e, s, |a| subtract(now(), a))),
        ("and",         |e, s| args_v(e, s, build::cmp::and)),
        ("away",        |e, s| args_1(e, s, |a| subtract(a, now()))),
        ("ceil",        |e, s| args_1(e, s, ceil)),
        ("concat",      |e, s| args_v(e, s, concat)),
        ("days",        |e, s| args_1(e, s, days)),
        ("divide",      |e, s| args_2(e, s, divide)),
        ("floor",       |e, s| args_1(e, s, floor)),
        ("hours",       |e, s| args_1(e, s, hours)),
        ("if_null",     |e, s| args_v(e, s, coalesce)),
        ("keep_above",  |e, s| args_v(e, s, greatest)),
        ("keep_below",  |e, s| args_v(e, s, least)),
        ("length",      |e, s| args_1(e, s, char_length)),
        ("lowercase",   |e, s| args_1(e, s, lower)),
        ("max",         |e, s| args_v(e, s, greatest)),
        ("md5",         |e, s| args_1(e, s, md5)),
        ("min",         |e, s| args_v(e, s, least)),
        ("minus",       |e, s| args_2(e, s, subtract)),
        ("minutes",     |e, s| args_1(e, s, minutes)),
        ("mod",         |e, s| args_2(e, s, modulo)),
        ("not",         |e, s| args_1(e, s, not)),
        ("or",          |e, s| args_v(e, s, build::cmp::or)),
        ("plus",        |e, s| args_2(e, s, add)),
        ("seconds",     |e, s| args_1(e, s, seconds)),
        ("times",       |e, s| args_2(e, s, multiply)),
        ("trim",        |e, s| args_1(e, s, trim)),
        ("uppercase",   |e, s| args_1(e, s, upper)),
        ("xor",         |e, s| args_v(e, s, build::cmp::xor)),
    ];
    templates
        .into_iter()
        .map(|(s, f)| (s.to_string(), f))
        .collect()
}

/// Used for an aggregate function that takes one argument
fn agg_1(
    args: Vec<Expr>,
    order_by: Vec<SortExpr>,
    scope: &mut Scope,
    agg_wrapper: AggWrapper,
) -> Result<SqlExpr, String> {
    let arg0 = iter_one(args).ok_or_else(msg::expected_one_arg)?;
    let Expr::Path(path_parts) = arg0 else {
        return Err(msg::aggregate_fn_applied_to_a_non_path());
    };
    let ClarifiedPath { head, tail } = clarify_path(path_parts, scope)?;
    match tail {
        // Aggregating data that joins many records (e.g. `#issues.created_at%max`). This is handled
        // by building a CTE that performs the aggregation grouped by the linking column.
        Some(ClarifiedPathTail::ChainToMany((chain_to_many, column_name_opt))) => {
            let Some(column_name) = column_name_opt else {
                return Err(msg::aggregate_fn_applied_to_a_path_without_a_column());
            };
            let aggregate_expr_template =
                AggregateExprTemplate::new(column_name, agg_wrapper, order_by);
            scope.join_chain_to_many(
                &head,
                chain_to_many,
                Some(aggregate_expr_template),
                CtePurpose::AggregateValue,
            )
        }
        // Aggregating a column on the base table or a to-one related table (e.g. `created_at%max`).
        // This produces a direct SQL aggregate expression used in conjunction with `GROUP BY`.
        Some(ClarifiedPathTail::Column(column_name)) => {
            let table_name = match head {
                Some(chain_to_one) => scope.join_chain_to_one(&chain_to_one),
                None => scope.get_base_table().name.clone(),
            };
            let reference = scope.table_column_expr(&table_name, &column_name);
            let order_by_str = render_order_by(order_by, scope)?;
            Ok(agg_wrapper(
                reference,
                order_by_str,
                scope.options.dialect.as_ref(),
            ))
        }
        None => Err(msg::aggregate_fn_applied_to_a_path_without_a_column()),
    }
}

pub fn get_standard_aggregate_functions() -> AggregateFuncMap {
    // Each wrapper ignores the trailing `&dyn Dialect` argument except `product`, whose SQL differs
    // between dialects.
    #[rustfmt::skip]
    let templates: [(&str, AggregateFunc); 10] = [
        ("all_true", |e, ob, s| agg_1(e, ob, s, |a, ob, _| bool_and(a, ob))),
        ("any_true", |e, ob, s| agg_1(e, ob, s, |a, ob, _| bool_or(a, ob))),
        ("avg",      |e, ob, s| agg_1(e, ob, s, |a, ob, _| avg(a, ob))),
        ("count",    |e, ob, s| agg_1(e, ob, s, |a, ob, _| count(a, ob))),
        ("distinct", |e, ob, s| agg_1(e, ob, s, |a, ob, _| count_distinct(a, ob))),
        ("list",     |e, ob, s| agg_1(e, ob, s, |a, ob, _| array_agg(a, ob))),
        ("max",      |e, ob, s| agg_1(e, ob, s, |a, ob, _| max(a, ob))),
        ("min",      |e, ob, s| agg_1(e, ob, s, |a, ob, _| min(a, ob))),
        ("product",  |e, ob, s| agg_1(e, ob, s, |a, _ob, d| d.aggregate_product(a))),
        ("sum",      |e, ob, s| agg_1(e, ob, s, |a, ob, _| sum(a, ob))),
    ];
    templates
        .into_iter()
        .map(|(s, f)| (s.to_string(), f))
        .collect()
}
