use querydown::{Compiler, Postgres};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn compile(schema_json: &str, dialect: &str, input: String) -> Result<String, String> {
    let dialect = match dialect {
        "postgres" => Postgres(),
        _ => return Err("Invalid dialect".to_string()),
    };
    let compiler = Compiler::new(schema_json, Box::new(dialect))?;
    compiler.compile(input.to_owned())
}
