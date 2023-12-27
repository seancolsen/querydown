use clap::{Args, Parser, Subcommand};
use querydown::*;
use std::io::{self, Read};

/// Querydown compiler
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Compile Querydown code to SQL
    Compile(CompileArgs),
    /// Analyze a database to generate a schema JSON file
    Introspect,
}

#[derive(Debug, Args)]
struct CompileArgs {
    /// Path to the schema JSON file
    #[arg(short, long)]
    schema: String,
    /// The querydown query to execute. If empty, stdin will be used.
    query: Option<String>,
}

fn get_stdin() -> String {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer).unwrap();
    buffer
}

fn compile(args: CompileArgs) {
    let querydown_code = args.query.unwrap_or_else(get_stdin);
    let schema_json = std::fs::read_to_string(args.schema).unwrap();
    let options = Options {
        dialect: Box::new(Postgres()),
        identifier_resolution: IdentifierResolution::Flexible,
    };
    let compiler = Compiler::new(&schema_json, options).unwrap();
    let sql_code = compiler.compile(querydown_code).unwrap();
    println!("{sql_code}");
}

fn introspect() {
    todo!()
}

fn main() -> () {
    let args = Cli::parse();
    match args.command {
        Command::Compile(args) => compile(args),
        Command::Introspect => introspect(),
    }
}
