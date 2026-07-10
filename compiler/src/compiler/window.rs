//! Window functions: `%%( partition\p ordering\s )%func(args)`.
//!
//! A window function compiles to a SQL `func(args) OVER (PARTITION BY ... ORDER BY ...)` expression.
//! Because SQL forbids window functions in a `WHERE` clause, a stage whose conditions reference a
//! window function is automatically wrapped: the window values are computed in an inner query (a
//! CTE) and the predicate is applied in an outer query that reads from it. See
//! [`super::mod`]'s `compile_stage` for the wrapping logic; this module supplies the pieces it needs
//! ([`expr_contains_window`] and [`extract_window_conditions`]).

use querydown_parser::ast::{Comparison, ComparisonSide, Expr, WindowFn};

use crate::{
    compiler::{expr::convert_expr, functions::TypeRule, paths::render_order_by, scope::Scope},
    errors::msg,
    schema::ValueType,
    sql::tree::SqlExpr,
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
pub fn convert_window(window: WindowFn, scope: &mut Scope) -> Result<SqlExpr, String> {
    let WindowFn {
        function,
        args,
        partition_by,
        order_by,
    } = window;

    let spec = window_fn_spec(&function).ok_or_else(|| msg::unknown_window_function(&function))?;
    if args.len() < spec.min_args || args.len() > spec.max_args {
        return Err(msg::window_fn_wrong_arg_count(
            &function,
            spec.min_args,
            spec.max_args,
            args.len(),
        ));
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

/// Whether an expression contains a window function anywhere within it.
pub fn expr_contains_window(expr: &Expr) -> bool {
    match expr {
        Expr::Window(_) => true,
        Expr::Not(e) => expr_contains_window(e),
        Expr::Product(a, b) | Expr::Quotient(a, b) | Expr::Sum(a, b) | Expr::Difference(a, b) => {
            expr_contains_window(a) || expr_contains_window(b)
        }
        Expr::Call(call) => call.args.iter().any(expr_contains_window),
        Expr::Case(case) => {
            case.variants
                .iter()
                .any(|v| expr_contains_window(&v.condition) || expr_contains_window(&v.value))
                || expr_contains_window(&case.fallback)
        }
        Expr::ConditionSet(cs) => cs.entries.iter().any(expr_contains_window),
        Expr::ScopedConditionSet(s) => s.condition_set.entries.iter().any(expr_contains_window),
        Expr::Comparison(c) => side_contains_window(&c.left) || side_contains_window(&c.right),
        Expr::AnonymousFunctionCall(c) => {
            c.args.iter().any(expr_contains_window)
                || c.body
                    .assignments
                    .iter()
                    .any(|a| expr_contains_window(&a.expr))
                || expr_contains_window(&c.body.expr)
        }
        Expr::Number(_)
        | Expr::Date(_)
        | Expr::Duration(_)
        | Expr::String(_)
        | Expr::Variable(_)
        | Expr::Path(_)
        // A scalar subquery is a self-contained query; an outer window function cannot reach into it.
        | Expr::Subquery(_)
        | Expr::HasQuantity(_) => false,
    }
}

fn side_contains_window(side: &ComparisonSide) -> bool {
    match side {
        ComparisonSide::Expr(e) => expr_contains_window(e),
        ComparisonSide::Expansion(cs) => cs.entries.iter().any(expr_contains_window),
        ComparisonSide::Range(r) => {
            expr_contains_window(&r.lower.expr) || expr_contains_window(&r.upper.expr)
        }
    }
}

/// A window function extracted from a stage's conditions, paired with the alias under which its value
/// is projected by the inner (wrapped) query so the outer query can filter on it.
pub struct WindowProjection {
    pub alias: String,
    pub window: WindowFn,
}

/// The alias prefix for the hidden columns that carry window-function values out of the inner query
/// of a wrapped stage.
const WINDOW_PROJECTION_ALIAS_PREFIX: &str = "qdwin";

/// Replaces every window function within `expr` with a reference to a projected column, collecting
/// the extracted windows into `projections`. The returned expression is suitable for the outer query
/// of a wrapped stage, where each window value is available as a plain column.
fn extract_windows(expr: Expr, projections: &mut Vec<WindowProjection>) -> Expr {
    use querydown_parser::ast::PathPart;
    match expr {
        Expr::Window(window) => {
            let alias = format!("{}{}", WINDOW_PROJECTION_ALIAS_PREFIX, projections.len());
            projections.push(WindowProjection {
                alias: alias.clone(),
                window,
            });
            Expr::Path(vec![PathPart::Column(alias)])
        }
        Expr::Not(e) => Expr::Not(Box::new(extract_windows(*e, projections))),
        Expr::Product(a, b) => Expr::Product(
            Box::new(extract_windows(*a, projections)),
            Box::new(extract_windows(*b, projections)),
        ),
        Expr::Quotient(a, b) => Expr::Quotient(
            Box::new(extract_windows(*a, projections)),
            Box::new(extract_windows(*b, projections)),
        ),
        Expr::Sum(a, b) => Expr::Sum(
            Box::new(extract_windows(*a, projections)),
            Box::new(extract_windows(*b, projections)),
        ),
        Expr::Difference(a, b) => Expr::Difference(
            Box::new(extract_windows(*a, projections)),
            Box::new(extract_windows(*b, projections)),
        ),
        Expr::Comparison(c) => {
            let Comparison {
                left,
                operator,
                right,
            } = *c;
            Expr::Comparison(Box::new(Comparison {
                left: extract_windows_from_side(left, projections),
                operator,
                right: extract_windows_from_side(right, projections),
            }))
        }
        Expr::ConditionSet(mut cs) => {
            cs.entries = cs
                .entries
                .into_iter()
                .map(|e| extract_windows(e, projections))
                .collect();
            Expr::ConditionSet(cs)
        }
        // Window functions inside a function call, case expression, or has-quantity are not extracted
        // (and won't appear there in practice); such conditions would surface a "window in WHERE"
        // error from the database, which is acceptable for these unusual shapes.
        other => other,
    }
}

fn extract_windows_from_side(
    side: ComparisonSide,
    projections: &mut Vec<WindowProjection>,
) -> ComparisonSide {
    match side {
        ComparisonSide::Expr(e) => ComparisonSide::Expr(extract_windows(e, projections)),
        ComparisonSide::Expansion(mut cs) => {
            cs.entries = cs
                .entries
                .into_iter()
                .map(|e| extract_windows(e, projections))
                .collect();
            ComparisonSide::Expansion(cs)
        }
        ComparisonSide::Range(r) => ComparisonSide::Range(r),
    }
}

/// The result of separating a stage's conditions into those that may stay in the inner query's
/// `WHERE` and those that must be applied in an outer query because they reference window functions.
pub struct SplitConditions {
    /// Conditions without window functions, to be applied in the inner query.
    pub inner: querydown_parser::ast::ConditionSet,
    /// Conditions referencing window functions, with each window replaced by a reference to its
    /// projected alias, to be applied in the outer (wrapping) query. Empty when no wrapping is
    /// needed.
    pub outer: querydown_parser::ast::ConditionSet,
    /// The window functions extracted from `outer`, to be projected by the inner query.
    pub projections: Vec<WindowProjection>,
}

/// Splits a stage's conditions into inner and outer parts for window-function wrapping. When the
/// top-level conjunction is `AND`, each entry is routed independently: entries free of window
/// functions stay inner, while those that reference one move outer. Otherwise (e.g. a top-level `OR`
/// that mentions a window), the whole set moves outer.
pub fn extract_window_conditions(
    conditions: querydown_parser::ast::ConditionSet,
) -> SplitConditions {
    use querydown_parser::ast::{ConditionSet, Conjunction};

    let has_window = conditions.entries.iter().any(expr_contains_window);
    if !has_window {
        return SplitConditions {
            inner: conditions,
            outer: ConditionSet::default(),
            projections: Vec::new(),
        };
    }

    let mut projections = Vec::new();
    if conditions.conjunction == Conjunction::And {
        let mut inner_entries = Vec::new();
        let mut outer_entries = Vec::new();
        for entry in conditions.entries {
            if expr_contains_window(&entry) {
                outer_entries.push(extract_windows(entry, &mut projections));
            } else {
                inner_entries.push(entry);
            }
        }
        SplitConditions {
            inner: ConditionSet::via_and(inner_entries),
            outer: ConditionSet::via_and(outer_entries),
            projections,
        }
    } else {
        let conjunction = conditions.conjunction;
        let outer_entries = conditions
            .entries
            .into_iter()
            .map(|e| extract_windows(e, &mut projections))
            .collect();
        SplitConditions {
            inner: ConditionSet::default(),
            outer: ConditionSet {
                conjunction,
                entries: outer_entries,
            },
            projections,
        }
    }
}
