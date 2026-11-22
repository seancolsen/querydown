//! The compiler for [Querydown](https://querydown.org), a language for writing relational database
//! queries that compile to SQL, for PostgreSQL and DuckDB.
//!
//! Create a [`Compiler`] from a JSON description of your database (which Querydown can
//! [generate from your database](https://querydown.org/docs/setup/introspection)), and then compile
//! queries with it:
//!
//! ```no_run
//! use querydown::{Compiler, IdentifierResolution, Options, Postgres};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let schema_json = std::fs::read_to_string("schema.json")?;
//! let options = Options {
//!     dialect: Box::new(Postgres()),
//!     identifier_resolution: IdentifierResolution::Flexible,
//!     limit: None,
//!     offset: None,
//! };
//! let compiler = Compiler::new(&schema_json, options)?;
//!
//! let result = compiler.compile("#issues status:open $title $author.username".to_string())?;
//! println!("{}", result.sql);
//! # Ok(())
//! # }
//! ```
//!
//! The compiler doesn't connect to a database: what you do with the SQL is up to you. The
//! [Rust guide](https://querydown.org/docs/setup/rust) shows how to read the annotations that a
//! query carries, handle errors, and offer completions in an editor. The
//! [language reference](https://querydown.org/docs/language) describes Querydown itself.

// `Error` is deliberately a plain struct with public fields (e.g. `err.span`) rather than a boxed
// handle, for ease of use. It's larger than Clippy's threshold for error types, but errors are the
// cold path, so the cost of moving it around is immaterial.
#![allow(clippy::result_large_err)]

mod compiler;
mod errors;
mod options;
mod schema;
mod sql;
#[cfg(test)]
mod tests;
mod utils;

pub use compiler::{completion, CompileResult, Compiler};
// The error type is defined in the parser crate so that parsing and compilation share it; see the
// `error` module documentation for an overview of how errors are reported.
pub use options::{IdentifierResolution, Options};
pub use querydown_parser::error;
pub use querydown_parser::error::{Error, ErrorKind, Label, Position, Section, Span};
pub use sql::{Dialect, DuckDB, Postgres};

// Re-exported from the parser crate so that consumers of the `querydown` crate can parse Querydown
// code (in whole or in sections) and inspect or assemble the resulting AST without taking a direct
// dependency on `querydown-parser`.
pub use querydown_parser::ast::AnnotationValue;
pub use querydown_parser::{
    ast, parse, parse_conditions, parse_definitions, parse_display, parse_sorting, parse_table_name,
};
// Re-exported so that consumers can classify Querydown source for syntax highlighting.
pub use querydown_parser::highlight;
