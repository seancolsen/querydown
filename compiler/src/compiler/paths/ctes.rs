use querydown_parser::ast::{AggregateSortExpr, NullsSort, PathPart, SortDirection, SortExpr};

use crate::{
    compiler::{
        aggregate_sorting::{
            compile_aggregate, render_sort_entry, resolve_aggregate_sorting, AggregateKind,
            CompiledAggregate, ListMode, ResolvedSort, SortKey,
        },
        constants::{
            CTE_PK_COLUMN_ALIAS, CTE_VALUE_COLUMN_PREFIX, DISTINCT_VALUES_CTE_ALIAS,
            DISTINCT_VALUES_RANK_COLUMN_ALIAS, DISTINCT_VALUES_SORT_COLUMN_PREFIX,
            DISTINCT_VALUE_COLUMN_ALIAS,
        },
        expr::{convert_condition_set, convert_expr},
        join_tree::make_join_from_link,
        scope::{Expansion, Scope, WindowBan},
    },
    errors::msg,
    schema::{
        chain::Chain,
        links::{FilteredLink, Link},
        Table, TableId,
    },
    sql::expr::build,
    sql::{
        expr::{build::cmp, SqlExpr},
        tree::{Column, Cte, CtePurpose, JoinType, Select},
    },
    Dialect, Error,
};

/// Wraps a resolved `table.column` expression and a pre-rendered `ORDER BY` clause into an
/// aggregate expression. The dialect is supplied because a few aggregates (e.g. `product`) compile
/// differently per dialect.
pub type AggWrapper = fn(SqlExpr, String, &dyn Dialect) -> SqlExpr;

pub struct ValueViaCte {
    pub select: Select,
    pub value_alias: String,
    /// True when the CTE's value is a count, which should be coalesced to zero in the outer query.
    /// Because the CTE is joined via a LEFT JOIN, base-table rows with no related records produce a
    /// NULL value. For counts we want those rows to show `0` instead.
    pub coalesce_to_zero: bool,
}

pub struct AggregateExprTemplate {
    column_name: String,
    /// How to compile the aggregate (see [`AggregateKind`]). When this AggregateExprTemplate
    /// instance is rendered within a CTE, the column_name is resolved to a table.column expression,
    /// and then the aggregate is compiled from that expression along with the resolved `order_by`
    /// entries, producing e.g.:
    ///
    /// `array_agg(DISTINCT "col" ORDER BY "col" ASC NULLS LAST)`
    kind: AggregateKind,
    order_by: Vec<AggregateSortExpr>,
    /// True when this aggregate produces a count (e.g. `count` or `distinct`), so that its value is
    /// coalesced to zero in the outer query.
    coalesce_to_zero: bool,
    /// The table from whose context the `order_by` expressions are evaluated: the table named by the
    /// first `#` part of the aggregated path. This lets a sort reference a column of the join table
    /// (e.g. `#issue_labels.label.name%list(\\id)` sorts by `issue_labels.id`), rather than only
    /// columns of the path's final table.
    order_by_anchor_table_id: TableId,
    /// The aggregated path relative to the anchor table (the parts following the first `#` part),
    /// used to recognize a sorting expression that sorts by the aggregated value itself.
    value_path: Vec<PathPart>,
}

impl AggregateExprTemplate {
    pub fn new(
        column_name: String,
        kind: AggregateKind,
        order_by: Vec<AggregateSortExpr>,
        coalesce_to_zero: bool,
        order_by_anchor_table_id: TableId,
        value_path: Vec<PathPart>,
    ) -> Self {
        Self {
            column_name,
            kind,
            order_by,
            coalesce_to_zero,
            order_by_anchor_table_id,
            value_path,
        }
    }
}

pub fn render_order_by(order_by: Vec<SortExpr>, scope: &mut Scope) -> Result<String, Error> {
    let mut parts = Vec::with_capacity(order_by.len());
    for sort_expr in order_by {
        let sql_expr = convert_expr(sort_expr.expr, scope)?;
        parts.push(render_sort_entry(
            &sql_expr,
            &sort_expr.direction,
            &sort_expr.nulls_sort,
        ));
    }
    Ok(parts.join(", "))
}

pub fn build_cte_select(
    chain: Chain<FilteredLink>,
    aggregate_expr_template_opt: Option<AggregateExprTemplate>,
    parent_scope: &Scope,
    purpose: CtePurpose,
) -> Result<ValueViaCte, Error> {
    let schema = parent_scope.schema;
    let mut links_iter = chain.into_iter();
    let first_link = links_iter.next().unwrap();
    let end = first_link.get_end();
    let base_table = schema.tables.get(&end.table_id).unwrap();
    let base_column = base_table.columns.get(&end.column_id).unwrap();
    let mut cte_scope = parent_scope.spawn(base_table);
    // Everything compiled for this CTE lands in its `WHERE` clause or within an aggregate.
    cte_scope.ban_windows(WindowBan::RelatedRecords);
    let mut select = Select::from(cte_scope.get_base_table().name.clone());
    let pk_expr = cte_scope.table_column_expr(&base_table.name, &base_column.name);
    select.grouping.push(pk_expr.clone());
    let pr_expr_col = Column::new(pk_expr.clone(), Some(CTE_PK_COLUMN_ALIAS.to_owned()));
    select.columns.push(pr_expr_col);
    select.conditions = convert_condition_set(first_link.condition_set.clone(), &mut cte_scope)?;
    let mut starting_alias = base_table.name.clone();
    let mut ending_table = schema.tables.get(&first_link.get_end().table_id).unwrap();
    for link in links_iter {
        ending_table = schema.tables.get(&link.get_end().table_id).unwrap();
        let ideal_ending_alias = ending_table.name.as_str();
        let ending_alias = cte_scope.get_alias(ideal_ending_alias);
        let join_type = JoinType::Inner;
        let link_start = link.get_start();
        let link_end = link.get_end();
        let join = make_join_from_link(
            link_start,
            &starting_alias,
            link_end,
            &ending_alias,
            join_type,
            &cte_scope,
        );
        select.joins.push(join);
        if !link.condition_set.is_empty() {
            let link_table = schema.tables.get(&link.get_end().table_id).unwrap();
            // Seed the filter's scope with the aliases already reserved by the CTE so that any
            // tables or nested CTEs it introduces (e.g. a `{--#comments}` filter that itself
            // aggregates a related table) get distinct aliases. The filter's base is addressed by
            // this link's alias, which is suffixed when the path revisits a table (e.g. the second
            // `issues` in `#issues.project.#issues{…}`).
            let mut link_scope =
                cte_scope.spawn_with_aliases(link_table, &ending_alias, cte_scope.cloned_aliases());
            let converted = convert_condition_set(link.condition_set, &mut link_scope)?;
            select.conditions = cmp::and([select.conditions, converted]);
            // Fold any joins and nested CTEs the filter produced into this CTE's SELECT. Without
            // this, a filter that reaches through another table (a to-one join) or aggregates a
            // related table (a nested CTE) would be silently dropped, leaving the CTE's WHERE
            // clause referencing tables absent from its FROM clause.
            let (link_joins, link_ctes) = link_scope.decompose_join_tree();
            select.joins.extend(link_joins);
            select.ctes.extend(link_ctes);
            // Keep the aliases of those joins taken, so that later joins in this CTE (e.g. one made
            // by an aggregate's sorting) can't reuse them.
            let link_aliases = link_scope.cloned_aliases();
            drop(link_scope);
            cte_scope.reserve_aliases(link_aliases);
        }
        starting_alias = ending_alias;
    }

    // Fold any to-one joins resolved while converting the filter's condition set (e.g. a
    // `patron.first_name` path) into the CTE's own SELECT. Without this, those joins accumulate in
    // the CTE scope's join tree and are silently dropped, producing SQL that references a table
    // absent from the FROM clause.
    let (cte_joins, cte_ctes) = cte_scope.decompose_join_tree();
    select.joins.extend(cte_joins);
    select.ctes.extend(cte_ctes);

    if purpose == CtePurpose::AggregateValue {
        // A bare to-many path with no aggregate template is a plain count of related records.
        let mut coalesce_to_zero = aggregate_expr_template_opt.is_none();
        let value_expr = match aggregate_expr_template_opt {
            Some(template) => {
                coalesce_to_zero = template.coalesce_to_zero;
                // `starting_alias` now holds the alias of the chain's final table.
                let reference = aggregated_value(
                    &template.column_name,
                    ending_table,
                    &starting_alias,
                    &mut select,
                    &mut cte_scope,
                )?;
                // Sorting conditions are evaluated from the context of the table named by the first
                // `#` part of the path (the anchor), not the path's final table. The anchor is
                // already joined into this CTE (it lies along the chain), so we share the CTE's
                // existing aliases to avoid a duplicate-alias collision when a sort reaches a table
                // the CTE already joined. The anchor's alias is its table name: the anchor lies on
                // the first `#` part's chain, which never revisits a table, and the CTE reserves no
                // aliases before that chain's tables.
                let sorts = {
                    let anchor_table = schema
                        .tables
                        .get(&template.order_by_anchor_table_id)
                        .unwrap();
                    let mut ob_scope = cte_scope.spawn_with_aliases(
                        anchor_table,
                        &anchor_table.name,
                        cte_scope.cloned_aliases(),
                    );
                    let sorts = resolve_aggregate_sorting(
                        template.order_by,
                        &template.value_path,
                        &mut ob_scope,
                    )?;
                    let (ob_joins, ob_ctes) = ob_scope.decompose_join_tree();
                    select.joins.extend(ob_joins);
                    select.ctes.extend(ob_ctes);
                    let ob_aliases = ob_scope.cloned_aliases();
                    drop(ob_scope);
                    cte_scope.reserve_aliases(ob_aliases);
                    sorts
                };
                let dialect = cte_scope.options.dialect.as_ref();
                match compile_aggregate(template.kind, reference.clone(), &sorts, dialect)? {
                    CompiledAggregate::Expr(expr) => expr,
                    CompiledAggregate::NeedsTwoLevelList(mode) => {
                        return build_distinct_list_select(
                            select,
                            pk_expr,
                            reference,
                            sorts,
                            mode,
                            &mut cte_scope,
                        );
                    }
                }
            }
            None => build::agg::count_star(),
        };
        let value_alias = format!("{}{}", CTE_VALUE_COLUMN_PREFIX, 1);
        select
            .columns
            .push(Column::new(value_expr, Some(value_alias.clone())));
        return Ok(ValueViaCte {
            select,
            value_alias,
            coalesce_to_zero,
        });
    }
    Ok(ValueViaCte {
        select,
        value_alias: CTE_PK_COLUMN_ALIAS.to_owned(),
        coalesce_to_zero: false,
    })
}

/// Resolves the value an aggregated `#` path's CTE aggregates: the column `column_name` of the path's
/// final table, which the CTE's `select` addresses as `alias`.
///
/// When `column_name` names a computed column, its definition is compiled in the context of that
/// table, as though it were written in a filter on it. Any joins and nested CTEs the definition
/// needs (e.g. for a `#assignments:>0` within it) are folded into `select`.
fn aggregated_value(
    column_name: &str,
    table: &Table,
    alias: &str,
    select: &mut Select,
    cte_scope: &mut Scope,
) -> Result<SqlExpr, Error> {
    if let Some(column_id) = cte_scope
        .options
        .resolve_identifier(&table.column_lookup, column_name)
    {
        let column = table.columns.get(column_id).unwrap();
        return Ok(cte_scope.table_column_expr(alias, &column.name));
    }
    let expr = cte_scope
        .get_computed_column(&table.name, column_name)
        .cloned()
        .ok_or_else(|| msg::col_not_in_table(column_name, &table.name))?;
    let _guard = cte_scope.enter_expansion(Expansion::ComputedColumn {
        table: table.name.clone(),
        column: column_name.to_owned(),
    })?;
    let mut value_scope = cte_scope.spawn_with_aliases(table, alias, cte_scope.cloned_aliases());
    let value = convert_expr(expr, &mut value_scope)?;
    let (joins, ctes) = value_scope.decompose_join_tree();
    select.joins.extend(joins);
    select.ctes.extend(ctes);
    let aliases = value_scope.cloned_aliases();
    drop(value_scope);
    cte_scope.reserve_aliases(aliases);
    Ok(value)
}

/// Builds the two-level aggregation for a `list` sorted by one or more aggregated sort keys (e.g.
/// `#comments.user.username%list(\\created_at%min)`), or for a `top`. SQL can't sort a `DISTINCT`
/// aggregate by anything other than the aggregated value, so instead:
///
/// 1. `inner` — the CTE's usual `SELECT`, grouped by the linking column (`pk`) — is additionally
///    grouped by the value. Each aggregated sort key is computed per `(pk, value)` group.
/// 2. An outer `SELECT` reads those groups (now one row per distinct value) and collects the values
///    with `array_agg`.
///
/// For [`ListMode::All`], the outer `array_agg` is ordered by the precomputed sort keys. For
/// [`ListMode::Top`], the inner `SELECT` instead ranks each group within its `pk` by the sort keys
/// (a window over the grouped rows, so it can use the aggregates directly), and the outer `SELECT`
/// keeps only the groups ranked first. Those all tie on every sort key, so they're collected in the
/// order of the values themselves, which keeps the result deterministic.
fn build_distinct_list_select(
    mut inner: Select,
    pk_expr: SqlExpr,
    value: SqlExpr,
    sorts: Vec<ResolvedSort>,
    mode: ListMode,
    scope: &mut Scope,
) -> Result<ValueViaCte, Error> {
    let inner_alias = scope.get_alias(DISTINCT_VALUES_CTE_ALIAS);
    inner.columns.push(Column::new(
        value.clone(),
        Some(DISTINCT_VALUE_COLUMN_ALIAS.to_owned()),
    ));
    inner.grouping.push(value.clone());
    let column = |name: &str| scope.table_column_expr(&inner_alias, name);
    let mut outer = Select::from(inner_alias.clone());
    let value_column = column(DISTINCT_VALUE_COLUMN_ALIAS);
    let list = match mode {
        ListMode::All => {
            let mut order_by = Vec::with_capacity(sorts.len());
            for (index, sort) in sorts.into_iter().enumerate() {
                let expr = match sort.key {
                    SortKey::Value => value_column.clone(),
                    SortKey::Aggregated(sql) => {
                        let alias = format!("{}{}", DISTINCT_VALUES_SORT_COLUMN_PREFIX, index + 1);
                        inner.columns.push(Column::new(sql, Some(alias.clone())));
                        column(&alias)
                    }
                    // `compile_aggregate` rejects row-level sort keys before requesting a two-level
                    // list.
                    SortKey::Row(_) => return Err(msg::list_sorted_by_row_expr().at(sort.loc)),
                };
                order_by.push(render_sort_entry(&expr, &sort.direction, &sort.nulls_sort));
            }
            build::agg::array_agg(value_column, order_by.join(", "))
        }
        ListMode::Top => {
            let mut order_by = Vec::with_capacity(sorts.len());
            for sort in sorts {
                let expr = match sort.key {
                    SortKey::Value => value.clone(),
                    SortKey::Aggregated(sql) => sql,
                    // `compile_aggregate` rejects row-level sort keys before requesting a two-level
                    // list.
                    SortKey::Row(_) => return Err(msg::top_sorted_by_row_expr().at(sort.loc)),
                };
                order_by.push(render_sort_entry(&expr, &sort.direction, &sort.nulls_sort));
            }
            let rank = SqlExpr::atom(format!(
                "rank() OVER (PARTITION BY {} ORDER BY {})",
                pk_expr,
                order_by.join(", ")
            ));
            inner.columns.push(Column::new(
                rank,
                Some(DISTINCT_VALUES_RANK_COLUMN_ALIAS.to_owned()),
            ));
            outer.conditions = cmp::eq(
                column(DISTINCT_VALUES_RANK_COLUMN_ALIAS),
                SqlExpr::atom("1".to_owned()),
            );
            let order_by = render_sort_entry(&value_column, &SortDirection::Asc, &NullsSort::Last);
            build::agg::array_agg(value_column, order_by)
        }
    };
    let pk = column(CTE_PK_COLUMN_ALIAS);
    outer.ctes.push(Cte {
        alias: inner_alias,
        select: inner,
        join_column_name: String::new(),
    });
    outer.columns.push(Column::new(
        pk.clone(),
        Some(CTE_PK_COLUMN_ALIAS.to_owned()),
    ));
    outer.grouping.push(pk);
    let value_alias = format!("{}{}", CTE_VALUE_COLUMN_PREFIX, 1);
    outer
        .columns
        .push(Column::new(list, Some(value_alias.clone())));
    Ok(ValueViaCte {
        select: outer,
        value_alias,
        coalesce_to_zero: false,
    })
}
