use std::collections::HashSet;

use crate::{schema::schema::TableId, syntax_tree::ConditionSet};

use super::links::{GenericLink, Link, SimpleLink};

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

    pub fn has_table_id(&self, table_id: TableId) -> bool {
        self.stats.table_ids.contains(&table_id)
    }

    pub fn allow_intersecting(&mut self) {
        self.intersecting = ChainIntersecting::Allowed
    }

    pub fn disallow_intersecting(&mut self) {
        self.intersecting = ChainIntersecting::Disallowed
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

impl<L: Link + Copy> Chain<L> {
    pub fn with_first_link_broken_off(&self) -> (&L, Option<Self>) {
        // This unwrap is safe because we know that a chain will have at least one link
        let first_link = self.links.first().unwrap();
        let remaining_links = self.links[1..].to_vec();
        let new_chain = remaining_links.first().copied().map(|new_first_link| {
            let new_starting_table_id = new_first_link.get_start().table_id;
            let mut table_ids = self.stats.table_ids.clone();
            table_ids.remove(&first_link.get_start().table_id);
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

impl From<Chain<SimpleLink>> for Chain<GenericLink> {
    fn from(chain: Chain<SimpleLink>) -> Self {
        Self {
            links: chain.links.into_iter().map(|link| link.into()).collect(),
            stats: chain.stats,
            intersecting: chain.intersecting,
        }
    }
}

impl Chain<GenericLink> {
    pub fn set_final_condition_set(&mut self, condition_set: ConditionSet) {
        // unwrap is safe here because we know that a chain will have at least one link
        self.links
            .last_mut()
            .unwrap()
            .set_condition_set(condition_set);
    }
}
