use itertools::Itertools;

use crate::{
    compiling::{
        constants::{CTE_PK_COLUMN_ALIAS, CTE_VALUE_COLUMN_PREFIX},
        conversion::conditions::convert_condition_set,
        join_tree::JoinTree,
        rendering::rendering::Render,
        scope::Scope,
        sql_tree::{
            Column, Cte, CtePurpose, Join, JoinType, Select, SqlConditionSet, SqlConditionSetEntry,
        },
    },
    dialects::sql,
    errors::msg,
    schema::{
        chain::{Chain, ChainIntersecting},
        links::{FilteredLink, Link, LinkToOne, MultiLink},
        ChainSearchBase, Table,
    },
    syntax_tree::{
        Composition, ConditionSet, Conjunction, Expression, FunctionDimension, Literal, PathPart,
        TableWithMany, Value,
    },
};

#[derive(Debug)]
pub struct ClarifiedPath {
    pub head: Option<Chain<LinkToOne>>,
    pub tail: ClarifiedPathTail,
}

#[derive(Debug)]
pub enum ClarifiedPathTail {
    Column(String),
    /// chain, column_name
    ChainToMany((Chain<FilteredLink>, Option<String>)),
}

pub fn clarify_path(parts: Vec<PathPart>, scope: &Scope) -> Result<ClarifiedPath, String> {
    let linked_path = build_linked_path(parts, scope)?;
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

fn build_linked_path(parts: Vec<PathPart>, scope: &Scope) -> Result<LinkedPath, String> {
    let mut current_table_opt: Option<&Table> = Some(scope.get_base_table());
    let mut chain_opt: Option<Chain<FilteredLink>> = None;
    let mut final_column_name: Option<String> = None;
    for part in parts {
        let current_table = current_table_opt.ok_or_else(msg::no_current_table)?;
        match part {
            PathPart::Column(column_name) => {
                let column_id = scope
                    .options
                    .resolve_identifier(&current_table.column_lookup, &column_name)
                    .copied()
                    .ok_or_else(|| msg::col_not_in_table(&column_name, &current_table.name))?;
                if let Some(link) = current_table.forward_links_to_one.get(&column_id).copied() {
                    current_table_opt = scope.schema.tables.get(&link.get_end().table_id);
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
                    let column = current_table.columns.get(&column_id).unwrap();
                    current_table_opt = None;
                    final_column_name = Some(column.name.clone());
                }
            }
            PathPart::TableWithOne(table_name) => {
                todo!()
            }
            PathPart::TableWithMany(mut table_with_many) => {
                let base = ChainSearchBase::TableId(current_table.id);
                let condition_set = std::mem::take(&mut table_with_many.condition_set);
                let mut new_chain =
                    get_chain_to_table_with_many(base, &table_with_many, None, scope)?;
                new_chain.set_final_condition_set(condition_set);
                new_chain.allow_intersecting();
                current_table_opt = scope.schema.tables.get(&new_chain.get_ending_table_id());
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

pub fn get_chain_to_table_with_many(
    base: ChainSearchBase,
    target: &TableWithMany,
    max_chain_length: Option<usize>,
    scope: &Scope,
) -> Result<Chain<FilteredLink>, String> {
    let max_chain_len = max_chain_length.unwrap_or(usize::MAX);
    if base.len() >= max_chain_len {
        // I don't think this should never happen, but I put it here just in case
        return Err("Chain search base already too long before searching.".to_string());
    }
    let target_table = scope
        .get_table_by_name(&target.table)
        .ok_or("Target table not found.".to_string())?;

    // Success case where the base is already at the target
    if base.get_ending_table_id() == Some(target_table.id) {
        if let ChainSearchBase::Chain(multi_link_chain) = base {
            return Ok(Chain::<FilteredLink>::from(multi_link_chain));
        }
    }

    let base_table = scope
        .schema
        .tables
        .get(&base.get_base_table_id())
        .ok_or("Base table not found.".to_string())?;

    // Success case where we can directly find the target from the base
    if let Some(links) = base_table.reverse_links_to_many.get(&target_table.id) {
        if let Ok(link) = links.iter().exactly_one() {
            let multi_link = MultiLink::ReverseLinkToMany(*link);
            if let Ok(multi_link_chain) = base.clone().try_append_into_chain(multi_link) {
                return Ok(Chain::<FilteredLink>::from(multi_link_chain));
            }
        }
    }

    if base.len() + 1 >= max_chain_len {
        return Err("Max chain length reached.".to_string());
    }

    let get_transitive_chain = |link: MultiLink, max: usize| {
        let chain = base.clone().try_append_into_chain(link)?;
        get_chain_to_table_with_many(ChainSearchBase::Chain(chain), target, Some(max), scope)
    };
    enum ChainSearchResult {
        Winner(Chain<FilteredLink>),
        Tie(usize),
        NoneFound,
    }
    let get_max_len = |result: &ChainSearchResult| match result {
        ChainSearchResult::Winner(chain) => chain.len(),
        ChainSearchResult::Tie(len) => *len,
        ChainSearchResult::NoneFound => max_chain_len,
    };

    // Recursive case
    let mut result = ChainSearchResult::NoneFound;
    for link in base_table.get_links() {
        let max_len = get_max_len(&result);
        let Ok(chain) = get_transitive_chain(link, max_len) else {continue};
        if let ChainSearchResult::Winner(winner) = &result {
            if chain.len() == winner.len() {
                result = ChainSearchResult::Tie(chain.len());
            } else if chain.len() < winner.len() {
                result = ChainSearchResult::Winner(chain);
            }
        } else {
            result = ChainSearchResult::Winner(chain);
        }
    }
    match result {
        ChainSearchResult::Winner(chain) => Ok(chain),
        ChainSearchResult::Tie(_) => Err("Two chains tie for the same length".to_string()),
        ChainSearchResult::NoneFound => Err("No chain found.".to_string()),
    }
}

pub fn convert_join_tree(mut tree: JoinTree, scope: &Scope) -> (Vec<Join>, Vec<Cte>) {
    let mut ctes = tree.take_ctes();
    let mut joins: Vec<Join> = ctes
        .iter()
        .map(|cte| build_join_for_cte(cte, tree.get_alias().to_owned(), scope))
        .collect();
    for (link, subtree) in tree.take_dependents() {
        let starting_alias = tree.get_alias();
        let ending_alias = subtree.get_alias();
        let join_type = JoinType::LeftOuter;
        let join = make_join_from_link(&link, starting_alias, ending_alias, join_type, scope);
        joins.push(join);
        let (new_joins, new_ctes) = convert_join_tree(subtree, scope);
        joins.extend(new_joins);
        ctes.extend(new_ctes);
    }
    (joins, ctes)
}

fn build_join_for_cte(cte: &Cte, table: String, scope: &Scope) -> Join {
    let condition = format!(
        "{} = {}",
        scope
            .options
            .dialect
            .table_column(&table, &cte.join_column_name),
        scope
            .options
            .dialect
            .table_column(&cte.alias, CTE_PK_COLUMN_ALIAS),
    );
    Join {
        table: cte.alias.clone(),
        alias: cte.alias.clone(),
        condition_set: SqlConditionSet {
            conjunction: Conjunction::And,
            entries: vec![SqlConditionSetEntry::Expression(condition)],
        },
        join_type: JoinType::LeftOuter,
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
    parent_cx: &Scope,
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
                let column_id = cte_cx
                    .options
                    .resolve_identifier(&ending_table.column_lookup, &column_name)
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
    scope: &Scope,
) -> Join {
    let start = link.get_start();
    let starting_table_id = start.table_id;
    let starting_table = scope.schema.tables.get(&starting_table_id).unwrap();
    let starting_column_id = start.column_id;
    let starting_column = starting_table.columns.get(&starting_column_id).unwrap();

    let end = link.get_end();
    let ending_table_id = end.table_id;
    let ending_table = scope.schema.tables.get(&ending_table_id).unwrap();
    let ending_column_id = end.column_id;
    let ending_column = ending_table.columns.get(&ending_column_id).unwrap();

    let condition = format!(
        "{} = {}",
        scope
            .options
            .dialect
            .table_column(starting_alias, &starting_column.name),
        scope
            .options
            .dialect
            .table_column(ending_alias, &ending_column.name),
    );
    Join {
        table: scope
            .schema
            .tables
            .get(&ending_table_id)
            .unwrap()
            .name
            .clone(),
        alias: ending_alias.to_owned(),
        condition_set: SqlConditionSet {
            conjunction: Conjunction::And,
            entries: vec![SqlConditionSetEntry::Expression(condition)],
        },
        join_type,
    }
}
