use std::collections::HashSet;

use querydown_parser::ast::{ConditionSet, PathPart, TableWithMany};

use crate::{
    compiler::scope::Scope,
    errors::msg,
    schema::{
        chain::{Chain, ChainIntersecting},
        links::{FilteredLink, Link, LinkToOne, MultiLink, Reference},
        Table, TableId,
    },
};

#[derive(Debug)]
// TODO: this data structure has grown and now seems like it needs a bit of refactoring. For the
// sake of correctness, it should not permit both the head and the tail to be None. And the sake
// of ergonomics, it should probably not be quite so nested. We might consider flattening it out
// to an enum with many different variants.
pub struct ClarifiedPath {
    pub head: Option<Chain<LinkToOne>>,
    pub tail: Option<ClarifiedPathTail>,
}

#[derive(Debug)]
pub enum ClarifiedPathTail {
    Column(String),
    /// chain, column_name, order_by_anchor_table_id
    ///
    /// The `order_by_anchor_table_id` identifies the table named by the first `#` part of the path
    /// (the first table reached via a to-many link). Any sorting conditions attached to an aggregate
    /// function applied to this path are evaluated from the context of that table — see
    /// [`crate::compiler::paths::build_cte_select`].
    ChainToMany((Chain<FilteredLink>, Option<String>, TableId)),
}

pub fn clarify_path(parts: Vec<PathPart>, scope: &Scope) -> Result<ClarifiedPath, String> {
    let linked_path = build_linked_path(parts, scope)?;
    let chain_opt = linked_path.chain;
    let column_name_opt = linked_path.column;
    let first_to_many_end_table_id = linked_path.first_to_many_end_table_id;
    let Some(chain) = chain_opt else {
        return column_name_opt
            .map(|column_name| ClarifiedPath {
                head: None,
                tail: Some(ClarifiedPathTail::Column(column_name)),
            })
            .ok_or_else(msg::no_path_parts);
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
        // `first_to_many_end_table_id` is always set when a to-many chain exists, since the chain
        // begins at a to-many link produced by a `#` (TableWithMany) path part. The fallback to the
        // chain's ending table is a defensive default that should never be reached.
        let anchor_table_id =
            first_to_many_end_table_id.unwrap_or_else(|| chain_to_many.get_ending_table_id());
        Some(ClarifiedPathTail::ChainToMany((
            chain_to_many,
            column_name_opt,
            anchor_table_id,
        )))
    } else {
        column_name_opt.map(ClarifiedPathTail::Column)
    };
    Ok(ClarifiedPath { head, tail })
}

/// Returns true when two paths, both evaluated from the scope's base table, reach the same column
/// via the same route (the same links, with the same filters). Paths that fail to resolve are never
/// alike. Comparing resolved paths rather than their written form means that differences which
/// don't affect the result (such as the letter case of identifiers, when the compiler is configured
/// to ignore it) don't prevent a match.
pub fn paths_resolve_alike(a: Vec<PathPart>, b: Vec<PathPart>, scope: &Scope) -> bool {
    let (Ok(a), Ok(b)) = (build_linked_path(a, scope), build_linked_path(b, scope)) else {
        return false;
    };
    type LinkKey = (Reference, Reference, ConditionSet);
    let link_keys = |chain: Option<Chain<FilteredLink>>| -> Vec<LinkKey> {
        chain
            .into_iter()
            .flatten()
            .map(|link| (link.get_start(), link.get_end(), link.condition_set))
            .collect()
    };
    a.column == b.column && link_keys(a.chain) == link_keys(b.chain)
}

#[derive(Debug)]
struct LinkedPath {
    pub chain: Option<Chain<FilteredLink>>,
    /// The name of the column at the end of the path, if there is one. When a path ends with a FK
    /// column, then the column will be treated as a link and will be included in the chain,
    /// making the `column` field `None`.
    pub column: Option<String>,
    /// The table reached by the first `#` (TableWithMany) part of the path, i.e. the ending table of
    /// that part's chain. This anchors any sorting conditions attached to an aggregate function. It
    /// is `None` when the path contains no to-many part.
    pub first_to_many_end_table_id: Option<TableId>,
}

fn build_linked_path(parts: Vec<PathPart>, scope: &Scope) -> Result<LinkedPath, String> {
    let mut current_table_opt: Option<&Table> = Some(scope.get_base_table());
    let mut chain_opt: Option<Chain<FilteredLink>> = None;
    let mut final_column_name: Option<String> = None;
    let mut first_to_many_end_table_id: Option<TableId> = None;
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
                return Err(msg::table_with_one_path_unsupported(&table_name));
            }
            PathPart::TableWithMany(mut table_with_many) => {
                let condition_set = std::mem::take(&mut table_with_many.condition_set);
                let mut new_chain =
                    get_chain_to_table_with_many(current_table, &table_with_many, scope)?;
                new_chain.set_final_condition_set(condition_set);
                new_chain.allow_intersecting();
                if first_to_many_end_table_id.is_none() {
                    first_to_many_end_table_id = Some(new_chain.get_ending_table_id());
                }
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
        first_to_many_end_table_id,
    })
}

/// Finds the route that a `#table` path part takes from `start`: the shortest series of links that
/// ends at the named table. When the path part names a linking column (as in `#blocks(blocker)`),
/// the route's final link must follow that foreign key column.
///
/// A route never visits a table twice, with one exception: its final link may be a self-referencing
/// link from the named table back to itself (e.g. `issues.duplicate_of`). That is how a path part
/// like `#issues` can lead from `issues` to other `issues` records.
///
/// If more than one route ties for being the shortest, the path part is ambiguous.
fn get_chain_to_table_with_many(
    start: &Table,
    target: &TableWithMany,
    scope: &Scope,
) -> Result<Chain<FilteredLink>, String> {
    let schema = scope.schema;
    let target_table = scope
        .get_table_by_name(&target.table)
        .ok_or_else(|| msg::table_not_found(&target.table))?;
    let uses_linking_column = |link: &MultiLink| -> bool {
        let Some(column_name) = &target.linking_column else {
            return true;
        };
        let column = link.get_foreign_key_column();
        let table = schema.tables.get(&column.table_id).unwrap();
        scope
            .options
            .resolve_identifier(&table.column_lookup, column_name)
            == Some(&column.column_id)
    };
    let extend = |prefix: &Option<Chain<MultiLink>>, link: MultiLink, intersecting| match prefix {
        None => Chain::try_new(link, intersecting).ok(),
        Some(chain) => {
            let mut chain = chain.clone();
            if intersecting == ChainIntersecting::Allowed {
                chain.allow_intersecting();
            }
            chain.try_append(link).ok().map(|()| chain)
        }
    };

    // Search outward from `start` one link at a time, so that the first routes found are the
    // shortest. Each prefix is a route (empty at first) along with the table it ends at.
    let mut prefixes: Vec<(Option<Chain<MultiLink>>, TableId)> = vec![(None, start.id)];
    // Tables reached at an earlier step. A route that reaches one of these later can't be shortest.
    let mut visited = HashSet::from([start.id]);
    while !prefixes.is_empty() {
        let mut routes: Vec<Chain<MultiLink>> = vec![];
        let mut next_prefixes = vec![];
        let mut reached = HashSet::new();
        for (prefix, end_table_id) in &prefixes {
            let table = schema.tables.get(end_table_id).unwrap();
            for link in table.get_links() {
                let link_end_table_id = link.get_end().table_id;
                if link_end_table_id == *end_table_id {
                    // A self-referencing link can only end a route, and only when followed from the
                    // referenced record to the many records that reference it.
                    let is_route = link_end_table_id == target_table.id
                        && matches!(link, MultiLink::ReverseLinkToMany(_))
                        && uses_linking_column(&link);
                    if is_route {
                        routes.extend(extend(prefix, link, ChainIntersecting::Allowed));
                    }
                    continue;
                }
                if visited.contains(&link_end_table_id) {
                    continue;
                }
                let Some(chain) = extend(prefix, link, ChainIntersecting::Disallowed) else {
                    continue;
                };
                if link_end_table_id == target_table.id && uses_linking_column(&link) {
                    routes.push(chain.clone());
                }
                // A route that reached the target via some other column may still continue along a
                // self-referencing link that uses the right one.
                reached.insert(link_end_table_id);
                next_prefixes.push((Some(chain), link_end_table_id));
            }
        }
        match routes.len() {
            0 => {}
            1 => return Ok(Chain::from(routes.pop().unwrap())),
            _ => return Err(ambiguous_route_error(&routes, start, target, scope)),
        }
        visited.extend(reached);
        prefixes = next_prefixes;
    }
    Err(match &target.linking_column {
        Some(column) => msg::no_path_to_table_via_column(&start.name, &target_table.name, column),
        None if start.id == target_table.id => msg::no_path_from_table_to_itself(&start.name),
        None => msg::no_path_to_table(&start.name, &target.table),
    })
}

/// Explains why a `#table` path part is ambiguous. When the tied routes differ only in which
/// foreign key their final link follows, the error suggests naming that column.
fn ambiguous_route_error(
    routes: &[Chain<MultiLink>],
    start: &Table,
    target: &TableWithMany,
    scope: &Scope,
) -> String {
    let route_up_to_final_link = |route: &Chain<MultiLink>| -> Vec<(Reference, Reference)> {
        let links = route.get_links();
        links[..links.len() - 1]
            .iter()
            .map(|link| (link.get_start(), link.get_end()))
            .collect()
    };
    let first_route = route_up_to_final_link(&routes[0]);
    let differ_only_in_final_link = target.linking_column.is_none()
        && routes
            .iter()
            .all(|route| route_up_to_final_link(route) == first_route);
    let column_names: Vec<String> = if differ_only_in_final_link {
        routes
            .iter()
            .map(|route| {
                let final_link = route.get_links().last().unwrap();
                let column = final_link.get_foreign_key_column();
                scope.schema.get_referenced_column_name(&column)
            })
            .collect()
    } else {
        vec![]
    };
    msg::ambiguous_path_to_table(&start.name, &target.table, &column_names)
}
