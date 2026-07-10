use std::collections::HashMap;

use itertools::Itertools;
use querydown_parser::ast::{
    AnonymousFunctionCall, Assignment, Call, Expr, FunctionBody, FunctionDef, FunctionDimension,
    SortExpr,
};

use crate::{
    compiler::{
        expr::convert_expr,
        paths::{
            clarify_path, render_order_by, AggWrapper, AggregateExprTemplate, ClarifiedPath,
            ClarifiedPathTail,
        },
        scope::Scope,
        temporal::{now_operand, reconcile, TemporalZone},
        typing::infer_type,
    },
    errors::msg::{self, unknown_aggregate_function, unknown_scalar_function},
    schema::ValueType,
    sql::expr::build::{self, agg::*, cond::*, date_time::*, math::*, strings::*},
    sql::tree::{CtePurpose, SqlExpr},
};

/// Describes the [`ValueType`] a function produces, in terms of the function's arguments. This lets
/// type inference (see [`crate::compiler::typing`]) propagate a value's type through a function call
/// without compiling or evaluating it — for example, recognizing that `created_at%max` is still a
/// datetime because `max` preserves its argument's type.
#[derive(Debug, Clone)]
pub enum TypeRule {
    /// The function always returns this type, regardless of its arguments (e.g. `length` → number).
    Fixed(ValueType),
    /// The function returns the same type as the argument at this index (e.g. `max`/`min`, which
    /// return one of their inputs unchanged).
    SameAsArg(usize),
    /// The function returns the common type of all its arguments — that shared type if they all
    /// agree, otherwise [`ValueType::Unknown`]. Used by variadic type-preserving functions like
    /// `if_null` (coalesce) and `keep_above`/`keep_below` (greatest/least).
    UnifyArgs,
    /// The function returns an array whose element type is that of the argument at this index (e.g.
    /// the `list` aggregate).
    ArrayOfArg(usize),
}

pub fn convert_call(call: Call, scope: &mut Scope) -> Result<SqlExpr, String> {
    match call.dimension {
        FunctionDimension::Scalar => convert_scalar_call(&call.name, call.args, scope),
        FunctionDimension::Aggregate => {
            convert_aggregate_call(&call.name, call.args, call.order_by, scope)
        }
    }
}

fn convert_scalar_call(name: &str, e: Vec<Expr>, s: &mut Scope) -> Result<SqlExpr, String> {
    // Built-in scalar functions take precedence; a user-defined function of the same name is only
    // consulted if no built-in matches.
    if let Some(func) = s.get_scalar_function(name) {
        return (func.call)(e, s);
    }
    if let Some(def) = s.get_user_function(name).cloned() {
        return convert_user_function_call(def, e, s);
    }
    Err(unknown_scalar_function(name))
}

/// Applies a user-defined scalar function by binding its parameters to the call's arguments and its
/// local assignments, then compiling its body expression. Because the bindings map names to AST
/// expressions (resolved lazily when referenced), the body is effectively inlined into the
/// surrounding SQL.
fn convert_user_function_call(
    def: FunctionDef,
    args: Vec<Expr>,
    scope: &mut Scope,
) -> Result<SqlExpr, String> {
    apply_function(&def.name, def.params, def.body, args, scope)
}

/// Applies an inline anonymous function (e.g. `value|(@d => @d:<0)`). This works exactly like
/// applying a user-defined function: the parameters are bound to the arguments and the body is
/// inlined. The only difference is that an anonymous function has no name.
pub fn convert_anonymous_function_call(
    call: AnonymousFunctionCall,
    scope: &mut Scope,
) -> Result<SqlExpr, String> {
    apply_function("(anonymous)", call.params, call.body, call.args, scope)
}

/// Shared logic for applying a function (named or anonymous): bind its parameters to the arguments
/// and its local assignments, then compile its body expression.
fn apply_function(
    name: &str,
    params: Vec<String>,
    body: FunctionBody,
    args: Vec<Expr>,
    scope: &mut Scope,
) -> Result<SqlExpr, String> {
    if args.len() != params.len() {
        return Err(msg::function_wrong_arg_count(
            name,
            params.len(),
            args.len(),
        ));
    }
    let bindings: Vec<(String, Expr)> = params
        .into_iter()
        .zip(args)
        .chain(
            body.assignments
                .into_iter()
                .map(|a: Assignment| (a.name, a.expr)),
        )
        .collect();
    scope.with_variable_bindings(bindings, |scope| convert_expr(body.expr, scope))
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
    (func.call)(e, order_by, s)
}

pub type ScalarFuncMap = HashMap<String, ScalarFunction>;
pub type ScalarFunc = fn(Vec<Expr>, &mut Scope) -> Result<SqlExpr, String>;

pub type AggregateFuncMap = HashMap<String, AggregateFunction>;
pub type AggregateFunc = fn(Vec<Expr>, Vec<SortExpr>, &mut Scope) -> Result<SqlExpr, String>;

/// A built-in scalar function: the closure that compiles it, paired with the rule describing the
/// type of value it returns (consulted by type inference).
pub struct ScalarFunction {
    pub call: ScalarFunc,
    pub return_type: TypeRule,
}

/// A built-in aggregate function: the closure that compiles it, paired with the rule describing the
/// type of value it returns (consulted by type inference).
pub struct AggregateFunction {
    pub call: AggregateFunc,
    pub return_type: TypeRule,
}

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

/// The zone classification of an AST argument, for temporal reconciliation (see
/// [`crate::compiler::temporal`]).
fn arg_zone(arg: &Expr, scope: &mut Scope) -> TemporalZone {
    TemporalZone::of(&infer_type(arg, scope))
}

/// `age(x)` = `@now - x`. `@now` is zoned, so when `x` is naive the subtraction is reconciled for
/// dialects that can't mix zoned and naive temporal values.
fn temporal_age(args: Vec<Expr>, scope: &mut Scope) -> Result<SqlExpr, String> {
    let arg = iter_one(args).ok_or_else(msg::expected_one_arg)?;
    let zone = arg_zone(&arg, scope);
    let a = convert_expr(arg, scope)?;
    let (now_sql, now_zone) = now_operand(scope.options.dialect.as_ref());
    let (now, a) = reconcile(
        (now_sql, now_zone),
        (a, zone),
        scope.options.dialect.as_ref(),
    );
    Ok(subtract(now, a))
}

/// `countdown(x)` = `x - @now`, the mirror of [`temporal_age`].
fn temporal_countdown(args: Vec<Expr>, scope: &mut Scope) -> Result<SqlExpr, String> {
    let arg = iter_one(args).ok_or_else(msg::expected_one_arg)?;
    let zone = arg_zone(&arg, scope);
    let a = convert_expr(arg, scope)?;
    let (now_sql, now_zone) = now_operand(scope.options.dialect.as_ref());
    let (a, now) = reconcile(
        (a, zone),
        (now_sql, now_zone),
        scope.options.dialect.as_ref(),
    );
    Ok(subtract(a, now))
}

/// `minus(a, b)` = `a - b`, reconciling the operands when they mix a zoned and a naive temporal
/// value.
fn temporal_minus(args: Vec<Expr>, scope: &mut Scope) -> Result<SqlExpr, String> {
    let (a, b) = iter_two(args).ok_or_else(msg::expected_two_args)?;
    let a_zone = arg_zone(&a, scope);
    let b_zone = arg_zone(&b, scope);
    let a_sql = convert_expr(a, scope)?;
    let b_sql = convert_expr(b, scope)?;
    let (a_sql, b_sql) = reconcile(
        (a_sql, a_zone),
        (b_sql, b_zone),
        scope.options.dialect.as_ref(),
    );
    Ok(subtract(a_sql, b_sql))
}

pub fn get_standard_scalar_functions() -> ScalarFuncMap {
    use TypeRule::*;
    use ValueType::*;
    // Each row pairs a function's compiler with the rule for the type it returns. The duration-
    // producing functions (`age`, `countdown`, `days`, …) report `Unknown` because there is no
    // dedicated duration value type. Addition and subtraction preserve their left operand's type so
    // that `date|plus(1w)` stays a date.
    #[rustfmt::skip]
    let templates: [(&str, ScalarFunc, TypeRule); 30] = [
        ("abs",         |e, s| args_1(e, s, abs),                    SameAsArg(0)),
        ("age",         |e, s| temporal_age(e, s),                  Fixed(Unknown)),
        ("and",         |e, s| args_v(e, s, build::cmp::and),        Fixed(Boolean)),
        ("ceil",        |e, s| args_1(e, s, ceil),                   Fixed(Number)),
        ("concat",      |e, s| args_v(e, s, concat),                 Fixed(Text)),
        ("countdown",   |e, s| temporal_countdown(e, s),            Fixed(Unknown)),
        ("days",        |e, s| args_1(e, s, days),                   Fixed(Unknown)),
        ("divide",      |e, s| args_2(e, s, divide),                 Fixed(Number)),
        ("floor",       |e, s| args_1(e, s, floor),                  Fixed(Number)),
        ("hours",       |e, s| args_1(e, s, hours),                  Fixed(Unknown)),
        ("if_null",     |e, s| args_v(e, s, coalesce),               UnifyArgs),
        ("keep_above",  |e, s| args_v(e, s, greatest),               UnifyArgs),
        ("keep_below",  |e, s| args_v(e, s, least),                  UnifyArgs),
        ("length",      |e, s| args_1(e, s, char_length),            Fixed(Number)),
        ("lowercase",   |e, s| args_1(e, s, lower),                  Fixed(Text)),
        ("max",         |e, s| args_v(e, s, greatest),               UnifyArgs),
        ("md5",         |e, s| args_1(e, s, md5),                    Fixed(Text)),
        ("min",         |e, s| args_v(e, s, least),                  UnifyArgs),
        ("minus",       |e, s| temporal_minus(e, s),                SameAsArg(0)),
        ("minutes",     |e, s| args_1(e, s, minutes),                Fixed(Unknown)),
        ("mod",         |e, s| args_2(e, s, modulo),                 Fixed(Number)),
        ("not",         |e, s| args_1(e, s, not),                    Fixed(Boolean)),
        ("or",          |e, s| args_v(e, s, build::cmp::or),         Fixed(Boolean)),
        ("plus",        |e, s| args_2(e, s, add),                    SameAsArg(0)),
        ("seconds",     |e, s| args_1(e, s, seconds),                Fixed(Unknown)),
        ("times",       |e, s| args_2(e, s, multiply),               Fixed(Number)),
        ("trim",        |e, s| args_1(e, s, trim),                   Fixed(Text)),
        ("uppercase",   |e, s| args_1(e, s, upper),                  Fixed(Text)),
        ("xor",         |e, s| args_v(e, s, build::cmp::xor),        Fixed(Boolean)),
        ("years",       |e, s| args_1(e, s, years),                  Fixed(Unknown)),
    ];
    templates
        .into_iter()
        .map(|(name, call, return_type)| (name.to_string(), ScalarFunction { call, return_type }))
        .collect()
}

/// Used for an aggregate function that takes one argument
fn agg_1(
    args: Vec<Expr>,
    order_by: Vec<SortExpr>,
    scope: &mut Scope,
    agg_wrapper: AggWrapper,
    coalesce_to_zero: bool,
) -> Result<SqlExpr, String> {
    let arg0 = iter_one(args).ok_or_else(msg::expected_one_arg)?;
    let Expr::Path(path_parts) = arg0 else {
        return Err(msg::aggregate_fn_applied_to_a_non_path());
    };
    let ClarifiedPath { head, tail } = clarify_path(path_parts, scope)?;
    match tail {
        // Aggregating data that joins many records (e.g. `#issues.created_at%max`). This is handled
        // by building a CTE that performs the aggregation grouped by the linking column.
        Some(ClarifiedPathTail::ChainToMany((chain_to_many, column_name_opt, anchor_table_id))) => {
            let Some(column_name) = column_name_opt else {
                return Err(msg::aggregate_fn_applied_to_a_path_without_a_column());
            };
            let aggregate_expr_template = AggregateExprTemplate::new(
                column_name,
                agg_wrapper,
                order_by,
                coalesce_to_zero,
                anchor_table_id,
            );
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
    use TypeRule::*;
    use ValueType::*;
    // Each wrapper ignores the trailing `&dyn Dialect` argument except `product`, whose SQL differs
    // between dialects. The trailing `TypeRule` describes the type each aggregate returns: `max`/`min`
    // preserve their argument's type, `list`/`list_all` wrap it in an array, and the rest are fixed.
    #[rustfmt::skip]
    let templates: [(&str, AggregateFunc, TypeRule); 11] = [
        ("all_true", |e, ob, s| agg_1(e, ob, s, |a, ob, _| bool_and(a, ob), false),           Fixed(Boolean)),
        ("any_true", |e, ob, s| agg_1(e, ob, s, |a, ob, _| bool_or(a, ob), false),            Fixed(Boolean)),
        ("avg",      |e, ob, s| agg_1(e, ob, s, |a, ob, _| avg(a, ob), false),                Fixed(Number)),
        ("count",    |e, ob, s| agg_1(e, ob, s, |a, ob, _| count(a, ob), true),               Fixed(Number)),
        ("distinct", |e, ob, s| agg_1(e, ob, s, |a, ob, _| count_distinct(a, ob), true),      Fixed(Number)),
        ("list",     |e, ob, s| agg_1(e, ob, s, |a, ob, _| array_agg_distinct(a, ob), false), ArrayOfArg(0)),
        ("list_all", |e, ob, s| agg_1(e, ob, s, |a, ob, _| array_agg(a, ob), false),          ArrayOfArg(0)),
        ("max",      |e, ob, s| agg_1(e, ob, s, |a, ob, _| max(a, ob), false),                SameAsArg(0)),
        ("min",      |e, ob, s| agg_1(e, ob, s, |a, ob, _| min(a, ob), false),                SameAsArg(0)),
        ("product",  |e, ob, s| agg_1(e, ob, s, |a, _ob, d| d.aggregate_product(a), false),   Fixed(Number)),
        ("sum",      |e, ob, s| agg_1(e, ob, s, |a, ob, _| sum(a, ob), false),                Fixed(Number)),
    ];
    templates
        .into_iter()
        .map(|(name, call, return_type)| {
            (name.to_string(), AggregateFunction { call, return_type })
        })
        .collect()
}
