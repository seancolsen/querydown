use querydown::*;
use wasm_bindgen::prelude::*;

fn parse_dialect(dialect: &str) -> Result<Box<dyn Dialect>, String> {
    match dialect {
        "postgres" => Ok(Box::new(Postgres())),
        "duckdb" => Ok(Box::new(DuckDB())),
        _ => Err("Invalid dialect".to_string()),
    }
}

/// Return the static SQL query that introspects a database of the given dialect to produce the
/// schema JSON consumed by [`compile`]. The query yields a single row with a single column
/// containing the JSON.
#[wasm_bindgen]
pub fn introspection_sql(dialect: &str) -> Result<String, String> {
    Ok(parse_dialect(dialect)?.introspection_sql().to_string())
}

#[wasm_bindgen]
pub fn compile(schema_json: &str, dialect: &str, input: String) -> Result<String, String> {
    let dialect = parse_dialect(dialect)?;
    let options = Options {
        dialect,
        identifier_resolution: IdentifierResolution::Flexible,
    };
    let compiler = Compiler::new(schema_json, options)?;
    let result = compiler.compile(input.to_owned())?;
    // Return a JSON object `{ sql, columnAnnotations }` serialized as a string.
    serde_json::to_string(&result).map_err(|e| e.to_string())
}
