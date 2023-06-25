use std::collections::HashSet;

use querydown_parser::ast::ConditionSet;

use crate::schema::schema::TableId;

use super::links::{FilteredLink, Link, MultiLink};

#[derive(Debug, Clone)]
struct ChainStats {
    starting_table_id: TableId,
    ending_table_id: TableId,
    table_ids: HashSet<TableId>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ChainIntersecting {
    Allowed,
    Disallowed,
}

#[derive(Debug)]
/// A series of zero or more connected links, along with cached information about that series
/// for the purpose of easy analysis.
pub struct Chain<L: Link> {
    links: Vec<L>,
    stats: ChainStats,
    intersecting: ChainIntersecting,
}

impl<L: Link> Chain<L> {
    // The number of links in the chain
    pub fn len(&self) -> usize {
        self.links.len()
    }

    fn validate_link(link: &L, intersecting: &ChainIntersecting) -> Result<(), &'static str> {
        if *intersecting == ChainIntersecting::Allowed {
            return Ok(());
        }
        let starting_table_id = link.get_start().table_id;
        let ending_table_id = link.get_end().table_id;
        if starting_table_id == ending_table_id {
            return Err("Self-referential links cannot be part of a non-intersecting chain");
        }
        Ok(())
    }

    pub fn try_new(link: L, intersecting: ChainIntersecting) -> Result<Self, &'static str> {
        Self::validate_link(&link, &intersecting)?;
        let starting_table_id = link.get_start().table_id;
        let ending_table_id = link.get_end().table_id;
        Ok(Self {
            links: Vec::from([link]),
            stats: ChainStats {
                starting_table_id,
                ending_table_id,
                table_ids: HashSet::from([starting_table_id, ending_table_id]),
            },
            intersecting,
        })
    }

    pub fn get_starting_table_id(&self) -> TableId {
        self.stats.starting_table_id
    }

    pub fn get_ending_table_id(&self) -> TableId {
        self.stats.ending_table_id
    }

    pub fn get_links(&self) -> &[L] {
        &self.links
    }

    pub fn get_first_link(&self) -> &L {
        // This unwrap is safe because we know that a chain will have at least one link
        self.links.first().unwrap()
    }

    pub fn allow_intersecting(&mut self) {
        self.intersecting = ChainIntersecting::Allowed
    }

    /// Try to add a link to the end of this chain. If it was successfully added, then return
    /// `Ok(())`. If it can't be added, then return an error message.
    pub fn try_append(&mut self, link: L) -> Result<(), &'static str> {
        Self::validate_link(&link, &self.intersecting)?;
        let link_starting_table_id = link.get_start().table_id;
        let link_ending_table_id = link.get_end().table_id;
        if self.stats.ending_table_id != link_starting_table_id {
            return Err("Link does not connect to chain");
        }
        if self.stats.table_ids.contains(&link_ending_table_id) {
            return Err("Link would cause chain to intersect itself");
        }
        self.links.push(link);
        self.stats.ending_table_id = link_ending_table_id;
        self.stats.table_ids.insert(link_ending_table_id);
        Ok(())
    }

    pub fn try_connect(&mut self, chain: Chain<L>) -> Result<(), &'static str> {
        if self.intersecting != chain.intersecting {
            return Err("Cannot connect chains with different intersecting settings.");
        }
        if self.stats.ending_table_id != chain.stats.starting_table_id {
            return Err("Chains do not connect.");
        }
        // This is compared to 1 because intersection will always contain the shared table id
        if self
            .stats
            .table_ids
            .intersection(&chain.stats.table_ids)
            .count()
            > 1
        {
            if self.intersecting == ChainIntersecting::Disallowed {
                return Err("Chains would intersect.");
            }
        }
        self.links.extend(chain.links);
        self.stats.ending_table_id = chain.stats.ending_table_id;
        self.stats.table_ids.extend(chain.stats.table_ids);
        Ok(())
    }
}

impl<L: Link> IntoIterator for Chain<L> {
    type Item = L;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.links.into_iter()
    }
}

fn calculate_table_ids<'a, L: Link + 'a>(links: impl Iterator<Item = &'a L>) -> HashSet<TableId> {
    let mut table_ids = HashSet::new();
    for link in links {
        table_ids.insert(link.get_start().table_id);
        table_ids.insert(link.get_end().table_id);
    }
    table_ids
}

impl<L: Link + Copy> Chain<L> {
    pub fn with_first_link_broken_off(&self) -> (&L, Option<Self>) {
        // This unwrap is safe because we know that a chain will have at least one link
        let first_link = self.links.first().unwrap();
        let remaining_links = self.links[1..].to_vec();
        let new_chain = remaining_links.first().copied().map(|new_first_link| {
            let new_starting_table_id = new_first_link.get_start().table_id;
            let table_ids = calculate_table_ids(remaining_links.iter());
            Self {
                links: remaining_links,
                stats: ChainStats {
                    starting_table_id: new_starting_table_id,
                    ending_table_id: self.get_ending_table_id(),
                    table_ids,
                },
                intersecting: self.intersecting,
            }
        });
        (first_link, new_chain)
    }

    pub fn with_last_link_broken_off(&self) -> (Option<Self>, &L) {
        // This unwrap is safe because we know that a chain will have at least one link
        let last_link = self.links.last().unwrap();
        let remaining_links = self.links[..self.links.len() - 1].to_vec();
        let new_chain = remaining_links.last().copied().map(|new_last_link| {
            let new_ending_table_id = new_last_link.get_end().table_id;
            let table_ids = calculate_table_ids(remaining_links.iter());
            Self {
                links: remaining_links,
                stats: ChainStats {
                    starting_table_id: self.get_starting_table_id(),
                    ending_table_id: new_ending_table_id,
                    table_ids,
                },
                intersecting: self.intersecting,
            }
        });
        (new_chain, last_link)
    }
}

impl<L: Link + Clone> Clone for Chain<L> {
    fn clone(&self) -> Self {
        Self {
            links: self.links.clone(),
            stats: self.stats.clone(),
            intersecting: self.intersecting,
        }
    }
}

impl From<Chain<MultiLink>> for Chain<FilteredLink> {
    fn from(chain: Chain<MultiLink>) -> Self {
        Self {
            links: chain.links.into_iter().map(|link| link.into()).collect(),
            stats: chain.stats,
            intersecting: chain.intersecting,
        }
    }
}

impl Chain<FilteredLink> {
    pub fn set_final_condition_set(&mut self, condition_set: ConditionSet) {
        // unwrap is safe here because we know that a chain will have at least one link
        let last_link = self.links.last_mut().unwrap();
        last_link.condition_set = condition_set;
    }
}
