use std::collections::HashMap;

use itertools::Itertools;
use querydown_parser::ast::{
    AggregateSortExpr, AggregateSortKey, AnonymousFunctionCall, Assignment, Call, CallSyntax, Expr,
    ExprKind, FunctionArg, FunctionBody, FunctionDef, FunctionDimension, Loc, PathPart,
    PathPartKind, RegexFlags,
};

use crate::{
    compiler::{
        aggregate_sorting::{
            compile_aggregate, resolve_aggregate_sorting, AggregateKind, CompiledAggregate,
            ListMode, ResolvedSort, SortKey,
        },
        expr::{
            convert_definition, convert_definition_with_env, convert_expr, resolve_computed_column,
        },
        paths::{clarify_value_path, AggregateExprTemplate, ClarifiedPath, ClarifiedPathTail},
        scope::{AggregateBan, Env, Expansion, Scope, WindowBan},
        temporal::{now_operand, reconcile, TemporalZone},
        typing::infer_type,
    },
    errors::msg::{self, unknown_aggregate_function, unknown_scalar_function},
    schema::ValueType,
    sql::expr::build::{self, agg::*, cond::*, date_time::*, func, math::*, strings::*},
    sql::tree::{CtePurpose, SqlExpr},
    sql::{Dialect, ElementNames, MaybeLiteral, ReduceNames, Separator},
    Error,
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
    /// The function returns an array of the type the inner rule gives (e.g. the `list` aggregate,
    /// an array of its argument's type, or `array`, of its arguments' common type).
    ArrayOf(Box<TypeRule>),
    /// The function returns an element of the array whose type the inner rule gives (e.g. `at`, an
    /// element of its first argument), or [`ValueType::Unknown`] when that isn't an array.
    ElementOf(Box<TypeRule>),
    /// The function adds its two arguments, so it returns what `+` does for them (see
    /// [`crate::compiler::typing::sum_type`]).
    Sum,
    /// The function subtracts its second argument from its first, so it returns what `-` does for
    /// them (see [`crate::compiler::typing::difference_type`]).
    Difference,
    /// The function applies the function given as the argument at this index to the elements of
    /// its first argument, an array, and returns the type of what that gives (e.g. `map`, whose
    /// result is an array of these).
    FunctionArgResult(usize),
}

impl TypeRule {
    /// An array of what `rule` gives. See [`TypeRule::ArrayOf`].
    pub fn array_of(rule: TypeRule) -> TypeRule {
        TypeRule::ArrayOf(Box::new(rule))
    }

    /// An element of the array that `rule` gives. See [`TypeRule::ElementOf`].
    pub fn element_of(rule: TypeRule) -> TypeRule {
        TypeRule::ElementOf(Box::new(rule))
    }
}

pub fn convert_call(call: Call, scope: &mut Scope) -> Result<SqlExpr, Error> {
    match call.dimension {
        FunctionDimension::Scalar => {
            convert_scalar_call(&call.name, call.name_loc, call.args, scope)
        }
        FunctionDimension::Aggregate => {
            convert_aggregate_call(&call.name, call.name_loc, call.args, call.order_by, scope)
        }
    }
}

/// Compiles a scalar function call. An error about the call itself (e.g. an unknown function, or
/// the wrong number of arguments) is located at the function's name, `name_loc`.
fn convert_scalar_call(
    name: &str,
    name_loc: Loc,
    e: Vec<Expr>,
    s: &mut Scope,
) -> Result<SqlExpr, Error> {
    // Built-in scalar functions take precedence; a user-defined function of the same name is only
    // consulted if no built-in matches.
    let result = if let Some(func) = s.get_scalar_function(name) {
        if !TAKE_A_FUNCTION.contains(&name) {
            reject_function_args(&e)?;
        }
        (func.call)(e, s)
    } else if let Some(def) = s.get_user_function(name).cloned() {
        reject_function_args(&e)?;
        convert_user_function_call(def, e, s)
    } else {
        Err(unknown_scalar_function(name))
    };
    result.map_err(|err| err.or_at(name_loc))
}

/// Applies a user-defined scalar function by binding its parameters to the call's arguments and its
/// local assignments, then compiling its body expression. Because the bindings map names to AST
/// expressions (resolved lazily when referenced), the body is effectively inlined into the
/// surrounding SQL.
fn convert_user_function_call(
    def: FunctionDef,
    args: Vec<Expr>,
    scope: &mut Scope,
) -> Result<SqlExpr, Error> {
    // The arguments belong to the call, so they are bound before the function's expansion is
    // entered: an argument that itself calls the function (`id|inc|inc`) isn't recursion. The body
    // of a named function sees the constants, but none of the caller's variables.
    let env = bind_params(&def.name, def.params, args, scope.globals(), scope)?;
    let _guard = scope.enter_expansion(Expansion::Function(def.name.clone()))?;
    let env = bind_assignments(def.body.assignments, env, scope);
    scope.with_env(env, |scope| convert_expr(def.body.expr, scope))
}

/// Applies an inline anonymous function (e.g. `value|(@d => @d:<0)`). This works exactly like
/// applying a user-defined function: the parameters are bound to the arguments and the body is
/// inlined. The differences are that an anonymous function has no name, and that — being written
/// within an expression — its body can see the variables visible around it.
pub fn convert_anonymous_function_call(
    call: AnonymousFunctionCall,
    scope: &mut Scope,
) -> Result<SqlExpr, Error> {
    reject_function_args(&call.args)?;
    let env = bind_params("(anonymous)", call.params, call.args, scope.env(), scope)?;
    let env = bind_assignments(call.body.assignments, env, scope);
    scope.with_env(env, |scope| convert_expr(call.body.expr, scope))
}

/// Extends `body_env` (the variables that a function's body can see from where it was written)
/// with the function's parameters, bound to the arguments of a call being made within `scope`.
///
/// An argument means what it meant at the call: it sees the caller's variables rather than the
/// function's, and a column it refers to is one of the table that the call was made on, even where
/// the body uses the parameter within a filter on related records.
pub(super) fn bind_params(
    name: &str,
    params: Vec<String>,
    args: Vec<Expr>,
    body_env: Env,
    scope: &Scope,
) -> Result<Env, Error> {
    if args.len() != params.len() {
        return Err(msg::function_wrong_arg_count(
            name,
            params.len(),
            args.len(),
        ));
    }
    let mut env = body_env;
    for (param, arg) in params.into_iter().zip(args) {
        env = scope.bind(&env, param, arg, scope.env(), scope.context());
    }
    Ok(env)
}

/// Extends `env` (which holds a function's parameters) with the function body's local assignments.
/// Each sees the parameters and the assignments before it.
pub(super) fn bind_assignments(assignments: Vec<Assignment>, env: Env, scope: &Scope) -> Env {
    let mut env = env;
    for Assignment { name, expr } in assignments {
        env = scope.bind(&env, name, expr, env.clone(), scope.context());
    }
    env
}

/// Compiles an aggregate function call. An unknown function, or one that needs an argument it
/// lacks, is reported at the function's name, `name_loc`.
fn convert_aggregate_call(
    name: &str,
    name_loc: Loc,
    e: Vec<Expr>,
    order_by: Vec<AggregateSortExpr>,
    s: &mut Scope,
) -> Result<SqlExpr, Error> {
    // A standalone aggregate with no argument, e.g. `%count`, which is shorthand for `count(*)`.
    // Only `count` is meaningful without an argument.
    if e.is_empty() {
        if let Some(value_sort) = order_by.iter().find(|o| o.key == AggregateSortKey::Value) {
            return Err(msg::value_sort_without_value(name).at(value_sort.loc));
        }
        if name == "count" {
            check_aggregate_ban(s).map_err(|err| err.at(name_loc))?;
            return Ok(build::agg::count_star());
        }
        return Err(msg::aggregate_fn_without_argument(name).at(name_loc));
    }
    let func = s
        .get_aggregate_function(name)
        .ok_or_else(|| unknown_aggregate_function(name).at(name_loc))?;
    (func.call)(e, order_by, s)
}

/// Fails if an aggregate of the rows being queried can't be used where `scope` is compiling,
/// because SQL would reject it there.
fn check_aggregate_ban(scope: &Scope) -> Result<(), Error> {
    match scope.aggregate_ban() {
        None => Ok(()),
        Some(AggregateBan::Condition) => Err(msg::aggregate_in_condition()),
        Some(AggregateBan::RelatedRecordsFilter) => Err(msg::aggregate_in_related_records_filter()),
        Some(AggregateBan::AggregatedValue) => Err(msg::aggregate_in_aggregated_value()),
        Some(AggregateBan::FunctionArg) => Err(msg::aggregate_in_function_arg()),
    }
}

pub type ScalarFuncMap = HashMap<String, ScalarFunction>;
pub type ScalarFunc = fn(Vec<Expr>, &mut Scope) -> Result<SqlExpr, Error>;

pub type AggregateFuncMap = HashMap<String, AggregateFunction>;
pub type AggregateFunc =
    fn(Vec<Expr>, Vec<AggregateSortExpr>, &mut Scope) -> Result<SqlExpr, Error>;

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
) -> Result<SqlExpr, Error> {
    let mut sql_args = Vec::<SqlExpr>::new();
    for arg in args {
        sql_args.push(convert_expr(arg, scope)?);
    }
    Ok(f(sql_args))
}

/// Used for a scalar function that takes one argument
fn args_1(args: Vec<Expr>, scope: &mut Scope, f: fn(SqlExpr) -> SqlExpr) -> Result<SqlExpr, Error> {
    let arg0 = iter_one(args).ok_or_else(msg::expected_one_arg)?;
    let a = convert_expr(arg0, scope)?;
    Ok(f(a))
}

/// Used for a scalar function that takes two arguments
fn args_2(
    args: Vec<Expr>,
    scope: &mut Scope,
    f: fn(SqlExpr, SqlExpr) -> SqlExpr,
) -> Result<SqlExpr, Error> {
    let (a, b) = iter_two(args).ok_or_else(msg::expected_two_args)?;
    Ok(f(convert_expr(a, scope)?, convert_expr(b, scope)?))
}

/// Used for a scalar function that takes one argument, and whose SQL differs between dialects.
fn dialect_1(
    args: Vec<Expr>,
    scope: &mut Scope,
    f: fn(&dyn Dialect, SqlExpr) -> SqlExpr,
) -> Result<SqlExpr, Error> {
    let arg0 = iter_one(args).ok_or_else(msg::expected_one_arg)?;
    let a = convert_expr(arg0, scope)?;
    Ok(f(scope.options.dialect.as_ref(), a))
}

/// Splits a call's arguments (the piped value first) into the `R` that the function requires and
/// the `O` optional ones that may follow them, failing if there are too few or too many:
///
/// ```ignore
/// let ([x, pattern], [group, flags]) = optional_args(args)?;
/// ```
pub(super) fn optional_args<const R: usize, const O: usize>(
    args: Vec<Expr>,
) -> Result<([Expr; R], [Option<Expr>; O]), Error> {
    let count = args.len();
    if !(R..=R + O).contains(&count) {
        return Err(msg::expected_args_between(R, R + O, count));
    }
    let mut args = args.into_iter();
    let required = std::array::from_fn(|_| args.next().expect("the arguments were counted"));
    let optional = std::array::from_fn(|_| args.next());
    Ok((required, optional))
}

/// The text of `arg` when it's written as a literal string, either directly or through constants
/// or parameters bound to one.
pub(super) fn string_literal(arg: &Expr, scope: &Scope) -> Option<String> {
    match scope.see_through_variables(arg).kind {
        ExprKind::String(s) => Some(s),
        _ => None,
    }
}

/// The value of `arg` when it's written as a literal whole number (perhaps negative), either
/// directly or through constants or parameters bound to one.
pub(super) fn integer_literal(arg: &Expr, scope: &Scope) -> Option<i64> {
    match scope.see_through_variables(arg).kind {
        ExprKind::Number(n) => n.parse().ok(),
        _ => None,
    }
}

/// Compiles `arg`, unless it's a literal that `read` recognizes, which is kept apart for the
/// dialect to write as suits it.
pub(super) fn maybe_literal<T>(
    arg: Expr,
    scope: &mut Scope,
    read: fn(&Expr, &Scope) -> Option<T>,
) -> Result<MaybeLiteral<T>, Error> {
    match read(&arg, scope) {
        Some(value) => Ok(MaybeLiteral::Literal(value)),
        None => Ok(MaybeLiteral::Expr(convert_expr(arg, scope)?)),
    }
}

/// Reads an argument that must be written as a literal string (directly, or through constants or
/// parameters), such as the `flags` of `regex_replace`. Otherwise, reports an error located at the
/// argument, which names it `param` and gives `example` as a literal that would do.
pub(super) fn literal_string_arg(
    arg: &Expr,
    param: &str,
    example: &str,
    scope: &Scope,
) -> Result<String, Error> {
    string_literal(arg, scope)
        .ok_or_else(|| msg::argument_not_literal(param, "string", example).at(arg.loc))
}

/// Reads an argument that must be written as a literal whole number of 0 or more (directly, or
/// through constants or parameters), such as the `group` of `regex_extract`. Otherwise, reports an
/// error located at the argument, which names it `param`.
pub(super) fn literal_count_arg(arg: &Expr, param: &str, scope: &Scope) -> Result<u32, Error> {
    if !matches!(scope.see_through_variables(arg).kind, ExprKind::Number(_)) {
        return Err(msg::argument_not_literal(param, "number", "1").at(arg.loc));
    }
    integer_literal(arg, scope)
        .and_then(|n| u32::try_from(n).ok())
        .ok_or_else(|| msg::argument_not_whole_number(param).at(arg.loc))
}

/// Compiles an argument that the database needs as an integer, such as the `digits` of `round`: a
/// literal whole number is kept apart, and anything else is cast to an integer.
fn integer_arg(arg: Expr, scope: &mut Scope) -> Result<MaybeLiteral<i64>, Error> {
    Ok(match maybe_literal(arg, scope, integer_literal)? {
        MaybeLiteral::Expr(e) => MaybeLiteral::Expr(func::cast(e, "INTEGER")),
        literal => literal,
    })
}

/// Reads the optional `flags` argument of a regex function: a literal string of the letters that
/// a flagged `:~` takes, with the same meanings (see [`RegexFlags`]).
fn regex_flags_arg(arg: Option<Expr>, scope: &Scope) -> Result<RegexFlags, Error> {
    let mut flags = RegexFlags::default();
    let Some(arg) = arg else {
        return Ok(flags);
    };
    for letter in literal_string_arg(&arg, "flags", "\"cm\"", scope)?.chars() {
        let flag = match letter {
            'c' => &mut flags.case_sensitive,
            'm' => &mut flags.multiline,
            's' => &mut flags.dot_all,
            _ => return Err(msg::unknown_regex_flag(letter).at(arg.loc)),
        };
        if *flag {
            return Err(msg::repeated_regex_flag(letter).at(arg.loc));
        }
        *flag = true;
    }
    Ok(flags)
}

/// `regex_replace(x pattern replacement flags?)`: every match of `pattern` replaced.
fn regex_replace(args: Vec<Expr>, scope: &mut Scope) -> Result<SqlExpr, Error> {
    let ([a, pattern, replacement], [flags]) = optional_args(args)?;
    let flags = regex_flags_arg(flags, scope)?;
    let a = convert_expr(a, scope)?;
    let pattern = maybe_literal(pattern, scope, string_literal)?;
    let replacement = convert_expr(replacement, scope)?;
    Ok(scope
        .options
        .dialect
        .regex_replace(a, pattern, replacement, &flags))
}

/// `regex_extract(x pattern group? flags?)`: the first match of `pattern`, or the text of one of
/// its capture groups.
fn regex_extract(args: Vec<Expr>, scope: &mut Scope) -> Result<SqlExpr, Error> {
    let ([a, pattern], [group, flags]) = optional_args(args)?;
    let group = match group {
        Some(group) => literal_count_arg(&group, "group", scope)?,
        None => 0,
    };
    let flags = regex_flags_arg(flags, scope)?;
    let a = convert_expr(a, scope)?;
    let pattern = maybe_literal(pattern, scope, string_literal)?;
    Ok(scope
        .options
        .dialect
        .regex_extract(a, pattern, group, &flags))
}

/// `round(x digits?)`.
fn round(args: Vec<Expr>, scope: &mut Scope) -> Result<SqlExpr, Error> {
    let ([a], [digits]) = optional_args(args)?;
    let a = convert_expr(a, scope)?;
    let digits = digits.map(|d| integer_arg(d, scope)).transpose()?;
    Ok(scope.options.dialect.round(a, digits))
}

/// `array(value…)`: an array of at least one value.
fn array(args: Vec<Expr>, scope: &mut Scope) -> Result<SqlExpr, Error> {
    if args.is_empty() {
        return Err(msg::expected_at_least_one_arg());
    }
    let values = args
        .into_iter()
        .map(|arg| convert_expr(arg, scope))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(scope.options.dialect.array(values))
}

/// `at(x index)`: an array's element at a position.
fn at(args: Vec<Expr>, scope: &mut Scope) -> Result<SqlExpr, Error> {
    let (a, index) = iter_two(args).ok_or_else(msg::expected_two_args)?;
    let a = convert_expr(a, scope)?;
    let index = maybe_literal(index, scope, integer_literal)?;
    Ok(scope.options.dialect.array_at(a, index))
}

/// `split(x separator)` and `join(x separator)`, whose SQL suits a literal separator.
fn with_separator(
    args: Vec<Expr>,
    scope: &mut Scope,
    f: fn(&dyn Dialect, SqlExpr, Separator) -> SqlExpr,
) -> Result<SqlExpr, Error> {
    let (a, separator) = iter_two(args).ok_or_else(msg::expected_two_args)?;
    let a = convert_expr(a, scope)?;
    let separator = maybe_literal(separator, scope, string_literal)?;
    Ok(f(scope.options.dialect.as_ref(), a, separator))
}

/// `length(x)`: the number of characters of text, or of elements of an array. Which it counts is
/// chosen by the argument's inferred type, so a value of unknown type is taken to be text.
fn length(args: Vec<Expr>, scope: &mut Scope) -> Result<SqlExpr, Error> {
    let arg = iter_one(args).ok_or_else(msg::expected_one_arg)?;
    let is_array = matches!(infer_type(&arg, scope), ValueType::Array(_));
    let a = convert_expr(arg, scope)?;
    Ok(if is_array {
        scope.options.dialect.array_length(a)
    } else {
        char_length(a)
    })
}

/// The built-in functions that take a function as an argument (see [`FunctionArg`]).
const TAKE_A_FUNCTION: [&str; 5] = ["every", "filter", "map", "reduce", "some"];

/// Fails, at the first of `args` that's a function (`@e => …`), if any is: only the functions in
/// [`TAKE_A_FUNCTION`] accept one.
fn reject_function_args(args: &[Expr]) -> Result<(), Error> {
    match args
        .iter()
        .find(|arg| matches!(arg.kind, ExprKind::FunctionArg(_)))
    {
        Some(arg) => Err(msg::function_arg_not_accepted().at(arg.loc)),
        None => Ok(()),
    }
}

/// The arguments of a function that applies a function to the elements of an array: the array, and
/// the function, which must take between `min` and `max` parameters (`params` describes how many,
/// for an error). `name` and `example` name the function being called and show it being used.
fn array_and_function(
    args: Vec<Expr>,
    name: &str,
    example: &str,
    (min, max, params): (usize, usize, &str),
) -> Result<(Expr, FunctionArg), Error> {
    let mut args = args.into_iter();
    let (Some(array), function) = (args.next(), args.next()) else {
        return Err(msg::function_arg_expected(name, example));
    };
    let Some(function) = function else {
        return Err(msg::function_arg_expected(name, example));
    };
    if args.next().is_some() {
        return Err(msg::expected_two_args());
    }
    let loc = function.loc;
    let ExprKind::FunctionArg(function) = function.kind else {
        return Err(msg::function_arg_expected(name, example).at(loc));
    };
    if !(min..=max).contains(&function.params.len()) {
        return Err(msg::function_arg_param_count(name, params).at(loc));
    }
    Ok((array, *function))
}

/// The type of the elements of `array`, or [`ValueType::Unknown`] if it's not known to be an array.
pub(super) fn element_type(array: &Expr, scope: &mut Scope) -> ValueType {
    match infer_type(array, scope) {
        ValueType::Array(element) => *element,
        _ => ValueType::Unknown,
    }
}

/// A name for SQL to give a value that a function's parameter (named `param`) stands for, unique
/// within the query, and unlike any name that the dialect gives the variables of its own lambdas.
fn param_sql_name(param: &str, scope: &mut Scope) -> String {
    let reserved = scope.options.dialect.reserved_lambda_names();
    loop {
        let name = scope.get_alias(param);
        if !reserved.contains(&name.as_str()) {
            return name;
        }
    }
}

/// Compiles the body of a function given as an argument, with its parameters bound to the SQL that
/// stands for their values (each with its name and type). The body sees the variables around it,
/// as an anonymous function's does.
fn convert_function_body(
    body: FunctionBody,
    params: Vec<(String, SqlExpr, ValueType)>,
    scope: &mut Scope,
) -> Result<SqlExpr, Error> {
    let mut env = scope.env();
    for (name, sql, value_type) in params {
        env = scope.bind_sql(&env, name, sql, value_type);
    }
    let env = bind_assignments(body.assignments, env, scope);
    let bans_subqueries = !scope.options.dialect.allows_subquery_in_function_arg();
    // The body is evaluated for each element, within one record, so it can't aggregate records
    // or use a window over them.
    scope.with_env(env, |scope| {
        scope.with_aggregate_ban(AggregateBan::FunctionArg, |scope| {
            scope.with_window_ban(WindowBan::FunctionArg, |scope| match bans_subqueries {
                true => scope.with_subquery_ban(|scope| convert_expr(body.expr, scope)),
                false => convert_expr(body.expr, scope),
            })
        })
    })
}

/// `map`, `filter`, `some`, and `every`: a function applied to each element of an array (and,
/// optionally, its position), whose results `build` combines.
fn element_function(
    args: Vec<Expr>,
    scope: &mut Scope,
    name: &str,
    build: fn(&dyn Dialect, SqlExpr, &ElementNames, SqlExpr) -> SqlExpr,
) -> Result<SqlExpr, Error> {
    let example = format!("{name}(@e => …)");
    let params = "one or two parameters: an element of the array, and optionally its position \
                  (counting from 0)";
    let (array, function) = array_and_function(args, name, &example, (1, 2, params))?;
    let element_type = element_type(&array, scope);
    let a = convert_expr(array, scope)?;
    let mut params = function.params.into_iter();
    let element_param = params.next().expect("the parameters were counted");
    let position_param = params.next();
    let names = ElementNames {
        array: param_sql_name("array", scope),
        element: param_sql_name(&element_param, scope),
        position: param_sql_name(position_param.as_deref().unwrap_or("i"), scope),
        uses_position: position_param.is_some(),
    };
    let options = scope.options;
    let dialect = options.dialect.as_ref();
    let (element, position) = dialect.element_params(&names);
    let mut bindings = vec![(element_param, element, element_type)];
    if let Some(param) = position_param {
        bindings.push((param, position, ValueType::Number));
    }
    let body = convert_function_body(function.body, bindings, scope)?;
    Ok(build(dialect, a, &names, body))
}

/// `reduce(x @acc @e => …)`: the elements of an array combined from the left.
fn reduce(args: Vec<Expr>, scope: &mut Scope) -> Result<SqlExpr, Error> {
    let params = "two parameters: the value so far, and the next element";
    let (array, function) =
        array_and_function(args, "reduce", "reduce(@a @b => @a + @b)", (2, 2, params))?;
    let element_type = element_type(&array, scope);
    let a = convert_expr(array, scope)?;
    let [acc_param, element_param]: [String; 2] = function
        .params
        .try_into()
        .expect("the parameters were counted");
    let names = ReduceNames {
        array: param_sql_name("array", scope),
        acc: param_sql_name(&acc_param, scope),
        element: param_sql_name(&element_param, scope),
        position: param_sql_name("i", scope),
    };
    let options = scope.options;
    let dialect = options.dialect.as_ref();
    let (acc, element) = dialect.reduce_params(&a, &names);
    let bindings = vec![
        (acc_param, acc, element_type.clone()),
        (element_param, element, element_type),
    ];
    let body = convert_function_body(function.body, bindings, scope)?;
    Ok(dialect.array_reduce(a, &names, body))
}

/// The zone classification of an AST argument, for temporal reconciliation (see
/// [`crate::compiler::temporal`]).
fn arg_zone(arg: &Expr, scope: &mut Scope) -> TemporalZone {
    TemporalZone::of(&infer_type(arg, scope))
}

/// `age(x)` = `@now - x`. `@now` is zoned, so when `x` is naive the subtraction is reconciled for
/// dialects that can't mix zoned and naive temporal values.
fn temporal_age(args: Vec<Expr>, scope: &mut Scope) -> Result<SqlExpr, Error> {
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
fn temporal_countdown(args: Vec<Expr>, scope: &mut Scope) -> Result<SqlExpr, Error> {
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
fn temporal_minus(args: Vec<Expr>, scope: &mut Scope) -> Result<SqlExpr, Error> {
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

/// Converts the single date/time argument of a calendar-component function (`year`, `month`, …).
/// On a dialect that can't operate on zoned timestamps (see
/// [`crate::sql::Dialect::coerces_timestamptz`]), a zoned argument is first coerced to a naive
/// timestamp, so the component is read from its UTC wall-clock value.
fn temporal_arg(args: Vec<Expr>, scope: &mut Scope) -> Result<SqlExpr, Error> {
    let arg = iter_one(args).ok_or_else(msg::expected_one_arg)?;
    let zone = arg_zone(&arg, scope);
    let a = convert_expr(arg, scope)?;
    let dialect = scope.options.dialect.as_ref();
    if zone == TemporalZone::Zoned && dialect.coerces_timestamptz() {
        Ok(dialect.coerce_temporal_to_naive(a))
    } else {
        Ok(a)
    }
}

/// `year(x)`, `month(x)`, `day(x)`: one calendar field of a date or timestamp, as an integer.
fn calendar_field(args: Vec<Expr>, scope: &mut Scope, field: &str) -> Result<SqlExpr, Error> {
    Ok(extract_field(field, temporal_arg(args, scope)?))
}

/// `year_month(x)`: a date or timestamp formatted as a `YYYY-MM` string.
fn year_month(args: Vec<Expr>, scope: &mut Scope) -> Result<SqlExpr, Error> {
    let a = temporal_arg(args, scope)?;
    Ok(scope.options.dialect.year_month(a))
}

/// `to_date(x)`: text as a date, or a date or timestamp's date.
fn to_date(args: Vec<Expr>, scope: &mut Scope) -> Result<SqlExpr, Error> {
    Ok(func::cast(temporal_arg(args, scope)?, "DATE"))
}

pub fn get_standard_scalar_functions() -> ScalarFuncMap {
    use TypeRule::*;
    use ValueType::*;
    let (array_of, element_of) = (TypeRule::array_of, TypeRule::element_of);
    // Each row pairs a function's compiler with the rule for the type it returns. `age` and
    // `countdown` produce durations, which `days`, `hours`, and so on turn into numbers. `plus` and
    // `minus` are typed like the operators they stand for, so that `date|plus(1w)` stays a date.
    #[rustfmt::skip]
    let templates: [(&str, ScalarFunc, TypeRule); 58] = [
        ("abs",         |e, s| args_1(e, s, abs),                    SameAsArg(0)),
        ("age",         |e, s| temporal_age(e, s),                  Fixed(Duration)),
        ("and",         |e, s| args_v(e, s, build::cmp::and),        Fixed(Boolean)),
        ("array",       |e, s| array(e, s),                          array_of(UnifyArgs)),
        ("at",          |e, s| at(e, s),                             element_of(SameAsArg(0))),
        ("ceil",        |e, s| args_1(e, s, ceil),                   Fixed(Number)),
        ("concat",      |e, s| args_v(e, s, concat),                 Fixed(Text)),
        ("countdown",   |e, s| temporal_countdown(e, s),            Fixed(Duration)),
        ("day",         |e, s| calendar_field(e, s, "day"),          Fixed(Number)),
        ("days",        |e, s| args_1(e, s, days),                   Fixed(Number)),
        ("divide",      |e, s| args_2(e, s, divide),                 Fixed(Number)),
        ("every",       |e, s| element_function(e, s, "every", |d, a, n, b| d.array_every(a, n, b)), Fixed(Boolean)),
        ("exp",         |e, s| args_1(e, s, exp),                    Fixed(Number)),
        ("filter",      |e, s| element_function(e, s, "filter", |d, a, n, b| d.array_filter(a, n, b)), SameAsArg(0)),
        ("floor",       |e, s| args_1(e, s, floor),                  Fixed(Number)),
        ("gcd",         |e, s| args_2(e, s, gcd),                    Fixed(Number)),
        ("hours",       |e, s| args_1(e, s, hours),                  Fixed(Number)),
        ("if_null",     |e, s| args_v(e, s, coalesce),               UnifyArgs),
        ("is_nan",      |e, s| dialect_1(e, s, |d, a| d.is_nan(a)),  Fixed(Boolean)),
        ("join",        |e, s| with_separator(e, s, |d, a, b| d.join(a, b)), Fixed(Text)),
        ("keep_above",  |e, s| args_v(e, s, greatest),               UnifyArgs),
        ("keep_below",  |e, s| args_v(e, s, least),                  UnifyArgs),
        ("lcm",         |e, s| args_2(e, s, lcm),                    Fixed(Number)),
        ("length",      |e, s| length(e, s),                         Fixed(Number)),
        ("ln",          |e, s| args_1(e, s, ln),                     Fixed(Number)),
        ("lowercase",   |e, s| args_1(e, s, lower),                  Fixed(Text)),
        ("map",         |e, s| element_function(e, s, "map", |d, a, n, b| d.array_map(a, n, b)), array_of(FunctionArgResult(1))),
        ("max",         |e, s| args_v(e, s, greatest),               UnifyArgs),
        ("md5",         |e, s| args_1(e, s, md5),                    Fixed(Text)),
        ("min",         |e, s| args_v(e, s, least),                  UnifyArgs),
        ("minus",       |e, s| temporal_minus(e, s),                Difference),
        ("minutes",     |e, s| args_1(e, s, minutes),                Fixed(Number)),
        ("mod",         |e, s| args_2(e, s, modulo),                 Fixed(Number)),
        ("month",       |e, s| calendar_field(e, s, "month"),        Fixed(Number)),
        ("not",         |e, s| args_1(e, s, not_true),               Fixed(Boolean)),
        ("or",          |e, s| args_v(e, s, build::cmp::or),         Fixed(Boolean)),
        ("plus",        |e, s| args_2(e, s, add),                    Sum),
        ("pow",         |e, s| args_2(e, s, power),                  Fixed(Number)),
        ("reduce",      |e, s| reduce(e, s),                         element_of(SameAsArg(0))),
        ("regex_extract", |e, s| regex_extract(e, s),                Fixed(Text)),
        ("regex_replace", |e, s| regex_replace(e, s),                Fixed(Text)),
        ("round",       |e, s| round(e, s),                          Fixed(Number)),
        ("seconds",     |e, s| args_1(e, s, seconds),                Fixed(Number)),
        ("seconds_to_duration", |e, s| dialect_1(e, s, |d, a| d.seconds_to_duration(a)), Fixed(Duration)),
        ("some",        |e, s| element_function(e, s, "some", |d, a, n, b| d.array_some(a, n, b)), Fixed(Boolean)),
        ("split",       |e, s| with_separator(e, s, |d, a, b| d.split(a, b)), Fixed(Array(Box::new(Text)))),
        ("sqrt",        |e, s| args_1(e, s, sqrt),                   Fixed(Number)),
        ("times",       |e, s| args_2(e, s, multiply),               Fixed(Number)),
        ("to_date",     |e, s| to_date(e, s),                        Fixed(Date)),
        ("to_number",   |e, s| args_1(e, s, |a| func::cast(a, "DOUBLE PRECISION")), Fixed(Number)),
        ("to_timestamp", |e, s| args_1(e, s, |a| func::cast(a, "TIMESTAMP")), Fixed(Time { has_tz: false })),
        ("trim",        |e, s| args_1(e, s, trim),                   Fixed(Text)),
        ("unit_hash",   |e, s| dialect_1(e, s, |d, a| d.unit_hash(a)), Fixed(Number)),
        ("uppercase",   |e, s| args_1(e, s, upper),                  Fixed(Text)),
        ("xor",         |e, s| args_v(e, s, build::cmp::xor),        Fixed(Boolean)),
        ("year",        |e, s| calendar_field(e, s, "year"),         Fixed(Number)),
        ("year_month",  |e, s| year_month(e, s),                     Fixed(Text)),
        ("years",       |e, s| args_1(e, s, years),                  Fixed(Number)),
    ];
    templates
        .into_iter()
        .map(|(name, call, return_type)| (name.to_string(), ScalarFunction { call, return_type }))
        .collect()
}

/// The operand of an aggregate function, split into the path that names the column being
/// aggregated and the scalar functions (if any) piped after it, which are applied to each record's
/// value before the values are aggregated: `#comments.body` and `body|length` in
/// `#comments.body|length%avg`.
pub(super) struct AggregateOperand {
    pub path: Vec<PathPart>,
    /// The operand with its leading path cut down to that path's last part (the column), when
    /// functions are piped after the path. This is compiled like the definition of a computed column
    /// on the table hosting the column: it's evaluated once for each of that table's records being
    /// aggregated, and any other paths within it start from there.
    pub per_record: Option<Expr>,
}

impl AggregateOperand {
    /// Splits an aggregate's operand, or returns `None` when it isn't a column path optionally
    /// followed by piped scalar functions.
    fn split(mut operand: Expr) -> Option<Self> {
        if let ExprKind::Path(path) = operand.kind {
            return Some(Self {
                path,
                per_record: None,
            });
        }
        let path = take_pipeline_head(&mut operand)?;
        Some(Self {
            path,
            per_record: Some(operand),
        })
    }
}

/// The path at the head of a pipeline of scalar functions (`#comments.body` in
/// `#comments.body|length|plus(1)`), or the path itself when `expr` is a path. Returns `None` when
/// `expr` is neither, which an aggregate function's operand may not be.
pub fn aggregate_operand_path(expr: &Expr) -> Option<&[PathPart]> {
    match &expr.kind {
        ExprKind::Path(parts) => Some(parts),
        _ => pipeline_input(expr).and_then(aggregate_operand_path),
    }
}

/// The value piped into `expr`, when `expr` applies a scalar function (named or anonymous) via a
/// pipe.
fn pipeline_input(expr: &Expr) -> Option<&Expr> {
    match &expr.kind {
        ExprKind::Call(call)
            if call.dimension == FunctionDimension::Scalar && call.syntax == CallSyntax::Piped =>
        {
            call.args.first()
        }
        ExprKind::AnonymousFunctionCall(call) => call.args.first(),
        _ => None,
    }
}

/// Like [`pipeline_input`], but mutable.
fn pipeline_input_mut(expr: &mut Expr) -> Option<&mut Expr> {
    match &mut expr.kind {
        ExprKind::Call(call)
            if call.dimension == FunctionDimension::Scalar && call.syntax == CallSyntax::Piped =>
        {
            call.args.first_mut()
        }
        ExprKind::AnonymousFunctionCall(call) => call.args.first_mut(),
        _ => None,
    }
}

/// Finds the path at the head of the pipeline `expr` (see [`aggregate_operand_path`]), cuts it
/// down to its last part within `expr`, and returns the whole path.
fn take_pipeline_head(expr: &mut Expr) -> Option<Vec<PathPart>> {
    if let ExprKind::Path(parts) = &mut expr.kind {
        let last = parts.last()?.clone();
        return Some(std::mem::replace(parts, vec![last]));
    }
    take_pipeline_head(pipeline_input_mut(expr)?)
}

/// Used for an aggregate function that takes one argument
fn agg_1(
    args: Vec<Expr>,
    order_by: Vec<AggregateSortExpr>,
    scope: &mut Scope,
    kind: AggregateKind,
    coalesce_to_zero: bool,
) -> Result<SqlExpr, Error> {
    let arg0 = iter_one(args).ok_or_else(msg::expected_one_arg)?;
    let arg0_loc = arg0.loc;
    let AggregateOperand {
        path: path_parts,
        per_record,
    } = AggregateOperand::split(arg0)
        .ok_or_else(|| msg::aggregate_fn_not_applied_to_a_column().at(arg0_loc))?;
    // Resolve the path from the scope's current table, as `convert_path` does, so that an aggregate
    // inside a scoped condition set (`author{#comments.id%max:>5}`) starts from the scoped table
    // rather than the base table.
    let path_parts = scope.prefixed(&path_parts);
    // Sorting expressions are evaluated from the context of the first `#` table in the path (or
    // the current table, when there is none), so this is the aggregated value's path from there.
    // When functions are applied to the column, no sorting path names the aggregated value (only
    // `\s` sorts by it).
    let value_path = match path_parts
        .iter()
        .position(|part| matches!(part.kind, PathPartKind::TableWithMany(_)))
    {
        Some(index) => path_parts[index + 1..].to_vec(),
        None => path_parts.clone(),
    };
    let value_path = per_record.is_none().then_some(value_path);
    // The parts leading to the table that hosts the aggregated column, from which the functions
    // applied to it are evaluated.
    let host_parts = path_parts[..path_parts.len().saturating_sub(1)].to_vec();
    // A computed column is resolved here, before `clarify_path` consumes the path, but expanded only
    // when the path doesn't traverse a to-many relationship. (Within an aggregated `#` path, the CTE
    // that performs the aggregation expands it instead; see `build_cte_select`.) When functions are
    // applied to the column, the column is compiled along with them instead.
    let computed_column = match per_record {
        None => resolve_computed_column(&path_parts, scope)?,
        Some(_) => None,
    };
    // The path names the value to aggregate, so a foreign key column at its end is that value
    // (e.g. `author%distinct` counts the distinct values of the `author` column).
    let ClarifiedPath { head, tail } = clarify_value_path(path_parts, scope)?;
    match tail {
        // Aggregating data that joins many records (e.g. `#issues.created_at%max`). This is handled
        // by building a CTE that performs the aggregation grouped by the linking column.
        Some(ClarifiedPathTail::ChainToMany((chain_to_many, column_name_opt, anchor_table_id))) => {
            let Some(column_name) = column_name_opt else {
                return Err(msg::aggregate_fn_applied_to_a_path_without_a_column());
            };
            let aggregate_expr_template = AggregateExprTemplate::new(
                column_name,
                per_record.map(|expr| (expr, scope.env())),
                kind,
                order_by,
                coalesce_to_zero,
                anchor_table_id,
                value_path,
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
            check_aggregate_ban(scope)?;
            scope.with_window_ban(WindowBan::Aggregate, |scope| {
                let reference = match (per_record, computed_column) {
                    // Functions applied to the column are evaluated relative to the table hosting
                    // it, like a computed column's definition, but they see the variables of
                    // wherever they're written.
                    (Some(expr), _) => {
                        let env = scope.env();
                        scope.with_aggregate_ban(AggregateBan::AggregatedValue, |scope| {
                            convert_definition_with_env(expr, host_parts, env, scope)
                        })?
                    }
                    // A computed column's definition is aggregated in its place, evaluated
                    // relative to the table hosting it (as `convert_path` does for an unaggregated
                    // reference).
                    (None, Some((head_parts, expr, expansion))) => {
                        let _guard = scope.enter_expansion(expansion)?;
                        scope.with_aggregate_ban(AggregateBan::AggregatedValue, |scope| {
                            convert_definition(expr, head_parts, scope)
                        })?
                    }
                    (None, None) => {
                        let table_name = match head {
                            Some(chain_to_one) => scope.join_chain_to_one(&chain_to_one),
                            None => scope.get_base_alias().to_owned(),
                        };
                        scope.table_column_expr(&table_name, &column_name)
                    }
                };
                let sorts = resolve_aggregate_sorting(order_by, value_path.as_deref(), scope)?;
                let dialect = scope.options.dialect.as_ref();
                match compile_aggregate(kind, reference, &sorts, dialect)? {
                    CompiledAggregate::Expr(expr) => Ok(expr),
                    // A two-level aggregation would need to restructure the whole grouped query,
                    // which isn't supported.
                    CompiledAggregate::NeedsTwoLevelList(ListMode::All) => {
                        let err = msg::aggregated_sort_in_grouped_list();
                        let aggregated =
                            ResolvedSort::find(&sorts, |k| matches!(k, SortKey::Aggregated(_)));
                        Err(err.at(aggregated.unwrap_or(Loc::NONE)))
                    }
                    CompiledAggregate::NeedsTwoLevelList(ListMode::Top) => {
                        Err(msg::top_in_grouped_query())
                    }
                }
            })
        }
        None => Err(msg::aggregate_fn_applied_to_a_path_without_a_column()),
    }
}

pub fn get_standard_aggregate_functions() -> AggregateFuncMap {
    use AggregateKind::*;
    use TypeRule::*;
    use ValueType::*;
    let array_of = TypeRule::array_of;
    // Each wrapper ignores the trailing `&dyn Dialect` argument except `product`, whose SQL differs
    // between dialects. The trailing `TypeRule` describes the type each aggregate returns: `max`/`min`
    // preserve their argument's type, `list`/`list_all`/`top` wrap it in an array, and the rest are
    // fixed.
    #[rustfmt::skip]
    let templates: [(&str, AggregateFunc, TypeRule); 12] = [
        ("all_true", |e, ob, s| agg_1(e, ob, s, Wrapped(|a, ob, _| bool_and(a, ob)), false),         Fixed(Boolean)),
        ("any_true", |e, ob, s| agg_1(e, ob, s, Wrapped(|a, ob, _| bool_or(a, ob)), false),          Fixed(Boolean)),
        ("avg",      |e, ob, s| agg_1(e, ob, s, Wrapped(|a, ob, _| avg(a, ob)), false),              Fixed(Number)),
        ("count",    |e, ob, s| agg_1(e, ob, s, Wrapped(|a, ob, _| count(a, ob)), true),             Fixed(Number)),
        ("distinct", |e, ob, s| agg_1(e, ob, s, Wrapped(|a, ob, _| count_distinct(a, ob)), true),    Fixed(Number)),
        ("list",     |e, ob, s| agg_1(e, ob, s, DistinctList, false),                                array_of(SameAsArg(0))),
        ("list_all", |e, ob, s| agg_1(e, ob, s, Wrapped(|a, ob, _| array_agg(a, ob)), false),        array_of(SameAsArg(0))),
        ("max",      |e, ob, s| agg_1(e, ob, s, Wrapped(|a, ob, _| max(a, ob)), false),              SameAsArg(0)),
        ("min",      |e, ob, s| agg_1(e, ob, s, Wrapped(|a, ob, _| min(a, ob)), false),              SameAsArg(0)),
        ("product",  |e, ob, s| agg_1(e, ob, s, Wrapped(|a, _ob, d| d.aggregate_product(a)), false), Fixed(Number)),
        ("sum",      |e, ob, s| agg_1(e, ob, s, Wrapped(|a, ob, _| sum(a, ob)), false),              Fixed(Number)),
        ("top",      |e, ob, s| agg_1(e, ob, s, DistinctTop, false),                                 array_of(SameAsArg(0))),
    ];
    templates
        .into_iter()
        .map(|(name, call, return_type)| {
            (name.to_string(), AggregateFunction { call, return_type })
        })
        .collect()
}
