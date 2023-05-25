use itertools::Itertools;

use crate::{
    constants::{CTE_PK_COLUMN_ALIAS, CTE_VALUE_COLUMN_PREFIX},
    dialects::{dialect::Dialect, sql},
    rendering::{JoinTree, Render, RenderingContext, SimpleExpression},
    schema::{
        chain::{Chain, ChainIntersecting},
        links::{FilteredLink, Link, LinkToOne, MultiLink},
        schema::{ChainSearchBase, Table},
    },
    sql_tree::*,
    syntax_tree::*,
};

struct SimpleConditionSet {
    conjunction: Conjunction,
    entries: Vec<SimpleConditionSetEntry>,
}

impl SimpleConditionSet {
    pub fn new(conjunction: Conjunction, entries: Vec<SimpleConditionSetEntry>) -> Self {
        Self {
            conjunction,
            entries,
        }
    }
}

enum SimpleConditionSetEntry {
    SimpleComparison(SimpleComparison),
    SimpleConditionSet(SimpleConditionSet),
}

impl SimpleConditionSetEntry {
    pub fn new_comparison(left: Expression, operator: Operator, right: Expression) -> Self {
        Self::SimpleComparison(SimpleComparison::new(left, operator, right))
    }

    pub fn new_set(conjunction: Conjunction, entries: Vec<SimpleConditionSetEntry>) -> Self {
        Self::SimpleConditionSet(SimpleConditionSet::new(conjunction, entries))
    }
}

struct SimpleComparison {
    left: Expression,
    operator: Operator,
    right: Expression,
}

impl SimpleComparison {
    pub fn new(left: Expression, operator: Operator, right: Expression) -> Self {
        Self {
            left,
            operator,
            right,
        }
    }
}

pub fn convert_condition_set(
    condition_set: &ConditionSet,
    cx: &mut RenderingContext,
) -> SqlConditionSet {
    SqlConditionSet {
        conjunction: condition_set.conjunction,
        entries: condition_set
            .entries
            .iter()
            .map(|entry| convert_condition_set_entry(entry, cx))
            .collect(),
    }
}

fn convert_condition_set_entry(
    condition_set_entry: &ConditionSetEntry,
    cx: &mut RenderingContext,
) -> SqlConditionSetEntry {
    match condition_set_entry {
        ConditionSetEntry::Comparison(comparison) => convert_comparison(comparison, cx),
        ConditionSetEntry::ConditionSet(condition_set) => {
            SqlConditionSetEntry::ConditionSet(convert_condition_set(condition_set, cx))
        }
    }
}

fn convert_comparison(comparison: &Comparison, cx: &mut RenderingContext) -> SqlConditionSetEntry {
    convert_simple_condition_set_entry(&expand_comparison(comparison), cx)
}

fn convert_simple_condition_set_entry(
    entry: &SimpleConditionSetEntry,
    cx: &mut RenderingContext,
) -> SqlConditionSetEntry {
    match entry {
        SimpleConditionSetEntry::SimpleComparison(comparison) => {
            convert_simple_comparison(comparison, cx)
        }
        SimpleConditionSetEntry::SimpleConditionSet(condition_set) => {
            SqlConditionSetEntry::ConditionSet(SqlConditionSet {
                conjunction: condition_set.conjunction,
                entries: condition_set
                    .entries
                    .iter()
                    .map(|entry| convert_simple_condition_set_entry(entry, cx))
                    .collect(),
            })
        }
    }
}

fn convert_simple_comparison(
    s: &SimpleComparison,
    cx: &mut RenderingContext,
) -> SqlConditionSetEntry {
    // When we see that we're comparing an expression equal to zero or greater to zero, then we
    // hand off the conversion to the context because, depending on the expression, the context
    // may choose to handle this condition via a join instead of a condition set entry. In that
    // case we'll receive an empty SqlConditionSet back, and that will get filtered out later on.
    if s.left.is_zero() && s.operator == Operator::Eq {
        return convert_expression_vs_zero(&s.right, ComparisonVsZero::Eq, cx);
    }
    if s.left.is_zero() && s.operator == Operator::Lt {
        return convert_expression_vs_zero(&s.right, ComparisonVsZero::Gt, cx);
    }
    if s.right.is_zero() && s.operator == Operator::Eq {
        return convert_expression_vs_zero(&s.left, ComparisonVsZero::Eq, cx);
    }
    if s.right.is_zero() && s.operator == Operator::Gt {
        return convert_expression_vs_zero(&s.left, ComparisonVsZero::Gt, cx);
    }
    SqlConditionSetEntry::Expression(format!(
        "{} {} {}",
        s.left.render(cx),
        s.operator.render(cx),
        s.right.render(cx)
    ))
}

#[derive(Clone, Copy)]
enum ComparisonVsZero {
    Eq,
    Gt,
}

impl From<ComparisonVsZero> for Operator {
    fn from(cmp: ComparisonVsZero) -> Self {
        match cmp {
            ComparisonVsZero::Eq => Operator::Eq,
            ComparisonVsZero::Gt => Operator::Gt,
        }
    }
}

impl From<ComparisonVsZero> for CtePurpose {
    fn from(cmp: ComparisonVsZero) -> Self {
        match cmp {
            ComparisonVsZero::Eq => CtePurpose::Exclusion,
            ComparisonVsZero::Gt => CtePurpose::Inclusion,
        }
    }
}

fn convert_expression_vs_zero(
    expr: &Expression,
    cmp: ComparisonVsZero,
    cx: &mut RenderingContext,
) -> SqlConditionSetEntry {
    let fallback = |cx: &mut RenderingContext| {
        let rendered_expr = expr.render(cx);
        let op = Operator::from(cmp).render(cx);
        SqlConditionSetEntry::Expression(format!("{} {} {}", rendered_expr, op, 0))
    };
    if expr.compositions.len() > 0 {
        return fallback(cx);
    }
    let Value::Path(path_parts) = &expr.base else { return fallback(cx) };
    let Ok(clarified_path) = clarify_path(path_parts.clone(), cx) else { return fallback(cx) };
    let ClarifiedPathTail::ChainToMany((chain, None)) = clarified_path.tail else {
        return fallback(cx)
    };
    let join_result = cx.join_chain_to_many(&clarified_path.head, chain, None, vec![], cmp.into());
    let Ok(simple_expr) = join_result else { return fallback(cx) };
    match cmp {
        ComparisonVsZero::Eq => {
            // We're confident that `simple_expr` doesn't have any compositions because we
            // checked that `expr` doesn't have any above.
            let rendered_expr = simple_expr.base.render(cx);
            SqlConditionSetEntry::Expression(sql::value_is_null(rendered_expr))
        }
        ComparisonVsZero::Gt => SqlConditionSetEntry::empty(),
    }
}

fn expand_comparison(comparison: &Comparison) -> SimpleConditionSetEntry {
    use ComparisonPart::{Expression as Expr, ExpressionSet as ExprSet};
    let make_comparison = |left: Expression, right: Expression| {
        SimpleConditionSetEntry::new_comparison(left, comparison.operator, right)
    };
    let make_set = SimpleConditionSetEntry::new_set;
    // All the `clone()` calls in here are kind of unfortunate. Cloning an expression is not
    // necessarily cheap because the expression could be quite deep. In theory, we could perform
    // this expansion, after the expression is rendered, in which case we'd be cloning strings
    // instead. Holding references to the objects instead of cloning them would be nice although
    // it seems like that could get messy. We could consider attempting to eliminate these clone
    // calls if we find that this is a performance bottleneck.
    match (&comparison.left, &comparison.right) {
        (Expr(l), Expr(r)) => make_comparison(l.clone(), r.clone()),
        (ExprSet(l), Expr(r)) => make_set(
            l.conjunction,
            l.entries
                .iter()
                .map(|e| make_comparison(e.clone(), r.clone()))
                .collect(),
        ),
        (Expr(l), ExprSet(r)) => make_set(
            r.conjunction,
            r.entries
                .iter()
                .map(|e| make_comparison(l.clone(), e.clone()))
                .collect(),
        ),
        (ExprSet(l), ExprSet(r)) => make_set(
            l.conjunction,
            l.entries
                .iter()
                .map(|l_exp| {
                    make_set(
                        r.conjunction,
                        r.entries
                            .iter()
                            .map(|r_exp| make_comparison(l_exp.clone(), r_exp.clone()))
                            .collect(),
                    )
                })
                .collect(),
        ),
    }
}

pub fn simplify_expression(expr: &Expression, cx: &mut RenderingContext) -> SimpleExpression {
    let expr = expr.clone();
    match expr.base {
        Value::Literal(literal) => SimpleExpression {
            base: literal,
            compositions: expr.compositions,
        },
        // TODO_ERR handle error
        Value::Path(path) => simplify_path_expression(path, expr.compositions, cx).unwrap(),
    }
}

fn simplify_path_expression(
    parts: Vec<PathPart>,
    compositions: Vec<Composition>,
    cx: &mut RenderingContext,
) -> Result<SimpleExpression, String> {
    let clarified_path = clarify_path(parts, cx)?;
    match (clarified_path.head, clarified_path.tail) {
        (None, ClarifiedPathTail::Column(column_name)) => {
            let table_name = cx.get_base_table().name.clone();
            Ok(SimpleExpression {
                base: Literal::TableColumnReference(table_name, column_name),
                compositions,
            })
        }
        (Some(chain_to_one), ClarifiedPathTail::Column(column_name)) => {
            let table_name = cx.join_chain_to_one(&chain_to_one);
            Ok(SimpleExpression {
                base: Literal::TableColumnReference(table_name, column_name),
                compositions,
            })
        }
        (head, ClarifiedPathTail::ChainToMany((chain_to_many, column_name_opt))) => cx
            .join_chain_to_many(
                &head,
                chain_to_many,
                column_name_opt,
                compositions,
                CtePurpose::AggregateValue,
            ),
    }
}

#[derive(Debug)]
struct ClarifiedPath {
    head: Option<Chain<LinkToOne>>,
    tail: ClarifiedPathTail,
}

#[derive(Debug)]
enum ClarifiedPathTail {
    Column(String),
    /// chain, column_name
    ChainToMany((Chain<FilteredLink>, Option<String>)),
}

fn clarify_path(parts: Vec<PathPart>, cx: &RenderingContext) -> Result<ClarifiedPath, String> {
    let linked_path = build_linked_path(parts, cx)?;
    let chain_opt = linked_path.chain;
    let column_name_opt = linked_path.column;
    let Some(chain) = chain_opt else {
        return column_name_opt.map(|column_name| ClarifiedPath {
            head: None,
            tail: ClarifiedPathTail::Column(column_name),
        }).ok_or_else(msg::no_path_parts)
    };
    let mut head: Option<Chain<LinkToOne>> = None;
    let mut chain_to_many_opt: Option<Chain<FilteredLink>> = None;
    for filtered_link in chain {
        if let Some(chain_to_many) = &mut chain_to_many_opt {
            // This unwrap is safe because we know that the chain has already been constructed.
            // We're just re-constructing part of it.
            chain_to_many.try_append(filtered_link).unwrap();
        } else {
            match LinkToOne::try_from(filtered_link) {
                Ok(link_to_one) => {
                    if let Some(chain) = &mut head {
                        // This unwrap is safe because we know that the chain has already been
                        // constructed using FilteredLink links. All we're doing here is
                        // re-constructing it with LinkToOne links.
                        chain.try_append(link_to_one).unwrap();
                    } else {
                        head =
                            Some(Chain::try_new(link_to_one, ChainIntersecting::Allowed).unwrap());
                    }
                }
                Err(generic_link) => {
                    chain_to_many_opt =
                        Some(Chain::try_new(generic_link, ChainIntersecting::Allowed).unwrap());
                }
            }
        }
    }
    let tail = if let Some(chain_to_many) = chain_to_many_opt {
        ClarifiedPathTail::ChainToMany((chain_to_many, column_name_opt))
    } else {
        ClarifiedPathTail::Column(column_name_opt.ok_or_else(msg::no_column_name_or_chain)?)
    };
    Ok(ClarifiedPath { head, tail })
}

#[derive(Debug)]
struct LinkedPath {
    pub chain: Option<Chain<FilteredLink>>,
    pub column: Option<String>,
}

fn build_linked_path(parts: Vec<PathPart>, cx: &RenderingContext) -> Result<LinkedPath, String> {
    let mut current_table_opt: Option<&Table> = Some(cx.get_base_table());
    let mut chain_opt: Option<Chain<FilteredLink>> = None;
    let mut final_column_name: Option<String> = None;
    for part in parts {
        let current_table = current_table_opt.ok_or_else(msg::no_current_table)?;
        match part {
            PathPart::Column(column_name) => {
                let column_id = current_table
                    .column_lookup
                    .get(&column_name)
                    .copied()
                    .ok_or_else(|| msg::col_not_in_table(&column_name, &current_table.name))?;
                if let Some(link) = current_table.forward_links_to_one.get(&column_id).copied() {
                    current_table_opt = cx.schema.tables.get(&link.get_end().table_id);
                    let link = FilteredLink {
                        link: MultiLink::ForwardLinkToOne(link),
                        condition_set: ConditionSet::default(),
                    };
                    chain_opt = match chain_opt {
                        Some(mut chain) => {
                            chain.try_append(link)?;
                            Some(chain)
                        }
                        None => Some(Chain::try_new(link, ChainIntersecting::Allowed)?),
                    };
                } else {
                    current_table_opt = None;
                    final_column_name = Some(column_name);
                }
            }
            PathPart::TableWithOne(table_name) => {
                todo!()
            }
            PathPart::TableWithMany(mut table_with_many) => {
                let base = ChainSearchBase::TableId(current_table.id);
                let condition_set = std::mem::take(&mut table_with_many.condition_set);
                let mut new_chain =
                    cx.schema
                        .get_chain_to_table_with_many(base, &table_with_many, None)?;
                new_chain.set_final_condition_set(condition_set);
                new_chain.allow_intersecting();
                current_table_opt = cx.schema.tables.get(&new_chain.get_ending_table_id());
                chain_opt = match chain_opt {
                    Some(mut chain) => {
                        chain.try_connect(new_chain)?;
                        Some(chain)
                    }
                    None => Some(new_chain),
                };
                final_column_name = None;
            }
        };
    }
    Ok(LinkedPath {
        chain: chain_opt,
        column: final_column_name,
    })
}

pub fn convert_join_tree(mut tree: JoinTree, cx: &RenderingContext) -> (Vec<Join>, Vec<Cte>) {
    let mut ctes = tree.take_ctes();
    let mut joins: Vec<Join> = ctes
        .iter()
        .map(|cte| build_join_for_cte(cte, tree.get_alias().to_owned(), cx))
        .collect();
    for (link, subtree) in tree.take_dependents() {
        let starting_alias = tree.get_alias();
        let ending_alias = subtree.get_alias();
        let join_type = JoinType::LeftOuter;
        let join = make_join_from_link(&link, starting_alias, ending_alias, join_type, cx);
        joins.push(join);
        let (new_joins, new_ctes) = convert_join_tree(subtree, cx);
        joins.extend(new_joins);
        ctes.extend(new_ctes);
    }
    (joins, ctes)
}

fn build_join_for_cte(cte: &Cte, table: String, cx: &RenderingContext) -> Join {
    let condition = format!(
        "{} = {}",
        cx.dialect.table_column(&table, &cte.join_column_name),
        cx.dialect.table_column(&cte.alias, CTE_PK_COLUMN_ALIAS),
    );
    let join_type = match cte.purpose {
        CtePurpose::Inclusion => JoinType::Inner,
        CtePurpose::Exclusion => JoinType::LeftOuter,
        CtePurpose::AggregateValue => JoinType::LeftOuter,
    };
    Join {
        table: cte.alias.clone(),
        alias: cte.alias.clone(),
        condition_set: SqlConditionSet {
            conjunction: Conjunction::And,
            entries: vec![SqlConditionSetEntry::Expression(condition)],
        },
        join_type,
    }
}

pub struct ValueViaCte {
    pub select: Select,
    pub value_alias: String,
    pub compositions: Vec<Composition>,
}

pub fn build_cte_select(
    chain: Chain<FilteredLink>,
    final_column_name: Option<String>,
    compositions: Vec<Composition>,
    parent_cx: &RenderingContext,
    purpose: CtePurpose,
) -> Result<ValueViaCte, String> {
    use Literal::TableColumnReference;
    let schema = parent_cx.schema;
    let mut links_iter = chain.into_iter();
    let first_link = links_iter.next().unwrap();
    let end = first_link.get_end();
    let base_table = schema.tables.get(&end.table_id).unwrap();
    let base_column = base_table.columns.get(&end.column_id).unwrap();
    let mut cte_cx = parent_cx.spawn(&base_table);
    let mut select = Select::from(cte_cx.get_base_table().name.clone());
    let pk_expr =
        TableColumnReference(base_table.name.clone(), base_column.name.clone()).render(&mut cte_cx);
    select.grouping.push(pk_expr.clone());
    let pr_expr_col = Column::new(pk_expr, Some(CTE_PK_COLUMN_ALIAS.to_owned()));
    select.columns.push(pr_expr_col);
    select.condition_set = convert_condition_set(&first_link.condition_set, &mut cte_cx);
    let mut starting_alias = base_table.name.clone();
    let mut ending_table = schema.tables.get(&first_link.get_end().table_id).unwrap();
    for link in links_iter {
        ending_table = schema.tables.get(&link.get_end().table_id).unwrap();
        let ideal_ending_alias = ending_table.name.as_str();
        let ending_alias = cte_cx.get_alias(ideal_ending_alias);
        let join_type = JoinType::Inner;
        if !link.condition_set.is_empty() {
            let link_table = schema.tables.get(&link.get_end().table_id).unwrap();
            let mut link_cx = cte_cx.spawn(&link_table);
            let converted = convert_condition_set(&link.condition_set, &mut link_cx);
            select.condition_set.merge(converted);
        }
        let join = make_join_from_link(&link, &starting_alias, &ending_alias, join_type, &cte_cx);
        select.joins.push(join);
        starting_alias = ending_alias;
    }
    let (aggregating_compositions, post_aggregate_compositions) =
        prepare_compositions_for_aggregation(compositions)?;

    if purpose == CtePurpose::AggregateValue {
        let value_expr = match final_column_name {
            Some(column_name) => {
                let column_id = ending_table
                    .column_lookup
                    .get(&column_name)
                    .ok_or_else(|| msg::col_not_in_table(&column_name, &ending_table.name))?;
                let column = ending_table.columns.get(column_id).unwrap();
                let expr = Expression {
                    base: Value::Literal(TableColumnReference(
                        ending_table.name.clone(),
                        column.name.clone(),
                    )),
                    compositions: aggregating_compositions,
                };
                expr.render(&mut cte_cx)
            }
            None => {
                let singular_composition = aggregating_compositions
                    .into_iter()
                    .exactly_one()
                    .map_err(|_| msg::pre_aggregate_composition_without_column())?;
                let function_name = singular_composition.function.name;
                if function_name != "count" {
                    return Err(msg::special_aggregate_composition_applied_without_column(
                        function_name,
                    ));
                }
                sql::COUNT_STAR.to_owned()
            }
        };
        let value_alias = format!("{}{}", CTE_VALUE_COLUMN_PREFIX.to_owned(), 1);
        select
            .columns
            .push(Column::new(value_expr, Some(value_alias.clone())));
        return Ok(ValueViaCte {
            select,
            value_alias,
            compositions: post_aggregate_compositions,
        });
    }
    Ok(ValueViaCte {
        select,
        value_alias: CTE_PK_COLUMN_ALIAS.to_owned(),
        compositions: post_aggregate_compositions,
    })
}

/// Returns a tuple of `(aggregating_compositions, post_aggregate_compositions)` where:
///
/// - `aggregating_compositions` are the compositions that should be applied within the CTE. This
///  vec is guaranteed to have at least one composition, with the last composition always being
///  the only aggregate composition.
///
/// - `post_aggregate_compositions` are the compositions that should be applied after the CTE.
/// This vec might be empty. It will not contain any aggregate compositions.
fn prepare_compositions_for_aggregation(
    compositions: Vec<Composition>,
) -> Result<(Vec<Composition>, Vec<Composition>), String> {
    let mut pre_aggregate_compositions = vec![];
    let mut aggregate_composition = None;
    let mut post_aggregate_compositions = vec![];
    for composition in compositions {
        if composition.function.dimension == FunctionDimension::Aggregate {
            if aggregate_composition.is_some() {
                return Err(msg::multiple_agg_fns());
            }
            aggregate_composition = Some(composition);
        } else if aggregate_composition.is_none() {
            pre_aggregate_compositions.push(composition);
        } else {
            post_aggregate_compositions.push(composition);
        }
    }
    match aggregate_composition {
        Some(a) => {
            pre_aggregate_compositions.push(a);
            Ok((pre_aggregate_compositions, post_aggregate_compositions))
        }
        None => Ok((vec![Composition::count()], pre_aggregate_compositions)),
    }
}

fn make_join_from_link(
    link: &impl Link,
    starting_alias: &str,
    ending_alias: &str,
    join_type: JoinType,
    cx: &RenderingContext,
) -> Join {
    let start = link.get_start();
    let starting_table_id = start.table_id;
    let starting_table = cx.schema.tables.get(&starting_table_id).unwrap();
    let starting_column_id = start.column_id;
    let starting_column = starting_table.columns.get(&starting_column_id).unwrap();

    let end = link.get_end();
    let ending_table_id = end.table_id;
    let ending_table = cx.schema.tables.get(&ending_table_id).unwrap();
    let ending_column_id = end.column_id;
    let ending_column = ending_table.columns.get(&ending_column_id).unwrap();

    let condition = format!(
        "{} = {}",
        cx.dialect
            .table_column(starting_alias, &starting_column.name),
        cx.dialect.table_column(ending_alias, &ending_column.name),
    );
    Join {
        table: cx.schema.tables.get(&ending_table_id).unwrap().name.clone(),
        alias: ending_alias.to_owned(),
        condition_set: SqlConditionSet {
            conjunction: Conjunction::And,
            entries: vec![SqlConditionSetEntry::Expression(condition)],
        },
        join_type,
    }
}

/// Error messages
mod msg {
    pub fn no_current_table() -> String {
        "Non-FK columns can only appear at the end of a path.".to_string()
    }

    pub fn col_not_in_table(column_name: &str, table_name: &str) -> String {
        format!("Column `{column_name}` not found within table `{table_name}`.")
    }

    pub fn no_path_parts() -> String {
        "Cannot build a ClarifiedPath without any path parts".to_string()
    }

    pub fn no_column_name_or_chain() -> String {
        "Cannot build a ClarifiedPathTail without a column name or chain".to_string()
    }

    pub fn multiple_agg_fns() -> String {
        "Cannot apply more than one aggregate function to the same expression.".to_string()
    }

    pub fn pre_aggregate_composition_without_column() -> String {
        "Functions can only be applied before aggregation when a column is specified.".to_string()
    }

    pub fn special_aggregate_composition_applied_without_column(function_name: String) -> String {
        format!(
            "Aggregate function `{}` can only be applied to a column.",
            function_name
        )
    }
}