use crate::{
    constants::CTE_PK_COLUMN_ALIAS,
    dialects::dialect::Dialect,
    rendering::{JoinTree, Render, RenderingContext, SimpleExpression},
    schema::{
        chain::{Chain, ChainIntersecting},
        links::{get_fk_column_name, GenericLink, Link, LinkToOne},
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

pub fn convert_condition_set<D: Dialect>(
    condition_set: &ConditionSet,
    cx: &mut RenderingContext<D>,
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

fn convert_condition_set_entry<D: Dialect>(
    condition_set_entry: &ConditionSetEntry,
    cx: &mut RenderingContext<D>,
) -> SqlConditionSetEntry {
    match condition_set_entry {
        ConditionSetEntry::Comparison(comparison) => convert_comparison(comparison, cx),
        ConditionSetEntry::ConditionSet(condition_set) => {
            SqlConditionSetEntry::ConditionSet(convert_condition_set(condition_set, cx))
        }
    }
}

fn convert_comparison<D: Dialect>(
    comparison: &Comparison,
    cx: &mut RenderingContext<D>,
) -> SqlConditionSetEntry {
    convert_simple_condition_set_entry(&expand_comparison(comparison), cx)
}

fn convert_simple_condition_set_entry<D: Dialect>(
    entry: &SimpleConditionSetEntry,
    cx: &mut RenderingContext<D>,
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

fn convert_simple_comparison<D: Dialect>(
    s: &SimpleComparison,
    cx: &mut RenderingContext<D>,
) -> SqlConditionSetEntry {
    // When we see that we're comparing an expression equal to zero or greater to zero, then we
    // hand off the conversion to the context because, depending on the expression, the context
    // may choose to handle this condition via a join instead of a condition set entry. In that
    // case we'll receive an empty SqlConditionSet back, and that will get filtered out later on.
    if s.left.is_zero() && s.operator == Operator::Eq {
        return convert_expression_eq_0(&s.right, cx);
    }
    if s.left.is_zero() && s.operator == Operator::Lt {
        return convert_expression_gt_0(&s.right, cx);
    }
    if s.right.is_zero() && s.operator == Operator::Eq {
        return convert_expression_eq_0(&s.left, cx);
    }
    if s.right.is_zero() && s.operator == Operator::Gt {
        return convert_expression_gt_0(&s.left, cx);
    }
    SqlConditionSetEntry::Expression(format!(
        "{} {} {}",
        s.left.render(cx),
        s.operator.render(cx),
        s.right.render(cx)
    ))
}

fn convert_expression_gt_0<D: Dialect>(
    expr: &Expression,
    cx: &mut RenderingContext<D>,
) -> SqlConditionSetEntry {
    // TODO_CODE
    todo!()
}

fn convert_expression_eq_0<D: Dialect>(
    expr: &Expression,
    cx: &mut RenderingContext<D>,
) -> SqlConditionSetEntry {
    // TODO_CODE
    todo!()
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

pub fn simplify_expression<D: Dialect>(
    expr: &Expression,
    cx: &mut RenderingContext<D>,
) -> SimpleExpression {
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

fn simplify_path_expression<D: Dialect>(
    parts: Vec<PathPart>,
    compositions: Vec<Composition>,
    cx: &mut RenderingContext<D>,
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
        (head, ClarifiedPathTail::ChainToMany((chain_to_many, column_name_opt))) => {
            Ok(cx.join_chain_to_many(&head, chain_to_many, column_name_opt, compositions))
        }
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
    ChainToMany((Chain<GenericLink>, Option<String>)),
}

fn clarify_path<D: Dialect>(
    parts: Vec<PathPart>,
    cx: &RenderingContext<D>,
) -> Result<ClarifiedPath, String> {
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
    let mut chain_to_many_opt: Option<Chain<GenericLink>> = None;
    for generic_link in chain {
        if let Some(chain_to_many) = &mut chain_to_many_opt {
            // This unwrap is safe because we know that the chain has already been constructed.
            // We're just re-constructing part of it.
            chain_to_many.try_append(generic_link).unwrap();
        } else {
            match LinkToOne::try_from(generic_link) {
                Ok(link_to_one) => {
                    if let Some(chain) = &mut head {
                        // This unwrap is safe because we know that the chain has already been
                        // constructed using GenericLink links. All we're doing here is
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
    pub chain: Option<Chain<GenericLink>>,
    pub column: Option<String>,
}

fn build_linked_path<D: Dialect>(
    parts: Vec<PathPart>,
    cx: &RenderingContext<D>,
) -> Result<LinkedPath, String> {
    let mut current_table_opt: Option<&Table> = Some(cx.get_base_table());
    let mut chain_opt: Option<Chain<GenericLink>> = None;
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
                    let generic_link = GenericLink::ForwardLinkToOne(link);
                    chain_opt = match chain_opt {
                        Some(mut chain) => {
                            chain.try_append(generic_link)?;
                            Some(chain)
                        }
                        None => Some(Chain::try_new(generic_link, ChainIntersecting::Allowed)?),
                    };
                } else {
                    current_table_opt = None;
                    final_column_name = Some(column_name);
                }
            }
            PathPart::TableWithOne(table_name) => {
                todo!()
            }
            PathPart::TableWithMany(table_with_many) => {
                let base = ChainSearchBase::TableId(current_table.id);
                let mut new_chain =
                    cx.schema
                        .get_chain_to_table_with_many(base, &table_with_many, None)?;
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
    chain_opt = if let Some(mut chain) = chain_opt {
        // If the last link is a forward link to one, then we can remove it and use the column
        // name as the final column name instead. This saves us from having to do a
        // unnecessary join.
        if let Some(GenericLink::ForwardLinkToOne(last_link)) = chain.get_links().last() {
            let column_name = get_fk_column_name(last_link, cx.schema);
            final_column_name = Some(column_name);
            if chain.pop_last().is_none() {
                None
            } else {
                Some(chain)
            }
        } else {
            final_column_name = None;
            Some(chain)
        }
    } else {
        chain_opt
    };
    Ok(LinkedPath {
        chain: chain_opt,
        column: final_column_name,
    })
}

pub fn convert_join_tree<D: Dialect>(tree: &JoinTree, cx: &RenderingContext<D>) -> Vec<Join> {
    let mut joins: Vec<Join> = vec![];
    for (link, subtree) in tree.get_dependents().iter() {
        let starting_alias = tree.get_alias();
        let ending_alias = subtree.get_alias();
        let join_type = JoinType::LeftOuter;
        let join = make_join_from_link(link, starting_alias, ending_alias, join_type, cx);
        joins.push(join);
        joins.extend(convert_join_tree(subtree, cx));
    }
    joins
}

pub fn build_cte_select<D: Dialect>(
    chain: Chain<GenericLink>,
    final_column_name: Option<String>,
    compositions: Vec<Composition>,
    context_of_parent_query: &RenderingContext<D>,
) -> Select {
    let mut links_iter = chain.into_iter();
    // Unwrap is safe because we know a chain will contain at least one link.
    let generic_first_link = links_iter.next().unwrap();
    let GenericLink::FilteredReverseLinkToMany(first_link) = generic_first_link else {
        // This should never happen because we've already split the chain into a tail which begins
        // FilteredReverseLinkToMany link.
        panic!("Bug: Tried to build a CTE but the first link was a link to one.");
    };
    let base = first_link.link.base;
    let base_table = context_of_parent_query
        .schema
        .tables
        .get(&base.table_id)
        // Unwrap is safe because we know schema only has valid links.
        .unwrap();
    // Unwrap is safe because we know schema only has valid links.
    let base_column = base_table.columns.get(&base.column_id).unwrap();
    let mut cx = context_of_parent_query.spawn(&base_table);
    let mut select = Select::from(cx.get_base_table().name.clone());
    let pk_expr = Literal::TableColumnReference(base_table.name.clone(), base_column.name.clone())
        .render(&mut cx);
    select.grouping.push(pk_expr.clone());
    select.columns.push(Column {
        expression: pk_expr,
        alias: Some(CTE_PK_COLUMN_ALIAS.to_owned()),
    });
    for link in links_iter {
        let starting_alias = "TODO";
        let ending_alias = "TODO";
        let join_type = JoinType::Inner;
        let join = make_join_from_link(&link, starting_alias, ending_alias, join_type, &cx);
        select.joins.push(join);
    }
    select
}

fn make_join_from_link<D: Dialect>(
    link: &impl Link,
    starting_alias: &str,
    ending_alias: &str,
    join_type: JoinType,
    cx: &RenderingContext<D>,
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
        format!("Column {column_name} not found within table {table_name}.")
    }

    pub fn no_path_parts() -> String {
        "Cannot build a ClarifiedPath without any path parts".to_string()
    }

    pub fn no_column_name_or_chain() -> String {
        "Cannot build a ClarifiedPathTail without a column name or chain".to_string()
    }

    pub fn spawn_context_with_invalid_table_id() -> String {
        "Attempted to spawn a child rendering context using an invalid table ID.".to_string()
    }

    pub fn cte_from_chain_that_does_not_start_with_link_to_many() -> String {
        "Cannot spawn CTE context for chain that does not start with a link to many.".to_string()
    }
}
