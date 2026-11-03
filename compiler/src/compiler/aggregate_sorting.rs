//! Sorting within aggregate functions, e.g. the `(\\name)` in `#labels.name%list(\\name)`.
//!
//! Each sorting entry is classified as one of three kinds of [`SortKey`], because they compile
//! differently — most notably for `list`, which removes duplicate values:
//!
//! - **The aggregated value itself**, written as `\s` or as a path that resolves to the same column
//!   via the same route as the aggregated value (e.g. `#comments.user.username%list(\\user.username)`).
//!   `list` can keep its `DISTINCT` while sorting by this.
//! - **An aggregated sort key**, such as `\\created_at%min` or `\\%count`. Within `list` these are
//!   computed once per distinct value (over the records that share that value), which requires a
//!   two-level aggregation — see [`crate::compiler::paths::build_cte_select`].
//! - **A row-level sort key**, such as `\\created_at`, which has one value per aggregated record.
//!   SQL can't sort distinct values by such a key (the duplicates being merged may disagree on it),
//!   so `list` rejects it.

use querydown_parser::ast::{
    AggregateSortExpr, AggregateSortKey, ComparisonSide, Expr, FunctionDimension, NullsSort,
    PathPart, SortDirection,
};

use crate::{
    compiler::{expr::convert_expr, paths::paths_resolve_alike, scope::Scope},
    errors::msg,
    sql::expr::SqlExpr,
};

pub enum SortKey {
    /// The value being aggregated.
    Value,
    /// An expression with one value per aggregated record.
    Row(SqlExpr),
    /// An expression that aggregates the records being aggregated, e.g. `min(created_at)`.
    Aggregated(SqlExpr),
}

pub struct ResolvedSort {
    pub key: SortKey,
    pub direction: SortDirection,
    pub nulls_sort: NullsSort,
}

/// Classifies and compiles the sorting entries attached to an aggregate function.
///
/// The `scope`'s base table is the table from whose context the sorting expressions are evaluated,
/// and `value_path` is the path to the aggregated value, relative to that same table. Comparing
/// each sorting path against `value_path` is how a sort by the aggregated value itself is
/// recognized, even when it's written as a path rather than as `\s`.
pub fn resolve_aggregate_sorting(
    order_by: Vec<AggregateSortExpr>,
    value_path: &[PathPart],
    scope: &mut Scope,
) -> Result<Vec<ResolvedSort>, String> {
    order_by
        .into_iter()
        .map(|entry| {
            let key = match entry.key {
                AggregateSortKey::Value => SortKey::Value,
                AggregateSortKey::Expr(Expr::Path(parts))
                    if paths_resolve_alike(parts.clone(), value_path.to_vec(), scope) =>
                {
                    SortKey::Value
                }
                AggregateSortKey::Expr(expr) => {
                    let aggregated = contains_local_aggregate(&expr);
                    let sql = convert_expr(expr, scope)?;
                    if aggregated {
                        SortKey::Aggregated(sql)
                    } else {
                        SortKey::Row(sql)
                    }
                }
            };
            Ok(ResolvedSort {
                key,
                direction: entry.direction,
                nulls_sort: entry.nulls_sort,
            })
        })
        .collect()
}

/// True when `expr` contains an aggregate function that aggregates the records in the current
/// context (e.g. `created_at%min` or `%count`), as opposed to one that aggregates a related table
/// via a `#` path (e.g. `#reactions%count`), which has one value per record in the current context.
fn contains_local_aggregate(expr: &Expr) -> bool {
    let either = |a: &Expr, b: &Expr| contains_local_aggregate(a) || contains_local_aggregate(b);
    match expr {
        Expr::Call(call) => match call.dimension {
            FunctionDimension::Aggregate => match call.args.first() {
                None => true,
                Some(Expr::Path(parts)) => !parts
                    .iter()
                    .any(|part| matches!(part, PathPart::TableWithMany(_))),
                // Aggregates must be applied to a path. Classifying anything else as aggregated
                // lets compilation report that problem rather than a misleading one about sorting.
                Some(_) => true,
            },
            FunctionDimension::Scalar => call.args.iter().any(contains_local_aggregate),
        },
        Expr::AnonymousFunctionCall(call) => call.args.iter().any(contains_local_aggregate),
        Expr::Product(a, b) | Expr::Quotient(a, b) | Expr::Sum(a, b) | Expr::Difference(a, b) => {
            either(a, b)
        }
        Expr::Not(inner) => contains_local_aggregate(inner),
        Expr::Comparison(comparison) => [&comparison.left, &comparison.right]
            .into_iter()
            .any(|side| matches!(side, ComparisonSide::Expr(e) if contains_local_aggregate(e))),
        Expr::Case(case) => {
            case.variants.iter().any(|v| either(&v.condition, &v.value))
                || contains_local_aggregate(&case.fallback)
        }
        _ => false,
    }
}

/// Renders one `ORDER BY` entry, e.g. `"users"."username" DESC NULLS LAST`.
pub fn render_sort_entry(
    expr: &SqlExpr,
    direction: &SortDirection,
    nulls_sort: &NullsSort,
) -> String {
    let direction = match direction {
        SortDirection::Asc => "ASC",
        SortDirection::Desc => "DESC",
    };
    let nulls_sort = match nulls_sort {
        NullsSort::First => "NULLS FIRST",
        NullsSort::Last => "NULLS LAST",
    };
    format!("{} {} {}", expr, direction, nulls_sort)
}

/// Renders resolved sorting entries as the body of an aggregate's `ORDER BY` clause (empty when
/// there are none), with `value` standing in for each sort by the aggregated value.
pub fn render_sorts(sorts: &[ResolvedSort], value: &SqlExpr) -> String {
    sorts
        .iter()
        .map(|sort| {
            let expr = match &sort.key {
                SortKey::Value => value,
                SortKey::Row(sql) | SortKey::Aggregated(sql) => sql,
            };
            render_sort_entry(expr, &sort.direction, &sort.nulls_sort)
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// How an aggregate function should be compiled, given its resolved sorting entries.
pub enum AggregateKind {
    /// An ordinary aggregate, compiled by wrapping the aggregated value and the rendered `ORDER BY`
    /// clause. Aggregated sort keys aren't meaningful here.
    Wrapped(super::paths::AggWrapper),
    /// `list`, which collects distinct values.
    DistinctList,
}

/// The result of compiling an aggregate expression within a single `SELECT`.
pub enum CompiledAggregate {
    Expr(SqlExpr),
    /// A `list` sorted by an aggregated sort key, which can't be expressed as a single aggregate
    /// call. The caller must build a two-level aggregation instead.
    NeedsTwoLevelList,
}

/// Compiles an aggregate of `value`, ordered by `sorts`, within a single `SELECT`.
pub fn compile_aggregate(
    kind: AggregateKind,
    value: SqlExpr,
    sorts: &[ResolvedSort],
    dialect: &dyn crate::Dialect,
) -> Result<CompiledAggregate, String> {
    let has = |predicate: fn(&SortKey) -> bool| sorts.iter().any(|s| predicate(&s.key));
    let has_row = has(|k| matches!(k, SortKey::Row(_)));
    let has_aggregated = has(|k| matches!(k, SortKey::Aggregated(_)));
    match kind {
        AggregateKind::Wrapped(wrapper) => {
            if has_aggregated {
                return Err(msg::aggregated_sort_outside_list());
            }
            let order_by = render_sorts(sorts, &value);
            Ok(CompiledAggregate::Expr(wrapper(value, order_by, dialect)))
        }
        AggregateKind::DistinctList => {
            if has_row {
                return Err(msg::list_sorted_by_row_expr());
            }
            if has_aggregated {
                return Ok(CompiledAggregate::NeedsTwoLevelList);
            }
            let order_by = render_sorts(sorts, &value);
            Ok(CompiledAggregate::Expr(
                crate::sql::expr::build::agg::array_agg_distinct(value, order_by),
            ))
        }
    }
}
