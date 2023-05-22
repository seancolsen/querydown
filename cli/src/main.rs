use clap::Parser;
use querydown::{Compiler, Postgres};
use std::io::{self, Read};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
/// Querydown compiler
struct Args {
    #[arg(short, long)]
    /// Path to the schema JSON file
    schema: String,
    /// The querydown query to execute. If empty, stdin will be used.
    query: Option<String>,
}

/// Get the query from the CLI argument if it exists, otherwise read it from stdin
fn get_querydown_code(args: &mut Args) -> String {
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
    let querydown_code = get_querydown_code(&mut args);
    let schema_json = std::fs::read_to_string(args.schema).unwrap();
    let compiler = Compiler::new(&schema_json, Box::new(Postgres())).unwrap();
    let sql_code = compiler.compile(querydown_code).unwrap();
    println!("{sql_code}");
}
