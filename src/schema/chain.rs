use std::collections::HashSet;

use crate::schema::schema::TableId;

use super::links::{Link, LinkToOne};

#[derive(Debug, Clone)]
struct ChainStats {
    starting_table_id: TableId,
    ending_table_id: TableId,
    table_ids: HashSet<TableId>,
}

#[derive(Debug)]
/// A series of zero or more connected links, along with cached information about that series
/// for the purpose of easy analysis.
pub struct ChainToOne {
    links: Vec<LinkToOne>,
    stats: ChainStats,
}

impl ChainToOne {
    pub fn validate_link(link: &LinkToOne) -> Result<(), &'static str> {
        let starting_table_id = link.get_start().table_id;
        let ending_table_id = link.get_end().table_id;
        if starting_table_id == ending_table_id {
            return Err("Self-referential links cannot be part of a chain");
        }
        Ok(())
    }

    pub fn new(link: &LinkToOne) -> Result<Self, &'static str> {
        Self::validate_link(link)?;
        let starting_table_id = link.get_start().table_id;
        let ending_table_id = link.get_end().table_id;
        Ok(Self {
            links: Vec::from([*link]),
            stats: ChainStats {
                starting_table_id,
                ending_table_id,
                table_ids: HashSet::from([starting_table_id, ending_table_id]),
            },
        })
    }

    pub fn get_starting_table_id(&self) -> TableId {
        self.stats.starting_table_id
    }

    pub fn get_ending_table_id(&self) -> TableId {
        self.stats.ending_table_id
    }

    pub fn get_links(&self) -> &[LinkToOne] {
        &self.links
    }

    pub fn has_table_id(&self, table_id: TableId) -> bool {
        self.stats.table_ids.contains(&table_id)
    }

    /// Attempt to create a new valid chain by adding a link to this chain.
    pub fn with(&self, link: &LinkToOne) -> Result<Self, &'static str> {
        Self::validate_link(link)?;
        let link_starting_table_id = link.get_start().table_id;
        let link_ending_table_id = link.get_end().table_id;
        if self.stats.ending_table_id != link_starting_table_id {
            return Err("Link does not connect to chain");
        }
        if self.stats.table_ids.contains(&link_ending_table_id) {
            return Err("Link would cause chain to intersect itself");
        }
        let mut links = self.links.clone();
        links.push(*link);
        let mut table_ids = self.stats.table_ids.clone();
        table_ids.insert(link_ending_table_id);
        Ok(Self {
            links,
            stats: ChainStats {
                starting_table_id: self.stats.starting_table_id,
                ending_table_id: link_ending_table_id,
                table_ids,
            },
        })
    }

    pub fn with_first_link_broken_off(&self) -> (&LinkToOne, Option<Self>) {
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
            }
        });
        (first_link, new_chain)
    }
}

impl TryFrom<Vec<LinkToOne>> for ChainToOne {
    type Error = &'static str;

    fn try_from(links: Vec<LinkToOne>) -> Result<Self, &'static str> {
        let mut iter = links.into_iter();
        let first_link_option = iter.next();
        match first_link_option {
            Some(first_link) => {
                let mut chain = ChainToOne::new(&first_link)?;
                for link in iter {
                    chain = chain.with(&link)?;
                }
                Ok(chain)
            }
            None => Err("A chain cannot be created from an empty list of links"),
        }
    }
}
