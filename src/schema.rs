#[derive(Debug, Clone, PartialEq)]
pub struct Schema {
    pub tables: Vec<Table>,
    pub relationships: Vec<Relationship>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub name: String,
    pub columns: Vec<Column>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    pub name: String,
}

impl From<String> for Column {
    fn from(name: String) -> Self {
        Column { name }
    }
}

impl From<&str> for Column {
    fn from(name: &str) -> Self {
        Column {
            name: name.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Relationship {
    pub from_table: String,
    pub to_table: String,
    /// This is a Vec so that we can support multi-column foreign keys.
    pub columns: Vec<RelationshipColumn>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RelationshipColumn {
    pub from_column: String,
    pub to_column: String,
}

impl From<(&str, &str)> for RelationshipColumn {
    fn from((from_column, to_column): (&str, &str)) -> Self {
        RelationshipColumn {
            from_column: from_column.to_string(),
            to_column: to_column.to_string(),
        }
    }
}
