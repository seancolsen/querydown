mod compiler;
mod errors;
mod options;
mod schema;
mod sql;
#[cfg(test)]
mod tests;
mod utils;

pub use compiler::{CompileResult, Compiler};
pub use options::{IdentifierResolution, Options};
pub use sql::{Dialect, DuckDB, Postgres};

// Re-exported from the parser crate so that consumers of the `querydown` crate can parse Querydown
// code (in whole or in sections) and inspect or assemble the resulting AST without taking a direct
// dependency on `querydown-parser`.
pub use querydown_parser::ast::AnnotationValue;
pub use querydown_parser::{
    ast, parse, parse_conditions, parse_definitions, parse_display, parse_sorting,
};
