use clap::Parser;
use dialects::postgres::Postgres;

use crate::compiler::Compiler;
use std::io::{self, Read};

mod compiler;
mod dialects;
mod error;
mod parsing;
mod schema;
mod sql_tree;
pub mod syntax_tree;
mod tests;
mod tokens;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
/// Lower Query Language (LQL) compiler
struct Args {
    #[arg(short, long)]
    /// Path to the schema JSON file
    schema: String,
    /// The LQL query to execute. If empty, stdin will be used.
    query: Option<String>,
}

/// Get the query from the CLI argument if it exists, otherwise read it from stdin
fn get_lql_code(args: &mut Args) -> String {
    if let Some(query) = std::mem::take(&mut args.query) {
        query
    } else {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer).unwrap();
        buffer
    }
}

fn main() -> () {
    let mut args = Args::parse();
    let lql_code = get_lql_code(&mut args);
    let schema_json = std::fs::read_to_string(args.schema).unwrap();
    let compiler = Compiler::new(&schema_json, Postgres()).unwrap();
    let sql_code = compiler.compile(lql_code).unwrap();
    println!("{sql_code}");
}
