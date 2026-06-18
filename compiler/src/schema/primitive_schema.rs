use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct PrimitiveSchema {
    pub tables: Vec<PrimitiveTable>,
    pub links: Vec<PrimitiveLink>,
}

#[derive(Debug, Deserialize)]
pub struct PrimitiveTable {
    pub name: String,
    pub columns: Vec<PrimitiveColumn>,
}

#[derive(Debug, Deserialize)]
pub struct PrimitiveColumn {
    pub name: String,
    /// The column's data type, as a free-form string (e.g. `"text"`, `"integer"`, `"text[]"`).
    /// Optional for backward compatibility; an absent or unrecognized type is treated as
    /// [`ValueType::Unknown`](crate::schema::ValueType).
    #[serde(default)]
    pub r#type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PrimitiveReference {
    pub table: String,
    pub column: String,
}

#[derive(Debug, Deserialize)]
pub struct PrimitiveLink {
    pub from: PrimitiveReference,
    pub to: PrimitiveReference,
    pub unique: bool,
}
