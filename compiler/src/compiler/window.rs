//! Window functions: `%%( partition\p ordering\s )%func(args)`.
//!
//! A window function compiles to a SQL `func(args) OVER (PARTITION BY ... ORDER BY ...)` expression.
//! Because SQL forbids window functions in a `WHERE` clause, a stage whose conditions reference a
//! window function is automatically wrapped: the window values are computed in an inner query (a
//! CTE) over the rows that pass the stage's other conditions, and the rest of the stage reads from
//! it, applying the window predicate in its own `WHERE`. See [`super::mod`]'s `compile_stage` for
//! the wrapping logic; this module supplies the condition splitting it needs
//! ([`extract_window_conditions`]).
//!
//! SQL also forbids window functions in aggregates, in other window functions, and (in the CTEs
//! that Querydown builds for related records) in `WHERE` clauses that can't be wrapped. Those
//! contexts set a [`WindowBan`] on the scope, so that a window function reaching them — e.g.
//! through a computed column — is reported as a compile error rather than producing invalid SQL.

use querydown_parser::ast::{ComparisonSide, Expr, ExprKind, Loc, WindowFn};

use crate::{
    compiler::{
        expr::convert_expr,
        functions::TypeRule,
        paths::render_order_by,
        scope::{Scope, WindowBan},
    },
    errors::msg,
    schema::ValueType,
    sql::tree::SqlExpr,
    Error,
};

/// The SQL realization of a window function: its SQL name, the inclusive range of value arguments it
/// accepts (not counting partition/ordering, which come from the window definition), and the rule
/// for the type of value it produces (used by type inference).
struct WindowFnSpec {
    sql_name: &'static str,
    min_args: usize,
    max_args: usize,
    return_type: TypeRule,
}

/// Looks up a window function by its Querydown name, returning how it maps to SQL. The set is
/// restricted to window functions that both Postgres and DuckDB support.
fn window_fn_spec(name: &str) -> Option<WindowFnSpec> {
    let spec = |sql_name, min_args, max_args, return_type| {
        Some(WindowFnSpec {
            sql_name,
            min_args,
            max_args,
            return_type,
        })
    };
    use TypeRule::*;
    use ValueType::*;
    match name {
        // Ranking functions — no value arguments.
        "row_number" => spec("row_number", 0, 0, Fixed(Number)),
        "rank" => spec("rank", 0, 0, Fixed(Number)),
        "dense_rank" => spec("dense_rank", 0, 0, Fixed(Number)),
        "percent_rank" => spec("percent_rank", 0, 0, Fixed(Number)),
        "cume_dist" => spec("cume_dist", 0, 0, Fixed(Number)),
        "ntile" => spec("ntile", 1, 1, Fixed(Number)),
        // Offset / positional functions return one of the partition's values unchanged.
        "lag" => spec("lag", 1, 3, SameAsArg(0)),
        "lead" => spec("lead", 1, 3, SameAsArg(0)),
        "first_value" => spec("first_value", 1, 1, SameAsArg(0)),
        "last_value" => spec("last_value", 1, 1, SameAsArg(0)),
        "nth_value" => spec("nth_value", 2, 2, SameAsArg(0)),
        // Aggregate functions usable as window functions.
        "count" => spec("count", 0, 1, Fixed(Number)),
        "sum" => spec("sum", 1, 1, Fixed(Number)),
        "avg" => spec("avg", 1, 1, Fixed(Number)),
        "min" => spec("min", 1, 1, SameAsArg(0)),
        "max" => spec("max", 1, 1, SameAsArg(0)),
        _ => None,
    }
}

/// The type-inference rule for a window function, or `None` if the name isn't a known window
/// function. Used by [`crate::compiler::typing`] to propagate types through window expressions.
pub(super) fn window_fn_return_type(name: &str) -> Option<TypeRule> {
    window_fn_spec(name).map(|spec| spec.return_type)
}

/// Converts a window function application into a SQL `func(...) OVER (...)` expression.
pub fn convert_window(window: WindowFn, scope: &mut Scope) -> Result<SqlExpr, Error> {
    if let Some(reference) = scope.substitute_window(&window) {
        return Ok(reference);
    }
    if let Some(ban) = scope.window_ban() {
        return Err(match ban {
            WindowBan::Condition => msg::window_in_indirect_condition(),
            WindowBan::RelatedRecords => msg::window_in_related_records(),
            WindowBan::Aggregate => msg::window_in_aggregate(),
            WindowBan::Nested => msg::window_in_window(),
        });
    }
    scope.note_compiled_window(window.function_loc);
    scope.with_window_ban(WindowBan::Nested, |scope| {
        convert_window_parts(window, scope)
    })
}

fn convert_window_parts(window: WindowFn, scope: &mut Scope) -> Result<SqlExpr, Error> {
    let WindowFn {
        function,
        function_loc,
        args,
        partition_by,
        order_by,
    } = window;

    let spec = window_fn_spec(&function)
        .ok_or_else(|| msg::unknown_window_function(&function).at(function_loc))?;
    if args.len() < spec.min_args || args.len() > spec.max_args {
        let err =
            msg::window_fn_wrong_arg_count(&function, spec.min_args, spec.max_args, args.len());
        return Err(err.at(function_loc));
    }

    // The function call itself. A bare `count` (no argument) becomes `count(*)`; everything else
    // renders its arguments directly.
    let call = if args.is_empty() && function == "count" {
        "count(*)".to_string()
    } else {
        let mut arg_sqls = Vec::with_capacity(args.len());
        for arg in args {
            arg_sqls.push(convert_expr(arg, scope)?.content);
        }
        format!("{}({})", spec.sql_name, arg_sqls.join(", "))
    };

    let mut partition_sqls = Vec::with_capacity(partition_by.len());
    for expr in partition_by {
        partition_sqls.push(convert_expr(expr, scope)?.content);
    }
    let order_by_str = render_order_by(order_by, scope)?;

    let mut over = String::new();
    if !partition_sqls.is_empty() {
        over.push_str(&format!("PARTITION BY {}", partition_sqls.join(", ")));
    }
    if !order_by_str.is_empty() {
        if !over.is_empty() {
            over.push(' ');
        }
        over.push_str(&format!("ORDER BY {}", order_by_str));
    }

    Ok(SqlExpr::atom(format!("{} OVER ({})", call, over)))
}

/// Collects the window functions within `expr` whose values a wrapping query can supply (see
/// [`extract_window_conditions`]), each with where it was written, skipping duplicates.
///
/// This descends only into the parts of an expression that are compiled in the stage's own scope,
/// without any path prefix, so that each collected window compiles the same way in the wrapping
/// query as it would have here. Windows elsewhere — within a scoped condition (`issue{…}`), a
/// has-quantity condition (`++#comments{…}`), an anonymous function's body, or a subquery — aren't
/// collected, and are reported as errors when compiled (see [`WindowBan`]).
fn collect_windows(expr: &Expr, windows: &mut Vec<(WindowFn, Loc)>) {
    match &expr.kind {
        ExprKind::Window(window) => {
            if !windows.iter().any(|(w, _)| w == window) {
                windows.push((window.clone(), expr.loc));
            }
        }
        ExprKind::Not(e) => collect_windows(e, windows),
        ExprKind::Product(a, b)
        | ExprKind::Quotient(a, b)
        | ExprKind::Sum(a, b)
        | ExprKind::Difference(a, b) => {
            collect_windows(a, windows);
            collect_windows(b, windows);
        }
        ExprKind::Call(call) => call.args.iter().for_each(|a| collect_windows(a, windows)),
        ExprKind::AnonymousFunctionCall(call) => {
            call.args.iter().for_each(|a| collect_windows(a, windows))
        }
        ExprKind::Case(case) => {
            for variant in &case.variants {
                collect_windows(&variant.condition, windows);
                collect_windows(&variant.value, windows);
            }
            collect_windows(&case.fallback, windows);
        }
        ExprKind::ConditionSet(cs) => cs.entries.iter().for_each(|e| collect_windows(e, windows)),
        ExprKind::Comparison(c) => {
            for side in [&c.left, &c.right] {
                match side {
                    ComparisonSide::Expr(e) => collect_windows(e, windows),
                    ComparisonSide::Expansion(cs) => {
                        cs.entries.iter().for_each(|e| collect_windows(e, windows))
                    }
                    ComparisonSide::Range(r) => {
                        collect_windows(&r.lower.expr, windows);
                        collect_windows(&r.upper.expr, windows);
                    }
                }
            }
        }
        ExprKind::ScopedConditionSet(_)
        | ExprKind::HasQuantity(_)
        | ExprKind::Subquery(_)
        | ExprKind::Number(_)
        | ExprKind::Date(_)
        | ExprKind::Duration(_)
        | ExprKind::String(_)
        | ExprKind::Variable(_)
        | ExprKind::Path(_) => {}
    }
}

/// The alias prefix for the hidden columns that carry window-function values out of the inner query
/// of a wrapped stage.
pub const WINDOW_PROJECTION_ALIAS_PREFIX: &str = "qdwin";

/// The result of separating a stage's conditions into those that may be applied before its window
/// functions are computed and those that must be applied afterward, because they reference them.
pub struct SplitConditions {
    /// Conditions without window functions, to be applied in the inner query, before the window
    /// functions are computed.
    pub inner: querydown_parser::ast::ConditionSet,
    /// Conditions referencing window functions, to be applied in the outer (wrapping) query. Empty
    /// when no wrapping is needed.
    pub outer: querydown_parser::ast::ConditionSet,
    /// The window functions within `outer`, to be computed by the inner query, each with where it
    /// was written.
    pub windows: Vec<(WindowFn, Loc)>,
}

/// Splits a stage's conditions into inner and outer parts for window-function wrapping. When the
/// top-level conjunction is `AND`, each entry is routed independently: entries free of window
/// functions stay inner, while those that reference one move outer. Otherwise (e.g. a top-level `OR`
/// that mentions a window), the whole set moves outer.
pub fn extract_window_conditions(
    conditions: querydown_parser::ast::ConditionSet,
) -> SplitConditions {
    use querydown_parser::ast::{ConditionSet, Conjunction};

    let windows_of = |entry: &Expr| {
        let mut windows = Vec::new();
        collect_windows(entry, &mut windows);
        windows
    };
    let mut windows = Vec::new();
    for entry in &conditions.entries {
        collect_windows(entry, &mut windows);
    }
    if windows.is_empty() {
        return SplitConditions {
            inner: conditions,
            outer: ConditionSet::default(),
            windows,
        };
    }

    if conditions.conjunction == Conjunction::And {
        let (outer_entries, inner_entries) = conditions
            .entries
            .into_iter()
            .partition(|entry| !windows_of(entry).is_empty());
        SplitConditions {
            inner: ConditionSet::via_and(inner_entries),
            outer: ConditionSet::via_and(outer_entries),
            windows,
        }
    } else {
        SplitConditions {
            inner: ConditionSet::default(),
            outer: conditions,
            windows,
        }
    }
}
