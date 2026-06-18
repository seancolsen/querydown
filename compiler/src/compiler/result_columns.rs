use std::collections::{HashMap, HashSet};

use itertools::Itertools;
use querydown_parser::ast::*;

use crate::{
    compiler::paths::{clarify_path, ClarifiedPathTail},
    errors::msg,
    sql::{
        expr::SqlExpr,
        tree::{Column, SortEntry},
    },
};

use self::grouping::GroupingStack;
use self::sorting::SortingStack;

use super::{expr::convert_expr, scope::Scope};

/// The result of converting result columns: the SQL columns, the sorting, the grouping, and a
/// `column_annotations` vector that is positionally aligned with `columns` (one entry per emitted
/// column, `None` where the column has no annotation).
pub struct ConvertedResultColumns {
    pub columns: Vec<Column>,
    pub sorting: Vec<SortEntry>,
    pub grouping: Vec<SqlExpr>,
    pub column_annotations: Vec<Option<AnnotationValue>>,
}

/// Determines whether an expression contains an aggregate function (or otherwise aggregates data).
/// Used to validate that, in a grouped query, every result column is either a grouping column or an
/// aggregate. A path that references many records (e.g. `#issues`) aggregates by default, as does a
/// "has quantity" expression (e.g. `++#issues`).
fn contains_aggregate(expr: &Expr) -> bool {
    match expr {
        Expr::Call(call) => {
            matches!(call.dimension, FunctionDimension::Aggregate)
                || call.args.iter().any(contains_aggregate)
        }
        Expr::Path(parts) => parts
            .iter()
            .any(|part| matches!(part, PathPart::TableWithMany(_))),
        Expr::HasQuantity(_) => true,
        Expr::Product(a, b) | Expr::Quotient(a, b) | Expr::Sum(a, b) | Expr::Difference(a, b) => {
            contains_aggregate(a) || contains_aggregate(b)
        }
        Expr::Not(e) => contains_aggregate(e),
        Expr::ConditionSet(cs) => cs.entries.iter().any(contains_aggregate),
        Expr::Comparison(c) => {
            comparison_side_contains_aggregate(&c.left)
                || comparison_side_contains_aggregate(&c.right)
        }
        Expr::Number(_)
        | Expr::Date(_)
        | Expr::Duration(_)
        | Expr::String(_)
        | Expr::Variable(_) => false,
    }
}

fn comparison_side_contains_aggregate(side: &ComparisonSide) -> bool {
    match side {
        ComparisonSide::Expr(e) => contains_aggregate(e),
        ComparisonSide::Expansion(cs) => cs.entries.iter().any(contains_aggregate),
        ComparisonSide::Range(r) => {
            contains_aggregate(&r.lower.expr) || contains_aggregate(&r.upper.expr)
        }
    }
}

/// Converts standalone `\\` sorting expressions into SQL sort entries, preserving the order in
/// which they were listed (no ordinal handling — listed order is the precedence). Each expression
/// is compiled directly; unlike column sorts, there is no result-column alias to resolve, since
/// these are defined before the result columns.
pub fn convert_sort_exprs(
    sorting: Vec<SortExpr>,
    scope: &mut Scope,
) -> Result<Vec<SortEntry>, String> {
    sorting
        .into_iter()
        .map(|sort_expr| {
            Ok(SortEntry {
                expr: convert_expr(sort_expr.expr, scope)?,
                direction: sort_expr.direction,
                nulls_sort: sort_expr.nulls_sort,
            })
        })
        .collect()
}

pub fn convert_result_columns(
    result_columns: Vec<ResultColumnStatement>,
    scope: &mut Scope,
) -> Result<ConvertedResultColumns, String> {
    let mut columns = Vec::<Column>::new();
    let mut column_annotations = Vec::<Option<AnnotationValue>>::new();
    let mut sorting_stack = SortingStack::new();
    let mut grouping_stack = GroupingStack::new();
    // Whether the query emits a result column that is neither a grouping column nor an aggregate.
    // Such a column is only valid when the query has no grouping at all.
    let mut has_ungrouped_unaggregated = false;
    for column_statement in result_columns {
        match column_statement {
            ResultColumnStatement::Spec(spec) => {
                handle_spec(
                    spec,
                    &mut columns,
                    &mut column_annotations,
                    &mut sorting_stack,
                    &mut grouping_stack,
                    &mut has_ungrouped_unaggregated,
                    scope,
                )?;
            }
            ResultColumnStatement::Glob(glob) => {
                handle_glob(
                    glob,
                    &mut columns,
                    &mut column_annotations,
                    &mut sorting_stack,
                    &mut grouping_stack,
                    &mut has_ungrouped_unaggregated,
                    scope,
                )?;
            }
        }
    }
    let grouping: Vec<SqlExpr> = grouping_stack.into();
    if !grouping.is_empty() && has_ungrouped_unaggregated {
        return Err(msg::ungrouped_unaggregated_column());
    }
    Ok(ConvertedResultColumns {
        columns,
        sorting: sorting_stack.into(),
        grouping,
        column_annotations,
    })
}

#[allow(clippy::too_many_arguments)]
fn handle_spec(
    spec: ColumnSpec,
    columns: &mut Vec<Column>,
    column_annotations: &mut Vec<Option<AnnotationValue>>,
    sorting_stack: &mut SortingStack,
    grouping_stack: &mut GroupingStack,
    has_ungrouped_unaggregated: &mut bool,
    scope: &mut Scope,
) -> Result<(), String> {
    let ColumnControl {
        sort,
        group,
        is_partition_by: _,
        is_hidden: _,
    } = spec.column_control;
    let is_grouped = group.is_some();
    let is_aggregated = contains_aggregate(&spec.expr);
    let expr = convert_expr(spec.expr, scope)?;
    let alias = spec.alias;
    if let Some(sort_spec) = sort {
        let sorting_expr = alias
            .as_ref()
            .map(|a| SqlExpr::atom(scope.options.dialect.quote_identifier(a)))
            .unwrap_or_else(|| expr.clone());
        sorting_stack.push(sorting_expr, sort_spec);
    }
    if let Some(group_spec) = group {
        grouping_stack.push(expr.clone(), group_spec);
    }
    if !is_grouped && !is_aggregated {
        *has_ungrouped_unaggregated = true;
    }
    columns.push(Column { expr, alias });
    column_annotations.push(spec.annotation);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn handle_glob(
    glob: ColumnGlob,
    columns: &mut Vec<Column>,
    column_annotations: &mut Vec<Option<AnnotationValue>>,
    sorting_stack: &mut SortingStack,
    grouping_stack: &mut GroupingStack,
    has_ungrouped_unaggregated: &mut bool,
    scope: &mut Scope,
) -> Result<(), String> {
    scope.with_path_prefix(glob.head.clone(), |scope| -> Result<(), String> {
        for spec in glob.specs.iter() {
            if let Some(ref sort_spec) = spec.column_control.sort {
                let sql_expr = convert_expr(spec.expr.clone(), scope)?;
                sorting_stack.push(sql_expr, sort_spec.to_owned());
            }
            if let Some(ref group_spec) = spec.column_control.group {
                let sql_expr = convert_expr(spec.expr.clone(), scope)?;
                grouping_stack.push(sql_expr, group_spec.to_owned());
            }
        }
        Ok(())
    })?;

    let (table, table_alias) = if glob.head.is_empty() {
        let base_table = scope.get_base_table();
        (scope.get_base_table(), base_table.name.clone())
    } else {
        let clarified_path = clarify_path(glob.head, scope)?;
        if let Some(tail) = clarified_path.tail {
            let err_msg = match tail {
                ClarifiedPathTail::Column(col) => msg::column_glob_after_non_fk_column(&col),
                ClarifiedPathTail::ChainToMany(_) => msg::column_glob_on_path_to_many(),
            };
            return Err(err_msg);
        }
        let Some(chain_to_one) = clarified_path.head else {
            return Err(msg::empty_path());
        };
        let table = scope
            .schema
            .tables
            .get(&chain_to_one.get_ending_table_id())
            .unwrap();
        let table_alias = scope.join_chain_to_one(&chain_to_one);
        (table, table_alias)
    };

    let mut hidden_columns: HashSet<usize> = HashSet::new();
    let mut grouped_columns: HashSet<usize> = HashSet::new();
    let mut column_aliases: HashMap<usize, String> = HashMap::new();
    let mut column_annotations_map: HashMap<usize, AnnotationValue> = HashMap::new();

    for spec in glob.specs {
        if let Expr::Path(ref path) = spec.expr {
            if let Ok(PathPart::Column(column_name)) = path.iter().exactly_one() {
                let column_id = scope
                    .options
                    .resolve_identifier(&table.column_lookup, column_name)
                    .copied()
                    .ok_or_else(|| msg::col_not_in_table(column_name, &table.name))?;
                if spec.column_control.is_hidden {
                    hidden_columns.insert(column_id);
                }
                if spec.column_control.group.is_some() {
                    grouped_columns.insert(column_id);
                }
                if let Some(alias) = spec.alias {
                    column_aliases.insert(column_id, alias);
                }
                if let Some(annotation) = spec.annotation {
                    column_annotations_map.insert(column_id, annotation);
                }
            }
        }
    }

    for column in table.columns.values().sorted_by_key(|c| c.id) {
        let expr = scope.table_column_expr(&table_alias, &column.name);
        let alias = column_aliases.get(&column.id).cloned();
        if !hidden_columns.contains(&column.id) {
            // A glob emits raw table columns, which are not aggregates. In a grouped query each
            // such column must therefore be one of the grouping columns.
            if !grouped_columns.contains(&column.id) {
                *has_ungrouped_unaggregated = true;
            }
            columns.push(Column { expr, alias });
            column_annotations.push(column_annotations_map.get(&column.id).cloned());
        }
    }
    Ok(())
}

mod sorting {
    use querydown_parser::ast::SortSpec;

    use crate::sql::tree::{SortEntry, SqlExpr};

    pub struct UnplacedSortEntry {
        entry: SortEntry,
        ordinal: Option<u32>,
    }

    pub struct SortingStack {
        entries: Vec<UnplacedSortEntry>,
    }

    impl Default for SortingStack {
        fn default() -> Self {
            Self::new()
        }
    }

    impl SortingStack {
        pub fn new() -> Self {
            Self {
                entries: Vec::new(),
            }
        }

        pub fn push(&mut self, expr: SqlExpr, sort_spec: SortSpec) {
            let entry = UnplacedSortEntry {
                entry: SortEntry {
                    expr,
                    direction: sort_spec.direction,
                    nulls_sort: sort_spec.nulls_sort,
                },
                ordinal: sort_spec.ordinal,
            };
            self.entries.push(entry);
        }
    }

    impl From<SortingStack> for Vec<SortEntry> {
        fn from(stack: SortingStack) -> Self {
            let mut entries = stack.entries;
            let max_ordinal = entries.iter().filter_map(|e| e.ordinal).max().unwrap_or(0);
            entries.sort_by_key(|entry| entry.ordinal.unwrap_or(max_ordinal.saturating_add(1)));
            entries.into_iter().map(|entry| entry.entry).collect()
        }
    }
}

mod grouping {
    use querydown_parser::ast::GroupSpec;

    use crate::sql::tree::SqlExpr;

    struct UnplacedGroupEntry {
        expr: SqlExpr,
        ordinal: Option<u32>,
    }

    /// Collects the `GROUP BY` expressions, ordering them by their ordinals (`\g1`, `\g2`, …).
    /// Entries without an explicit ordinal keep the order in which they appear, after all entries
    /// that do specify an ordinal — mirroring [`super::sorting::SortingStack`].
    pub struct GroupingStack {
        entries: Vec<UnplacedGroupEntry>,
    }

    impl Default for GroupingStack {
        fn default() -> Self {
            Self::new()
        }
    }

    impl GroupingStack {
        pub fn new() -> Self {
            Self {
                entries: Vec::new(),
            }
        }

        pub fn push(&mut self, expr: SqlExpr, group_spec: GroupSpec) {
            self.entries.push(UnplacedGroupEntry {
                expr,
                ordinal: group_spec.ordinal,
            });
        }
    }

    impl From<GroupingStack> for Vec<SqlExpr> {
        fn from(stack: GroupingStack) -> Self {
            let mut entries = stack.entries;
            let max_ordinal = entries.iter().filter_map(|e| e.ordinal).max().unwrap_or(0);
            entries.sort_by_key(|entry| entry.ordinal.unwrap_or(max_ordinal.saturating_add(1)));
            entries.into_iter().map(|entry| entry.expr).collect()
        }
    }
}
