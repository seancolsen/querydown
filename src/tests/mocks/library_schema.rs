use crate::schema::*;

pub fn library_schema() -> Schema {
    Schema {
        tables: vec![
            Table {
                name: "patron".to_string(),
                columns: vec![Column::from("id"), Column::from("name")],
            },
            Table {
                name: "email".to_string(),
                columns: vec![
                    Column::from("id"),
                    Column::from("patron"),
                    Column::from("email"),
                ],
            },
            Table {
                name: "patron_tag".to_string(),
                columns: vec![
                    Column::from("id"),
                    Column::from("patron"),
                    Column::from("tag"),
                ],
            },
            Table {
                name: "tag".to_string(),
                columns: vec![Column::from("id"), Column::from("name")],
            },
            Table {
                name: "checkout".to_string(),
                columns: vec![
                    Column::from("id"),
                    Column::from("item"),
                    Column::from("patron"),
                    Column::from("out_date"),
                    Column::from("due_date"),
                    Column::from("in_date"),
                ],
            },
            Table {
                name: "item".to_string(),
                columns: vec![Column::from("id"), Column::from("publication")],
            },
            Table {
                name: "publication".to_string(),
                columns: vec![
                    Column::from("id"),
                    Column::from("title"),
                    Column::from("year"),
                    Column::from("format"),
                    Column::from("author"),
                    Column::from("publisher"),
                ],
            },
            Table {
                name: "author".to_string(),
                columns: vec![
                    Column::from("id"),
                    Column::from("name"),
                    Column::from("birth_date"),
                    Column::from("death_date"),
                ],
            },
            Table {
                name: "publisher".to_string(),
                columns: vec![Column::from("id"), Column::from("name")],
            },
        ],
        relationships: vec![
            Relationship {
                from_table: "email".to_string(),
                to_table: "patron".to_string(),
                columns: vec![RelationshipColumn::from(("patron", "id"))],
            },
            Relationship {
                from_table: "patron_tag".to_string(),
                to_table: "patron".to_string(),
                columns: vec![RelationshipColumn::from(("patron", "id"))],
            },
            Relationship {
                from_table: "patron_tag".to_string(),
                to_table: "tag".to_string(),
                columns: vec![RelationshipColumn::from(("tag", "id"))],
            },
            Relationship {
                from_table: "checkout".to_string(),
                to_table: "item".to_string(),
                columns: vec![RelationshipColumn::from(("item", "id"))],
            },
            Relationship {
                from_table: "checkout".to_string(),
                to_table: "patron".to_string(),
                columns: vec![RelationshipColumn::from(("patron", "id"))],
            },
            Relationship {
                from_table: "item".to_string(),
                to_table: "publication".to_string(),
                columns: vec![RelationshipColumn::from(("publication", "id"))],
            },
            Relationship {
                from_table: "publication".to_string(),
                to_table: "author".to_string(),
                columns: vec![RelationshipColumn::from(("author", "id"))],
            },
            Relationship {
                from_table: "publication".to_string(),
                to_table: "publisher".to_string(),
                columns: vec![RelationshipColumn::from(("publisher", "id"))],
            },
        ],
    }
}
