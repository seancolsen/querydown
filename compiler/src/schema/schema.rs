use std::collections::{
    hash_map::Entry::{Occupied, Vacant},
    HashMap,
};

use itertools::Itertools;

use crate::errors::msg;

use super::{
    chain::{Chain, ChainIntersecting},
    links::{
        ForeignKey, ForwardLinkToOne, Link, LinkToOne, MultiLink, Reference, ReverseLinkToMany,
        ReverseLinkToOne,
    },
    primitive_schema::{PrimitiveSchema, PrimitiveTable},
};

pub type TableName = String;
pub type ColumnName = String;
pub type TableId = usize;
pub type ColumnId = usize;

#[derive(Debug)]
pub struct Schema {
    pub tables: HashMap<TableId, Table>,
    pub table_lookup: HashMap<TableName, TableId>,
}

impl Schema {
    pub fn get_ideal_alias_for_link_to_one(&self, link: &LinkToOne) -> &str {
        // The `unwrap` calls within this fn are safe because we know all links within the schema
        // are valid.
        match link {
            LinkToOne::ForwardLinkToOne(forward_link) => {
                let target_table_id = forward_link.target.table_id;
                let base_table_id = forward_link.base.table_id;
                let base_table = self.tables.get(&base_table_id).unwrap();
                let links_which_point_to_the_same_target_table = base_table
                    .forward_links_to_one
                    .values()
                    .filter(|&l| l.target.table_id == target_table_id);
                // TODO_PERF: we don't need to consume the whole iterator here just to see if the
                // count is greater than 1. We can stop when we get a count of 2.
                let is_duplicate = links_which_point_to_the_same_target_table.count() > 1;
                if is_duplicate {
                    &base_table
                        .columns
                        .get(&forward_link.base.column_id)
                        .unwrap()
                        .name
                } else {
                    &self.tables.get(&target_table_id).unwrap().name
                }
            }
            LinkToOne::ReverseLinkToOne(reverse_link) => {
                let table_id = reverse_link.get_end().table_id;
                let table = self.tables.get(&table_id).unwrap();
                &table.name
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum ChainSearchBase {
    Chain(Chain<MultiLink>),
    TableId(TableId),
}

impl ChainSearchBase {
    pub fn len(&self) -> usize {
        match self {
            ChainSearchBase::Chain(chain) => chain.len(),
            ChainSearchBase::TableId(_) => 0,
        }
    }

    pub fn get_base_table_id(&self) -> TableId {
        match self {
            Self::Chain(chain) => chain.get_ending_table_id(),
            Self::TableId(id) => *id,
        }
    }

    pub fn get_ending_table_id(&self) -> Option<usize> {
        match self {
            Self::Chain(chain) => Some(chain.get_ending_table_id()),
            Self::TableId(_) => None,
        }
    }

    pub fn try_append_into_chain(self, link: MultiLink) -> Result<Chain<MultiLink>, &'static str> {
        match self {
            Self::Chain(mut chain) => {
                chain.try_append(link)?;
                Ok(chain)
            }
            Self::TableId(table_id) => {
                if table_id != link.get_start().table_id {
                    return Err("Link does not connect to starting table");
                }
                Chain::try_new(link, ChainIntersecting::Disallowed)
            }
        }
    }
}

#[derive(Debug)]
pub struct Table {
    pub id: TableId,
    pub name: TableName,
    pub columns: HashMap<ColumnId, Column>,
    pub column_lookup: HashMap<ColumnName, ColumnId>,
    /// Keys are starting column ids in this table
    pub forward_links_to_one: HashMap<ColumnId, ForwardLinkToOne>,
    /// Keys are ending table ids in the other table
    pub reverse_links_to_one: HashMap<TableId, Vec<ReverseLinkToOne>>,
    /// Keys are ending table ids in the other table
    pub reverse_links_to_many: HashMap<TableId, Vec<ReverseLinkToMany>>,
}

impl Table {
    pub fn get_links(&self) -> impl Iterator<Item = MultiLink> + '_ {
        let forward_links_to_one = self
            .forward_links_to_one
            .values()
            .copied()
            .map(MultiLink::ForwardLinkToOne);
        let reverse_links_to_many = self
            .reverse_links_to_many
            .values()
            .flatten()
            .copied()
            .map(MultiLink::ReverseLinkToMany);
        let reverse_links_to_one = self
            .reverse_links_to_one
            .values()
            .flatten()
            .copied()
            .map(MultiLink::ReverseLinkToOne);
        forward_links_to_one
            .chain(reverse_links_to_many)
            .chain(reverse_links_to_one)
    }
}

#[derive(Debug)]
pub struct Column {
    pub id: ColumnId,
    pub name: ColumnName,
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
        forward_links_to_one: HashMap::new(),
        reverse_links_to_one: HashMap::new(),
        reverse_links_to_many: HashMap::new(),
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
            let base = foreign_key.base;
            let target = foreign_key.target;

            let base_table = tables.get_mut(&base.table_id).unwrap();
            match base_table.forward_links_to_one.entry(base.column_id) {
                Occupied(_) => {
                    return Err(msg::multiple_fk_from_col());
                }
                Vacant(e) => {
                    e.insert(ForwardLinkToOne::from(foreign_key));
                }
            }

            let target_table = tables.get_mut(&target.table_id).unwrap();
            if foreign_key.unique {
                target_table
                    .reverse_links_to_one
                    .entry(base.table_id)
                    .or_default()
                    .push(ReverseLinkToOne::from(foreign_key))
            } else {
                target_table
                    .reverse_links_to_many
                    .entry(base.table_id)
                    .or_default()
                    .push(ReverseLinkToMany::from(foreign_key));
            }
        }

        Ok(Schema {
            tables,
            table_lookup,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::get_test_resource;

    use super::*;

    #[test]
    fn test_schema_from_primitive_schema() {
        let primitive_schema: PrimitiveSchema =
            serde_json::from_str(&get_test_resource("issue_schema.json")).unwrap();
        let schema = Schema::try_from(primitive_schema);
        assert!(schema.is_ok())
    }
}
