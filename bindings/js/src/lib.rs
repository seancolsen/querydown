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
        identifier_resolution: IdentifierResolution::Strict,
    };
    let compiler = Compiler::new(schema_json, options)?;
    compiler.compile(input.to_owned())
}
