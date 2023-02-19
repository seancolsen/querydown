use std::collections::HashSet;

use crate::schema::schema::{JoinQuantity, Link, TableId};

#[derive(Debug, Clone)]
/// A series of zero or more connected links, along with cached information about that series
/// for the purpose of easy analysis.
pub struct Chain {
    links: Vec<Link>,
    starting_table_id: TableId,
    ending_table_id: TableId,
    join_quantity: JoinQuantity,
    table_ids: HashSet<TableId>,
}

impl Chain {
    pub fn validate_link(link: &Link) -> Result<(), &'static str> {
        let starting_table_id = link.get_starting_table_id();
        let ending_table_id = link.get_ending_table_id();
        if starting_table_id == ending_table_id {
            return Err("Self-referential links cannot be part of a chain");
        }
        Ok(())
    }

    pub fn new(link: &Link) -> Result<Self, &'static str> {
        Self::validate_link(link)?;
        let starting_table_id = link.get_starting_table_id();
        let ending_table_id = link.get_ending_table_id();
        Ok(Self {
            starting_table_id,
            ending_table_id,
            links: Vec::from([*link]),
            join_quantity: link.get_join_quantity(),
            table_ids: HashSet::from([starting_table_id, ending_table_id]),
        })
    }

    pub fn get_starting_table_id(&self) -> TableId {
        self.starting_table_id
    }

    pub fn get_ending_table_id(&self) -> TableId {
        self.ending_table_id
    }

    pub fn get_links(&self) -> &[Link] {
        &self.links
    }

    pub fn get_join_quantity(&self) -> JoinQuantity {
        self.join_quantity
    }

    pub fn has_table_id(&self, table_id: TableId) -> bool {
        self.table_ids.contains(&table_id)
    }

    /// Attempt to create a new valid chain by adding a link to this chain.
    pub fn with(&self, link: &Link) -> Result<Self, &'static str> {
        Self::validate_link(link)?;
        let link_starting_table_id = link.get_starting_table_id();
        let link_ending_table_id = link.get_ending_table_id();
        if self.ending_table_id != link_starting_table_id {
            return Err("Link does not connect to chain");
        }
        if self.table_ids.contains(&link_ending_table_id) {
            return Err("Link would cause chain to intersect itself");
        }
        let mut links = self.links.clone();
        links.push(*link);
        let mut table_ids = self.table_ids.clone();
        table_ids.insert(link_ending_table_id);
        Ok(Self {
            links,
            starting_table_id: self.starting_table_id,
            ending_table_id: link_ending_table_id,
            join_quantity: self.join_quantity & link.get_join_quantity(),
            table_ids,
        })
    }

    pub fn print_tables(&self, t: impl Fn(TableId) -> String) -> String {
        let mut table_names = Vec::from([t(self.starting_table_id)]);
        for link in self.links.iter() {
            table_names.push(t(link.get_ending_table_id()));
        }
        table_names.join(" -> ")
    }
}

impl TryFrom<Vec<Link>> for Chain {
    type Error = &'static str;

    fn try_from(links: Vec<Link>) -> Result<Self, &'static str> {
        let mut iter = links.into_iter();
        let first_link_option = iter.next();
        match first_link_option {
            Some(first_link) => {
                let mut chain = Chain::new(&first_link)?;
                for link in iter {
                    chain = chain.with(&link)?;
                }
                Ok(chain)
            }
            None => Err("A chain cannot be created from an empty list of links"),
        }
    }
}
