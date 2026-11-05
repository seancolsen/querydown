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

pub use compiler::{CompileResult, Compiler};
// The error type is defined in the parser crate so that parsing and compilation share it; see the
// `error` module documentation for an overview of how errors are reported.
pub use options::{IdentifierResolution, Options};
pub use querydown_parser::error;
pub use querydown_parser::error::{Error, ErrorKind, Label, Position, Span};
pub use sql::{Dialect, DuckDB, Postgres};

// Re-exported from the parser crate so that consumers of the `querydown` crate can parse Querydown
// code (in whole or in sections) and inspect or assemble the resulting AST without taking a direct
// dependency on `querydown-parser`.
pub use querydown_parser::ast::AnnotationValue;
pub use querydown_parser::{
    ast, parse, parse_conditions, parse_definitions, parse_display, parse_sorting,
};
// Re-exported so that consumers can classify Querydown source for syntax highlighting.
pub use querydown_parser::highlight;
