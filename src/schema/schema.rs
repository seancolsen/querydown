use std::{collections::HashMap, ops::BitAnd};

use super::primitive_schema::{PrimitiveSchema, PrimitiveTable};

pub type TableName = String;
pub type ColumnName = String;
pub type TableId = usize;
pub type ColumnId = usize;

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

impl Schema {
    pub fn get_table(&self, table_name: &str) -> Option<&Table> {
        self.tables.get(self.table_lookup.get(table_name)?)
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
        let schema = Schema::try_from(primitive_schema);
        assert!(schema.is_ok())
    }
}
