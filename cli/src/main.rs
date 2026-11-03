use clap::{Args, Parser, Subcommand, ValueEnum};
use querydown::*;
use std::io::{self, Read};
use std::process::ExitCode;

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
    ///
    /// On failure, the `sql` format prints a human-readable diagnostic to stderr, while the `json`
    /// format prints `{"error": {...}}` to stdout, describing the error's kind, message, and location
    /// within the query. Either way, the exit code is 1.
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

/// A failure of the CLI itself (as opposed to a Querydown [`Error`]), e.g. an unreadable file.
fn fail(message: String) -> ExitCode {
    eprintln!("error: {message}");
    ExitCode::FAILURE
}

/// Reports a Querydown error — rendered against `source` for the `sql` format, or as JSON for the
/// `json` format — and returns the failure exit code.
fn report(error: &Error, source: &str, format: OutputFormat) -> ExitCode {
    match format {
        OutputFormat::Sql => eprintln!("{}", error.render(source)),
        OutputFormat::Json => {
            #[derive(serde::Serialize)]
            struct Output<'a> {
                error: &'a Error,
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&Output { error }).unwrap()
            );
        }
    }
    ExitCode::FAILURE
}

fn compile(args: CompileArgs) -> ExitCode {
    let querydown_code = match args.query {
        Some(query) => query,
        None => {
            let mut buffer = String::new();
            if let Err(e) = io::stdin().read_to_string(&mut buffer) {
                return fail(format!("could not read the query from stdin: {e}"));
            }
            buffer
        }
    };
    let schema_json = match std::fs::read_to_string(&args.schema) {
        Ok(json) => json,
        Err(e) => {
            return fail(format!(
                "could not read the schema file `{}`: {e}",
                args.schema
            ))
        }
    };
    let dialect: Box<dyn Dialect> = match args.dialect {
        DialectOption::Postgres => Box::new(Postgres()),
        DialectOption::Duckdb => Box::new(DuckDB()),
    };
    let options = Options {
        dialect,
        identifier_resolution: IdentifierResolution::Flexible,
    };
    let compiler = match Compiler::new(&schema_json, options) {
        Ok(compiler) => compiler,
        Err(e) => return report(&e, &schema_json, args.format),
    };
    let result = match compiler.compile(querydown_code.clone()) {
        Ok(result) => result,
        Err(e) => return report(&e, &querydown_code, args.format),
    };
    match args.format {
        OutputFormat::Sql => println!("{}", result.sql),
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result).unwrap())
        }
    }
    ExitCode::SUCCESS
}

fn introspect(args: IntrospectArgs) {
    let dialect: Box<dyn Dialect> = match args.dialect {
        DialectOption::Postgres => Box::new(Postgres()),
        DialectOption::Duckdb => Box::new(DuckDB()),
    };
    println!("{}", dialect.introspection_sql());
}

fn main() -> ExitCode {
    let args = Cli::parse();
    match args.command {
        Command::Compile(args) => compile(args),
        Command::Introspect(args) => {
            introspect(args);
            ExitCode::SUCCESS
        }
    }
}
