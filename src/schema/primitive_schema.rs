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
