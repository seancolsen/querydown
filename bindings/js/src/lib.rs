use querydown::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn compile(schema_json: &str, dialect: &str, input: String) -> Result<String, String> {
    let dialect = match dialect {
        "postgres" => Box::new(Postgres()),
        _ => return Err("Invalid dialect".to_string()),
    };
    let options = Options {
        dialect,
        identifier_resolution: IdentifierResolution::Flexible,
    };
    let compiler = Compiler::new(schema_json, options)?;
    let result = compiler.compile(input.to_owned())?;
    // Return a JSON object `{ sql, columnAnnotations }` serialized as a string.
    serde_json::to_string(&result).map_err(|e| e.to_string())
}
