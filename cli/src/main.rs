use clap::{Args, Parser, Subcommand, ValueEnum};
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
    /// Print the SQL query that introspects a database to generate its schema JSON.
    ///
    /// The printed query yields a single row with a single column containing the schema JSON.
    /// Run it against your database, then pass the result to `compile --schema`.
    Introspect(IntrospectArgs),
}

#[derive(Debug, Args)]
struct IntrospectArgs {
    /// The SQL dialect to target.
    #[arg(short, long, value_enum, default_value_t = DialectOption::Postgres)]
    dialect: DialectOption,
}

#[derive(Debug, Args)]
struct CompileArgs {
    /// Path to the schema JSON file
    #[arg(short, long)]
    schema: String,
    /// Output format. `sql` prints only the generated SQL. `json` prints a JSON object containing
    /// the SQL along with the column annotation.
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Sql)]
    format: OutputFormat,
    /// The SQL dialect to target.
    #[arg(short, long, value_enum, default_value_t = DialectOption::Postgres)]
    dialect: DialectOption,
    /// The querydown query to execute. If empty, stdin will be used.
    query: Option<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Sql,
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum DialectOption {
    Postgres,
    Duckdb,
}

fn get_stdin() -> String {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer).unwrap();
    buffer
}

fn compile(args: CompileArgs) {
    let querydown_code = args.query.unwrap_or_else(get_stdin);
    let schema_json = std::fs::read_to_string(args.schema).unwrap();
    let dialect: Box<dyn Dialect> = match args.dialect {
        DialectOption::Postgres => Box::new(Postgres()),
        DialectOption::Duckdb => Box::new(DuckDB()),
    };
    let options = Options {
        dialect,
        identifier_resolution: IdentifierResolution::Flexible,
    };
    let compiler = Compiler::new(&schema_json, options).unwrap();
    let result = compiler.compile(querydown_code).unwrap();
    match args.format {
        OutputFormat::Sql => println!("{}", result.sql),
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result).unwrap())
        }
    }
}

fn introspect(args: IntrospectArgs) {
    let dialect: Box<dyn Dialect> = match args.dialect {
        DialectOption::Postgres => Box::new(Postgres()),
        DialectOption::Duckdb => Box::new(DuckDB()),
    };
    println!("{}", dialect.introspection_sql());
}

fn main() {
    let args = Cli::parse();
    match args.command {
        Command::Compile(args) => compile(args),
        Command::Introspect(args) => introspect(args),
    }
}
