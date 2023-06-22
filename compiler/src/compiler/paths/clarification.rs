use itertools::Itertools;

use querydown_parser::ast::{ConditionSet, PathPart, TableWithMany};

use crate::{
    compiler::scope::Scope,
    errors::msg,
    schema::{
        chain::{Chain, ChainIntersecting},
        links::{FilteredLink, Link, LinkToOne, MultiLink},
        ChainSearchBase, Table,
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

fn get_chain_to_table_with_many(
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
        .ok_or_else(|| "Target table not found.".to_string())?;

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
        .ok_or_else(|| "Base table not found.".to_string())?;

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
