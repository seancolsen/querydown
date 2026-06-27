use querydown_parser::ast::{NullsSort, SortDirection, SortExpr};

use crate::{
    compiler::{
        constants::{CTE_PK_COLUMN_ALIAS, CTE_VALUE_COLUMN_PREFIX},
        expr::{convert_condition_set, convert_expr},
        join_tree::make_join_from_link,
        scope::Scope,
    },
    errors::msg,
    schema::{
        chain::Chain,
        links::{FilteredLink, Link},
        TableId,
    },
    sql::expr::build,
    sql::{
        expr::{build::cmp, SqlExpr},
        tree::{Column, CtePurpose, JoinType, Select},
    },
    Dialect,
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
    /// This is a function that accepts a table.column expression and a pre-rendered ORDER BY
    /// clause (empty string when no ORDER BY was specified) and returns a wrapped expression
    /// that is used as the aggregate expression. E.g. it might produce:
    ///
    /// `array_agg("col" ORDER BY "col" ASC NULLS LAST)`
    ///
    /// When this AggregateExprTemplate instance is rendered within a CTE, the column_name is
    /// resolved to a table.column expression, and then the agg_wrapper is applied to that
    /// expression along with the compiled ORDER BY string.
    agg_wrapper: AggWrapper,
    order_by: Vec<SortExpr>,
    /// True when this aggregate produces a count (e.g. `count` or `distinct`), so that its value is
    /// coalesced to zero in the outer query.
    coalesce_to_zero: bool,
    /// The table from whose context the `order_by` expressions are evaluated: the table named by the
    /// first `#` part of the aggregated path. This lets a sort reference a column of the join table
    /// (e.g. `#issue_labels.label.name%list(\\id)` sorts by `issue_labels.id`), rather than only
    /// columns of the path's final table.
    order_by_anchor_table_id: TableId,
}

impl AggregateExprTemplate {
    pub fn new(
        column_name: String,
        agg_wrapper: AggWrapper,
        order_by: Vec<SortExpr>,
        coalesce_to_zero: bool,
        order_by_anchor_table_id: TableId,
    ) -> Self {
        Self {
            column_name,
            agg_wrapper,
            order_by,
            coalesce_to_zero,
            order_by_anchor_table_id,
        }
    }
}

pub fn render_order_by(order_by: Vec<SortExpr>, scope: &mut Scope) -> Result<String, String> {
    if order_by.is_empty() {
        return Ok(String::new());
    }
    let mut parts = Vec::with_capacity(order_by.len());
    for sort_expr in order_by {
        let sql_expr = convert_expr(sort_expr.expr, scope)?;
        let direction = match sort_expr.direction {
            SortDirection::Asc => "ASC",
            SortDirection::Desc => "DESC",
        };
        let nulls_sort = match sort_expr.nulls_sort {
            NullsSort::First => "NULLS FIRST",
            NullsSort::Last => "NULLS LAST",
        };
        parts.push(format!("{} {} {}", sql_expr, direction, nulls_sort));
    }
    Ok(parts.join(", "))
}

pub fn build_cte_select(
    chain: Chain<FilteredLink>,
    aggregate_expr_template_opt: Option<AggregateExprTemplate>,
    parent_scope: &Scope,
    purpose: CtePurpose,
) -> Result<ValueViaCte, String> {
    let schema = parent_scope.schema;
    let mut links_iter = chain.into_iter();
    let first_link = links_iter.next().unwrap();
    let end = first_link.get_end();
    let base_table = schema.tables.get(&end.table_id).unwrap();
    let base_column = base_table.columns.get(&end.column_id).unwrap();
    let mut cte_scope = parent_scope.spawn(base_table);
    let mut select = Select::from(cte_scope.get_base_table().name.clone());
    let pk_expr = cte_scope.table_column_expr(&base_table.name, &base_column.name);
    select.grouping.push(pk_expr.clone());
    let pr_expr_col = Column::new(pk_expr, Some(CTE_PK_COLUMN_ALIAS.to_owned()));
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
        if !link.condition_set.is_empty() {
            let link_table = schema.tables.get(&link.get_end().table_id).unwrap();
            let mut link_scope = cte_scope.spawn(link_table);
            let converted = convert_condition_set(link.condition_set, &mut link_scope)?;
            select.conditions = cmp::and([select.conditions, converted]);
        }
        let join = make_join_from_link(
            link_start,
            &starting_alias,
            link_end,
            &ending_alias,
            join_type,
            &cte_scope,
        );
        select.joins.push(join);
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
                let column_name = template.column_name;
                let column_id = cte_scope
                    .options
                    .resolve_identifier(&ending_table.column_lookup, &column_name)
                    .ok_or_else(|| msg::col_not_in_table(&column_name, &ending_table.name))?;
                let column = ending_table.columns.get(column_id).unwrap();
                let reference = cte_scope.table_column_expr(&ending_table.name, &column.name);
                // Sorting conditions are evaluated from the context of the table named by the first
                // `#` part of the path (the anchor), not the path's final table. The anchor is
                // already joined into this CTE (it lies along the chain), so we share the CTE's
                // existing aliases to avoid a duplicate-alias collision when a sort reaches a table
                // the CTE already joined.
                let order_by_str = {
                    let anchor_table = schema
                        .tables
                        .get(&template.order_by_anchor_table_id)
                        .unwrap();
                    let mut ob_scope =
                        cte_scope.spawn_with_aliases(anchor_table, cte_scope.cloned_aliases());
                    let s = render_order_by(template.order_by, &mut ob_scope)?;
                    let (ob_joins, ob_ctes) = ob_scope.decompose_join_tree();
                    select.joins.extend(ob_joins);
                    select.ctes.extend(ob_ctes);
                    s
                };
                let wrapper = template.agg_wrapper;
                wrapper(reference, order_by_str, cte_scope.options.dialect.as_ref())
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
