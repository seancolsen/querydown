//! Static type inference for expressions.
//!
//! [`infer_type`] performs a best-effort, compile-time classification of the [`ValueType`] an
//! expression will produce, without actually compiling it to SQL. It is deliberately shallow:
//! anything it can't confidently classify becomes [`ValueType::Unknown`].
//!
//! Two features rely on it:
//!
//! - The match operator (`:`) uses it to decide between case-insensitive "contains" (for text) and
//!   exact equality.
//! - The date/duration comparison magic (see [`super::comparisons`]) uses it to recognize the
//!   date-like left-hand side of a `date:<duration` comparison.
//!
//! Crucially, the inference propagates through function calls (built-in, user-defined, and
//! anonymous), arithmetic, `CASE` expressions, window functions, and variables — so a datetime run
//! through `max` is still recognized as a datetime, for example.

use querydown_parser::ast::{
    AnonymousFunctionCall, Call, Case, Expr, FunctionBody, FunctionDimension, PathPart, WindowFn,
};

use crate::{
    compiler::{
        constants::{VAR_FALSE, VAR_INFINITY, VAR_NOW, VAR_NULL, VAR_TRUE},
        expr::resolve_computed_column,
        functions::TypeRule,
        paths::{clarify_path, ClarifiedPathTail},
        scope::Scope,
        temporal::now_has_tz,
        window::window_fn_return_type,
    },
    schema::{Table, ValueType},
};

/// Best-effort classification of the [`ValueType`] that `expr` will produce. Returns
/// [`ValueType::Unknown`] for anything that can't be confidently classified.
///
/// This takes `&mut Scope` (rather than `&Scope`) because resolving the body of a user-defined or
/// anonymous function call requires temporarily binding its parameters, mirroring how the expression
/// would actually be compiled.
pub fn infer_type(expr: &Expr, scope: &mut Scope) -> ValueType {
    match expr {
        Expr::Number(_) => ValueType::Number,
        Expr::Date(_) => ValueType::Date,
        Expr::String(_) => ValueType::Text,
        // A duration literal has no dedicated value type; it only ever appears as the right-hand side
        // of the date/duration comparison magic, which matches it structurally rather than by type.
        Expr::Duration(_) => ValueType::Unknown,
        Expr::Variable(name) => infer_variable(name, scope),
        Expr::Path(parts) => infer_path(parts, scope),
        Expr::ConditionSet(_)
        | Expr::ScopedConditionSet(_)
        | Expr::HasQuantity(_)
        | Expr::Comparison(_)
        | Expr::Not(_) => ValueType::Boolean,
        Expr::Case(case) => infer_case(case, scope),
        Expr::Call(call) => infer_call(call, scope),
        // Multiplication and division always yield numbers. Addition and subtraction preserve the
        // type of their left operand, so `date + interval` stays a date (the common, useful case).
        // `date - date` is really an interval, not a date, so this is an approximation — but it only
        // affects whether the date/duration comparison magic fires, never the SQL we emit.
        Expr::Product(..) | Expr::Quotient(..) => ValueType::Number,
        Expr::Sum(a, _) | Expr::Difference(a, _) => infer_type(a, scope),
        Expr::Window(window) => infer_window(window, scope),
        Expr::AnonymousFunctionCall(call) => infer_anonymous_function_call(call, scope),
        // A scalar subquery's type would require inferring the type of its single result column,
        // which we don't attempt.
        Expr::Subquery(_) => ValueType::Unknown,
    }
}

fn infer_variable(name: &str, scope: &mut Scope) -> ValueType {
    match name {
        VAR_NOW => ValueType::Time {
            has_tz: now_has_tz(scope.options.dialect.as_ref()),
        },
        VAR_TRUE | VAR_FALSE => ValueType::Boolean,
        VAR_INFINITY => ValueType::Number,
        VAR_NULL => ValueType::Unknown,
        // A user-defined constant or a function parameter: infer the type of its bound expression.
        // We clone it to release the immutable borrow of `scope` before recursing.
        _ => match scope.get_variable(name).cloned() {
            Some(expr) => infer_type(&expr, scope),
            None => ValueType::Unknown,
        },
    }
}

fn infer_path(parts: &[PathPart], scope: &mut Scope) -> ValueType {
    // Mirror `convert_path`: apply the scope's path prefix, then resolve a computed column (whose
    // definition is an arbitrary expression) before falling back to clarifying the path to a column.
    let prefixed_parts: Vec<PathPart> = scope
        .path_prefix
        .iter()
        .cloned()
        .chain(parts.iter().cloned())
        .collect();
    match resolve_computed_column(&prefixed_parts, scope) {
        Ok(Some((head_parts, expr))) => return infer_with_path_prefix(&expr, head_parts, scope),
        Ok(None) => {}
        Err(_) => return ValueType::Unknown,
    }
    let Ok(clarified) = clarify_path(prefixed_parts, scope) else {
        return ValueType::Unknown;
    };
    let ending_table_column_type =
        |scope: &Scope, table_id, column_name: &str| match scope.schema.tables.get(&table_id) {
            Some(table) => column_type(scope, table, column_name),
            None => ValueType::Unknown,
        };
    match (clarified.head, clarified.tail) {
        (None, Some(ClarifiedPathTail::Column(column_name))) => {
            column_type(scope, scope.get_base_table(), &column_name)
        }
        (Some(chain_to_one), Some(ClarifiedPathTail::Column(column_name))) => {
            ending_table_column_type(scope, chain_to_one.get_ending_table_id(), &column_name)
        }
        // The column reached at the end of a to-many chain (e.g. `#comments.created_at`). Aggregating
        // it preserves this type, which is what lets `#comments.created_at%max` be recognized as a
        // datetime.
        (_, Some(ClarifiedPathTail::ChainToMany((chain, Some(column_name), _)))) => {
            ending_table_column_type(scope, chain.get_ending_table_id(), &column_name)
        }
        _ => ValueType::Unknown,
    }
}

/// Resolves the [`ValueType`] of `column_name` within `table`, using the scope's identifier
/// resolution to match the name the same way the compiler does.
fn column_type(scope: &Scope, table: &Table, column_name: &str) -> ValueType {
    scope
        .options
        .resolve_identifier(&table.column_lookup, column_name)
        .and_then(|id| table.columns.get(id))
        .map(|column| column.r#type.clone())
        .unwrap_or(ValueType::Unknown)
}

/// Infers the type of `expr` with `path_prefix` temporarily in effect, restoring the previous prefix
/// afterward. Mirrors [`super::expr::convert_expr_with_path_prefix`], used when a computed column's
/// definition must be evaluated relative to its host table.
fn infer_with_path_prefix(expr: &Expr, path_prefix: Vec<PathPart>, scope: &mut Scope) -> ValueType {
    let saved = std::mem::replace(&mut scope.path_prefix, path_prefix);
    let result = infer_type(expr, scope);
    scope.path_prefix = saved;
    result
}

/// A `CASE` expression's type is the common type of all its result values (each variant's value plus
/// the fallback). If they don't all agree, the type is [`ValueType::Unknown`].
fn infer_case(case: &Case, scope: &mut Scope) -> ValueType {
    let mut types = Vec::with_capacity(case.variants.len() + 1);
    for variant in &case.variants {
        types.push(infer_type(&variant.value, scope));
    }
    types.push(infer_type(case.fallback.as_ref(), scope));
    unify_types(types)
}

fn infer_call(call: &Call, scope: &mut Scope) -> ValueType {
    let rule = match call.dimension {
        FunctionDimension::Scalar => {
            // Built-in scalar functions take precedence; otherwise consult a user-defined function,
            // whose return type is the type of its (inlined) body.
            if let Some(func) = scope.get_scalar_function(&call.name) {
                func.return_type.clone()
            } else if let Some(def) = scope.get_user_function(&call.name).cloned() {
                return infer_function_body(def.params, def.body, &call.args, scope);
            } else {
                return ValueType::Unknown;
            }
        }
        FunctionDimension::Aggregate => match scope.get_aggregate_function(&call.name) {
            Some(func) => func.return_type.clone(),
            None => return ValueType::Unknown,
        },
    };
    apply_type_rule(&rule, &call.args, scope)
}

fn infer_anonymous_function_call(call: &AnonymousFunctionCall, scope: &mut Scope) -> ValueType {
    infer_function_body(call.params.clone(), call.body.clone(), &call.args, scope)
}

/// Infers the return type of a user-defined or anonymous function by binding its parameters and
/// local assignments, then inferring the type of its body — mirroring how the function would be
/// inlined during compilation.
fn infer_function_body(
    params: Vec<String>,
    body: FunctionBody,
    args: &[Expr],
    scope: &mut Scope,
) -> ValueType {
    if params.len() != args.len() {
        return ValueType::Unknown;
    }
    let FunctionBody { assignments, expr } = body;
    let bindings: Vec<(String, Expr)> = params
        .into_iter()
        .zip(args.iter().cloned())
        .chain(assignments.into_iter().map(|a| (a.name, a.expr)))
        .collect();
    scope.with_variable_bindings(bindings, |scope| infer_type(&expr, scope))
}

fn infer_window(window: &WindowFn, scope: &mut Scope) -> ValueType {
    match window_fn_return_type(&window.function) {
        Some(rule) => apply_type_rule(&rule, &window.args, scope),
        None => ValueType::Unknown,
    }
}

/// Resolves a [`TypeRule`] against a call's arguments to a concrete [`ValueType`].
fn apply_type_rule(rule: &TypeRule, args: &[Expr], scope: &mut Scope) -> ValueType {
    match rule {
        TypeRule::Fixed(value_type) => value_type.clone(),
        TypeRule::SameAsArg(index) => match args.get(*index) {
            Some(arg) => infer_type(arg, scope),
            None => ValueType::Unknown,
        },
        TypeRule::UnifyArgs => {
            let types: Vec<ValueType> = args.iter().map(|arg| infer_type(arg, scope)).collect();
            unify_types(types)
        }
        TypeRule::ArrayOfArg(index) => match args.get(*index) {
            Some(arg) => ValueType::Array(Box::new(infer_type(arg, scope))),
            None => ValueType::Unknown,
        },
    }
}

/// Reduces a set of types to their common type: the shared type if every entry agrees, otherwise
/// [`ValueType::Unknown`]. An empty set, or any [`ValueType::Unknown`] entry, yields `Unknown`.
fn unify_types(types: impl IntoIterator<Item = ValueType>) -> ValueType {
    let mut iter = types.into_iter();
    let Some(first) = iter.next() else {
        return ValueType::Unknown;
    };
    if first == ValueType::Unknown {
        return ValueType::Unknown;
    }
    for next in iter {
        if next != first {
            return ValueType::Unknown;
        }
    }
    first
}
