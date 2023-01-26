use crate::schema::*;

pub fn issue_schema() -> Schema {
    Schema {
        tables: vec![
            Table {
                name: "users".to_string(),
                columns: vec![
                    Column::from("id"),
                    Column::from("username"),
                    Column::from("email"),
                ],
            },
            Table {
                name: "issues".to_string(),
                columns: vec![
                    Column::from("id"),
                    Column::from("title"),
                    Column::from("description"),
                    Column::from("created_at"),
                    Column::from("created_by"),
                    Column::from("status"),
                    Column::from("project"),
                    Column::from("duplicate_of"),
                ],
            },
            Table {
                name: "assignments".to_string(),
                columns: vec![
                    Column::from("id"),
                    Column::from("issue"),
                    Column::from("user"),
                ],
            },
            Table {
                name: "blocks".to_string(),
                columns: vec![
                    Column::from("id"),
                    Column::from("blocker"),
                    Column::from("blocking"),
                ],
            },
            Table {
                name: "projects".to_string(),
                columns: vec![Column::from("id"), Column::from("title")],
            },
            Table {
                name: "labels".to_string(),
                columns: vec![Column::from("id"), Column::from("title")],
            },
            Table {
                name: "issue_labels".to_string(),
                columns: vec![
                    Column::from("id"),
                    Column::from("issue"),
                    Column::from("label"),
                ],
            },
            Table {
                name: "comments".to_string(),
                columns: vec![
                    Column::from("id"),
                    Column::from("issue"),
                    Column::from("user"),
                    Column::from("body"),
                    Column::from("created_at"),
                ],
            },
        ],
        relationships: vec![
            Relationship {
                from_table: "issues".to_string(),
                to_table: "projects".to_string(),
                columns: vec![RelationshipColumn::from(("project", "id"))],
            },
            Relationship {
                from_table: "issues".to_string(),
                to_table: "users".to_string(),
                columns: vec![RelationshipColumn::from(("created_by", "id"))],
            },
            Relationship {
                from_table: "issues".to_string(),
                to_table: "issues".to_string(),
                columns: vec![RelationshipColumn::from(("duplicate_of", "id"))],
            },
            Relationship {
                from_table: "assignments".to_string(),
                to_table: "issues".to_string(),
                columns: vec![RelationshipColumn::from(("issue", "id"))],
            },
            Relationship {
                from_table: "assignments".to_string(),
                to_table: "users".to_string(),
                columns: vec![RelationshipColumn::from(("user", "id"))],
            },
            Relationship {
                from_table: "blocks".to_string(),
                to_table: "issues".to_string(),
                columns: vec![RelationshipColumn::from(("blocker", "id"))],
            },
            Relationship {
                from_table: "blocks".to_string(),
                to_table: "issues".to_string(),
                columns: vec![RelationshipColumn::from(("blocking", "id"))],
            },
            Relationship {
                from_table: "issue_labels".to_string(),
                to_table: "issues".to_string(),
                columns: vec![RelationshipColumn::from(("issue", "id"))],
            },
            Relationship {
                from_table: "issue_labels".to_string(),
                to_table: "labels".to_string(),
                columns: vec![RelationshipColumn::from(("label", "id"))],
            },
            Relationship {
                from_table: "comments".to_string(),
                to_table: "issues".to_string(),
                columns: vec![RelationshipColumn::from(("issue", "id"))],
            },
            Relationship {
                from_table: "comments".to_string(),
                to_table: "users".to_string(),
                columns: vec![RelationshipColumn::from(("user", "id"))],
            },
        ],
    }
}
