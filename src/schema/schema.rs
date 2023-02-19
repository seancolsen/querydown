use std::{collections::HashMap, ops::BitAnd};

use super::primitive_schema::{PrimitiveSchema, PrimitiveTable};

type TableName = String;
type ColumnName = String;
type TableId = usize;
type ColumnId = usize;

#[derive(Debug)]
pub struct Schema {
    pub tables: HashMap<TableId, Table>,
    pub table_lookup: HashMap<TableName, TableId>,
}

#[derive(Debug)]
pub struct Table {
    pub id: TableId,
    pub name: TableName,
    pub columns: HashMap<ColumnId, Column>,
    pub column_lookup: HashMap<ColumnName, ColumnId>,
    pub links: Vec<Link>,
}

#[derive(Debug)]
pub struct Column {
    pub id: ColumnId,
    pub name: ColumnName,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum JoinQuantity {
    One,
    Many,
}

use JoinQuantity::*;

impl BitAnd for JoinQuantity {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (One, One) => One,
            (One, Many) => Many,
            (Many, One) => Many,
            (Many, Many) => Many,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum LinkDirection {
    Forward,
    Reverse,
}

use LinkDirection::*;

#[derive(Debug, Clone, Copy)]
pub struct Reference {
    pub table_id: TableId,
    pub column_id: ColumnId,
}

#[derive(Debug, Clone, Copy)]
pub struct ForeignKey {
    pub base: Reference,
    pub target: Reference,
    pub unique: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct Link {
    pub foreign_key: ForeignKey,
    pub direction: LinkDirection,
}

impl Link {
    pub fn get_starting_table_id(&self) -> TableId {
        match self.direction {
            Forward => self.foreign_key.base.table_id,
            Reverse => self.foreign_key.target.table_id,
        }
    }

    pub fn get_ending_table_id(&self) -> TableId {
        match self.direction {
            Forward => self.foreign_key.target.table_id,
            Reverse => self.foreign_key.base.table_id,
        }
    }

    pub fn get_join_quantity(&self) -> JoinQuantity {
        if self.foreign_key.unique {
            One
        } else {
            Many
        }
    }
}

mod chain {
    use super::{JoinQuantity, Link, TableId};
    use std::collections::HashSet;

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
}

use chain::Chain;

/// Keys are destination tables, values are possible paths to that destination.
type ChainMap = HashMap<TableId, Vec<Chain>>;

fn build_chain_map_from_chains(chains: Vec<Chain>) -> ChainMap {
    let mut map = HashMap::<TableId, Vec<Chain>>::new();
    for chain in chains {
        map.entry(chain.get_ending_table_id())
            .or_insert_with(Vec::new)
            .push(chain);
    }
    map
}

#[derive(Debug)]
enum ChainSeed {
    Table(TableId),
    Chain(Chain),
}

impl Schema {
    pub fn has_table(&self, table_name: &str) -> bool {
        self.table_lookup.contains_key(table_name)
    }

    pub fn get_table(&self, table_name: &str) -> Option<&Table> {
        self.tables.get(self.table_lookup.get(table_name)?)
    }

    pub fn get_chain_map(&self, from: TableId) -> ChainMap {
        build_chain_map_from_chains(self.get_chains_from_table(from))
    }

    fn get_chains_from_table(&self, from: TableId) -> Vec<Chain> {
        self.get_recursive_chains(ChainSeed::Table(from))
    }

    fn get_recursive_chains(&self, seed: ChainSeed) -> Vec<Chain> {
        let mut chains = vec![];
        let starting_table_id = match &seed {
            ChainSeed::Table(table_id) => *table_id,
            ChainSeed::Chain(chain) => chain.get_ending_table_id(),
        };
        // Unwrap is safe because we know all the table ids in the schema are valid.
        let starting_table = self.tables.get(&starting_table_id).unwrap();
        for link in starting_table.links.iter() {
            let new_chain_attempt = match &seed {
                ChainSeed::Table(_) => {
                    Chain::new(link).and_then(|c| {
                        if c.get_starting_table_id() == starting_table_id {
                            Ok(c)
                        } else {
                            // This should never happen if we have a valid schema, but we handle
                            // anyway just for code cleanliness.
                            Err("Starting link does not connect to starting table")
                        }
                    })
                }
                ChainSeed::Chain(chain) => chain.clone().with(link),
            };
            match new_chain_attempt {
                Err(_) => continue,
                Ok(new_chain) => {
                    chains.extend(self.get_recursive_chains(ChainSeed::Chain(new_chain)));
                }
            }
        }
        if let ChainSeed::Chain(chain) = seed {
            chains.push(chain);
        }
        chains
    }
}

fn make_table(id: TableId, primitive_table: PrimitiveTable) -> Table {
    let mut columns = HashMap::<ColumnId, Column>::new();
    let mut max_column_id: ColumnId = 0;
    for primitive_column in primitive_table.columns {
        max_column_id += 1;
        let column = Column {
            id: max_column_id,
            name: primitive_column.name,
        };
        columns.insert(max_column_id, column);
    }
    let column_lookup = columns
        .iter()
        .map(|(id, column)| (column.name.clone(), *id))
        .collect();
    Table {
        id,
        name: primitive_table.name,
        columns,
        column_lookup,
        links: vec![],
    }
}

impl TryFrom<PrimitiveSchema> for Schema {
    type Error = String;

    fn try_from(primitive_schema: PrimitiveSchema) -> Result<Schema, String> {
        let mut max_table_id: TableId = 0;
        let mut tables = HashMap::<TableId, Table>::new();
        for primitive_table in primitive_schema.tables {
            max_table_id += 1;
            let table = make_table(max_table_id, primitive_table);
            tables.insert(max_table_id, table);
        }

        let table_lookup: HashMap<TableName, TableId> = tables
            .iter()
            .map(|(id, table)| (table.name.clone(), *id))
            .collect();

        let foreign_keys: Vec<ForeignKey> = {
            let get_table_by_name = |name: &String| -> Result<&Table, String> {
                let table_id = table_lookup
                    .get(name)
                    .ok_or_else(|| format!("Unknown table: {}", name))?;
                let table = tables
                    .get(table_id)
                    .ok_or_else(|| format!("Table not found by id: {}", table_id))?;
                Ok(table)
            };
            let get_column_id_by_name =
                |table: &Table, name: &String| -> Result<ColumnId, String> {
                    let column_id = table
                        .column_lookup
                        .get(name)
                        .ok_or_else(|| format!("Unknown column: {}", name))?;
                    Ok(*column_id)
                };
            let mut foreign_keys: Vec<ForeignKey> = vec![];
            for primitive_link in primitive_schema.links {
                let base_table = get_table_by_name(&primitive_link.from.table)?;
                let base_column_id =
                    get_column_id_by_name(base_table, &primitive_link.from.column)?;
                let target_table = get_table_by_name(&primitive_link.to.table)?;
                let target_column_id =
                    get_column_id_by_name(target_table, &primitive_link.to.column)?;
                foreign_keys.push(ForeignKey {
                    base: Reference {
                        table_id: base_table.id,
                        column_id: base_column_id,
                    },
                    target: Reference {
                        table_id: target_table.id,
                        column_id: target_column_id,
                    },
                    unique: primitive_link.unique,
                });
            }
            foreign_keys
        };

        for foreign_key in foreign_keys {
            let base_table = tables.get_mut(&foreign_key.base.table_id).unwrap();
            base_table.links.push(Link {
                foreign_key: foreign_key.clone(),
                direction: Forward,
            });
            let target_table = tables.get_mut(&foreign_key.target.table_id).unwrap();
            target_table.links.push(Link {
                foreign_key,
                direction: Reverse,
            });
        }

        Ok(Schema {
            tables,
            table_lookup,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::test_utils::get_test_resource;

    use super::*;

    #[test]
    fn test_schema_from_primitive_schema() {
        let primitive_schema: PrimitiveSchema =
            serde_json::from_str(&get_test_resource("issue_schema.json")).unwrap();
        let schema = Schema::try_from(primitive_schema).unwrap();
        let issues_table_id = schema.table_lookup.get("issues").unwrap();
        let m = schema.get_chain_map(*issues_table_id);
        let get_table_name = |id| schema.tables.get(&id).unwrap().name.clone();
        for (table_id, chains) in m.iter() {
            println!("TO TABLE: {}", get_table_name(*table_id));
            for chain in chains {
                println!("  Chain: {:?}", chain.print_tables(get_table_name));
            }
        }
    }
}
