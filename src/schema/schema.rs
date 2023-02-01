use std::{collections::HashMap, ops::BitAnd};

use super::primitive_schema::{PrimitiveReference, PrimitiveSchema};

type TableName = String;
type ColumnName = String;
type NodeId = usize;

#[derive(Debug)]
pub struct Schema {
    pub tables: HashMap<TableName, Table>,
    pub columns: HashMap<TableName, HashMap<ColumnName, Column>>,
    pub nodes: HashMap<NodeId, Node>,
}

#[derive(Debug)]
pub struct Table {
    pub name: TableName,
}

#[derive(Debug)]
pub struct Column {
    pub name: ColumnName,
    pub node_id: NodeId,
}

#[derive(Debug)]
pub struct Node {
    pub id: NodeId,
    pub column_name: ColumnName,
    pub table_name: TableName,
    pub links: Vec<Link>,
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

#[derive(Debug, Clone, Copy)]
pub struct Link {
    pub to: NodeId,
    pub join_quantity: JoinQuantity,
    pub direction: LinkDirection,
}

mod chain {
    use std::collections::HashSet;

    use super::{JoinQuantity, Link, NodeId};
    use JoinQuantity::*;

    #[derive(Debug, Clone)]
    /// A series of zero or more connected links, along with cached information about that series.
    pub struct Chain {
        starting_node_id: NodeId,
        ending_node_id: NodeId,
        links: Vec<Link>,
        join_quantity: JoinQuantity,
        node_ids: HashSet<NodeId>,
    }

    impl Chain {
        pub fn get_starting_node_id(&self) -> NodeId {
            self.starting_node_id
        }

        pub fn get_ending_node_id(&self) -> NodeId {
            self.ending_node_id
        }

        pub fn get_links(&self) -> &[Link] {
            &self.links
        }

        pub fn is_empty(&self) -> bool {
            self.links.len() == 0
        }

        pub fn get_join_quantity(&self) -> JoinQuantity {
            self.join_quantity
        }

        pub fn has_node_id(&self, node_id: NodeId) -> bool {
            self.node_ids.contains(&node_id)
        }

        /// Attempt to add a link to this chain. If adding the link would cause the chain to
        /// intersect itself, then return None. Otherwise, return a new chain with the link added.
        pub fn with(&self, link: &Link) -> Option<Self> {
            if self.node_ids.contains(&link.to) {
                return None;
            }
            let mut links = self.links.clone();
            links.push(*link);
            let mut node_ids = self.node_ids.clone();
            node_ids.insert(link.to);
            Some(Self {
                starting_node_id: self.starting_node_id,
                ending_node_id: link.to,
                join_quantity: self.join_quantity & link.join_quantity,
                links,
                node_ids,
            })
        }
    }

    impl From<NodeId> for Chain {
        fn from(node_id: NodeId) -> Self {
            Chain {
                starting_node_id: node_id,
                ending_node_id: node_id,
                links: vec![],
                join_quantity: One,
                node_ids: HashSet::from([node_id]),
            }
        }
    }
}

use chain::Chain;

/// Keys are destination nodes, values are possible paths to that destination.
type ChainMap = HashMap<NodeId, Vec<Chain>>;

fn build_chain_map_from_chains(chains: Vec<Chain>) -> ChainMap {
    let mut map = HashMap::<NodeId, Vec<Chain>>::new();
    for chain in chains {
        if chain.is_empty() {
            continue;
        }
        map.entry(chain.get_ending_node_id())
            .or_insert_with(Vec::new)
            .push(chain);
    }
    map
}

impl Schema {
    pub fn get_chain_map(&self, from: NodeId) -> ChainMap {
        build_chain_map_from_chains(self.get_chains_from_node(from))
    }

    fn get_chains_from_node(&self, from: NodeId) -> Vec<Chain> {
        self.get_recursive_chains(Chain::from(from))
    }

    fn get_recursive_chains(&self, chain: Chain) -> Vec<Chain> {
        // Unwrap is safe because we know all the node ids in the schema are valid.
        let starting_node = self.nodes.get(&chain.get_ending_node_id()).unwrap();
        let mut chains = vec![];
        for link in starting_node.links.iter() {
            match chain.with(link) {
                None => continue,
                Some(new_chain) => {
                    chains.extend(self.get_recursive_chains(new_chain));
                }
            }
        }
        chains.push(chain);
        chains
    }
}

impl<'a> TryFrom<PrimitiveSchema> for Schema {
    type Error = &'static str;

    fn try_from(primitive_schema: PrimitiveSchema) -> Result<Schema, &'static str> {
        let mut tables = HashMap::<TableName, Table>::new();
        let mut columns = HashMap::<TableName, HashMap<ColumnName, Column>>::new();
        let mut nodes = HashMap::<NodeId, Node>::new();
        let mut max_node_id: NodeId = 0;

        for primitive_table in primitive_schema.tables.iter() {
            tables.insert(
                primitive_table.name.clone(),
                Table {
                    name: primitive_table.name.clone(),
                },
            );
            for primitive_column in primitive_table.columns.iter() {
                max_node_id += 1;
                let node_id = max_node_id;
                let column = Column {
                    name: primitive_column.name.clone(),
                    node_id,
                };
                nodes.insert(
                    node_id,
                    Node {
                        id: node_id,
                        table_name: primitive_table.name.clone(),
                        column_name: primitive_column.name.clone(),
                        links: Vec::new(),
                    },
                );
                let table_columns = columns
                    .entry(primitive_table.name.clone())
                    .or_insert_with(HashMap::new);
                table_columns.insert(column.name.clone(), column);
            }
        }

        let get_node_id = |r: &PrimitiveReference| -> Result<NodeId, &'static str> {
            let table_columns = columns.get(&r.table).ok_or("Unknown table")?;
            let column = table_columns.get(&r.column).ok_or("Unknown column")?;
            Ok(column.node_id)
        };

        let mut add_link = |node_id: NodeId, link: Link| -> Result<(), &'static str> {
            let node = nodes.get_mut(&node_id).ok_or("Unknown node")?;
            node.links.push(link);
            Ok(())
        };

        for primitive_link in primitive_schema.links.iter() {
            use JoinQuantity::*;
            let unique = primitive_link.unique;
            let from_node_id = get_node_id(&primitive_link.from)?;
            let to_node_id = get_node_id(&primitive_link.to)?;
            add_link(
                from_node_id,
                Link {
                    to: to_node_id,
                    join_quantity: One,
                    direction: LinkDirection::Forward,
                },
            )?;
            add_link(
                to_node_id,
                Link {
                    to: from_node_id,
                    join_quantity: if unique { One } else { Many },
                    direction: LinkDirection::Reverse,
                },
            )?;
        }

        Ok(Schema {
            tables,
            columns,
            nodes,
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
        let issue_id_node_id = schema
            .columns
            .get("issues")
            .unwrap()
            .get("id")
            .unwrap()
            .node_id;
        let m = schema.get_chain_map(issue_id_node_id);
        println!("{:#?}", m);
    }
}
