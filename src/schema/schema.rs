use std::collections::HashMap;

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

#[derive(Debug)]
pub enum JoinQuantity {
    One,
    Many,
}

#[derive(Debug)]
pub struct Link {
    pub to: NodeId,
    pub join_quantity: JoinQuantity,
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
                },
            )?;
            add_link(
                to_node_id,
                Link {
                    to: from_node_id,
                    join_quantity: if unique { One } else { Many },
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
        let schema = Schema::try_from(primitive_schema);
        println!("{:?}", schema);
        assert!(schema.is_ok());
    }
}
