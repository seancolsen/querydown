use serde::{Deserialize, Deserializer};

#[derive(Debug, Deserialize)]
pub struct PrimitiveSchema {
    pub tables: Vec<PrimitiveTable>,
    pub links: Vec<PrimitiveLink>,
}

#[derive(Debug, Deserialize)]
pub struct PrimitiveTable {
    pub name: String,
    /// The schema the table is in. Absent for a table in the current schema.
    #[serde(default)]
    pub schema: SchemaPath,
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
    /// The schema of `table`. Absent for a table in the current schema.
    #[serde(default)]
    pub schema: SchemaPath,
    pub column: String,
}

#[derive(Debug, Deserialize)]
pub struct PrimitiveLink {
    pub from: PrimitiveReference,
    pub to: PrimitiveReference,
    pub unique: bool,
}

/// The schema that a table is in, as the names of its schemas, outermost first: `["sales"]`, or
/// `["archive", "main"]` for a schema nested in another (as a DuckDB database's schemas are nested in
/// the database). Empty for the current schema.
///
/// In the JSON, this is a list of names, or a string as shorthand for a list of one name. A `null`,
/// an empty list, or an empty string is the current schema.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SchemaPath(pub Vec<String>);

impl<'de> Deserialize<'de> for SchemaPath {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Written {
            One(String),
            Many(Vec<String>),
        }
        Ok(match Option::<Written>::deserialize(deserializer)? {
            None => SchemaPath::default(),
            Some(Written::One(name)) if name.is_empty() => SchemaPath::default(),
            Some(Written::One(name)) => SchemaPath(vec![name]),
            Some(Written::Many(names)) => SchemaPath(names),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema_of(json: &str) -> SchemaPath {
        serde_json::from_str::<PrimitiveTable>(json).unwrap().schema
    }

    #[test]
    fn test_schema_path_forms() {
        let path = |names: &[&str]| SchemaPath(names.iter().map(|n| n.to_string()).collect());
        assert_eq!(schema_of(r#"{"name":"t","columns":[]}"#), path(&[]));
        assert_eq!(
            schema_of(r#"{"name":"t","schema":null,"columns":[]}"#),
            path(&[])
        );
        assert_eq!(
            schema_of(r#"{"name":"t","schema":"","columns":[]}"#),
            path(&[])
        );
        assert_eq!(
            schema_of(r#"{"name":"t","schema":[],"columns":[]}"#),
            path(&[])
        );
        assert_eq!(
            schema_of(r#"{"name":"t","schema":"s","columns":[]}"#),
            path(&["s"])
        );
        assert_eq!(
            schema_of(r#"{"name":"t","schema":["a","b"],"columns":[]}"#),
            path(&["a", "b"])
        );
        assert!(
            serde_json::from_str::<PrimitiveTable>(r#"{"name":"t","schema":1,"columns":[]}"#)
                .is_err()
        );
    }
}
