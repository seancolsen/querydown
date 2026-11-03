// `Error` is deliberately a plain struct with public fields (e.g. `err.span`) rather than a boxed
// handle, for ease of use. It's larger than Clippy's threshold for error types, but errors are the
// cold path, so the cost of moving it around is immaterial.
#![allow(clippy::result_large_err)]

mod diagnose;
mod labels;
mod parser;
mod render;

pub mod ast;
pub mod error;
pub mod tokens;

pub use error::{Error, ErrorKind, Label, Position, Span};

use chumsky::prelude::*;
use parser::{conditions, definitions, pad, query, result_columns, sorting, Psr};

/// Parses a complete Querydown query — definitions, base table, and transformations — into an
/// [`ast::Query`].
pub fn parse(input: &str) -> Result<ast::Query, Error> {
    run(query(), input)
}

/// Parses just the definitions section of a query (constants, functions, custom comparisons, and
/// computed columns) in isolation, without a base table or any transformations.
///
/// This is one of four section parsers — alongside [`parse_conditions`], [`parse_sorting`], and
/// [`parse_display`] — that let a consumer parse each part of a query independently and then
/// reassemble them via [`ast::Query::from_parts`] before compiling. Parsing the sections separately
/// keeps them isolated: text entered for one section cannot introduce constructs that belong to
/// another (e.g. a filtering section cannot smuggle in constant definitions).
pub fn parse_definitions(input: &str) -> Result<ast::Definitions, Error> {
    run(definitions(), input)
}

/// Parses just the filtering section of a query in isolation: a whitespace-separated sequence of
/// boolean expressions, combined with `AND`, into an [`ast::ConditionSet`].
///
/// See [`parse_definitions`] for how the section parsers fit together.
pub fn parse_conditions(input: &str) -> Result<ast::ConditionSet, Error> {
    run(conditions(), input)
}

/// Parses just the sorting section of a query in isolation: a sequence of standalone `\\` sorting
/// expressions into a `Vec<`[`ast::SortExpr`]`>`.
///
/// See [`parse_definitions`] for how the section parsers fit together.
pub fn parse_sorting(input: &str) -> Result<Vec<ast::SortExpr>, Error> {
    run(sorting(), input)
}

/// Parses just the display section of a query in isolation: the `$`-prefixed result column
/// statements into a `Vec<`[`ast::ResultColumnStatement`]`>`.
///
/// See [`parse_definitions`] for how the section parsers fit together.
pub fn parse_display(input: &str) -> Result<Vec<ast::ResultColumnStatement>, Error> {
    run(result_columns(), input)
}

/// Runs a parser over the whole of `input`, allowing surrounding whitespace/comments and requiring
/// that the entire input is consumed. A failure is reported as an [`ErrorKind::Syntax`] [`Error`]
/// locating the problem within `input`.
fn run<'src, T>(parser: impl Psr<'src, T>, input: &'src str) -> Result<T, Error> {
    pad()
        .ignore_then(parser)
        .then_ignore(pad())
        .then_ignore(end())
        .parse(input)
        .into_result()
        .map_err(|errors| diagnose::diagnose(input, errors))
}

#[cfg(test)]
mod tests {
    use super::ast::*;
    use super::*;

    #[test]
    fn test_parse_sections_reassemble_into_whole_query() {
        // Parsing each section in isolation and reassembling via `Query::from_parts` yields the same
        // AST as parsing the equivalent whole query in one shot.
        let definitions = parse_definitions("@user_id = 1234").unwrap();
        let conditions = parse_conditions("author:@user_id").unwrap();
        let sorting = parse_sorting(r"\\created_at \d").unwrap();
        let display = parse_display("$id $title").unwrap();
        let reassembled = Query::from_parts(
            "issues".to_string(),
            definitions,
            conditions,
            sorting,
            display,
        );

        let whole = parse("@user_id = 1234\n#issues author:@user_id \\\\created_at \\d $id $title")
            .unwrap();

        assert_eq!(reassembled, whole);
    }

    #[test]
    fn test_empty_sections_parse_to_empty_results() {
        assert_eq!(parse_definitions("").unwrap(), Definitions::default());
        assert_eq!(parse_conditions("").unwrap(), ConditionSet::default());
        assert_eq!(parse_sorting("").unwrap(), vec![]);
        assert!(parse_display("").unwrap().is_empty());
        // Whitespace and comments alone are also empty.
        assert_eq!(
            parse_definitions("  // nothing here\n").unwrap(),
            Definitions::default()
        );
    }

    #[test]
    fn test_sections_are_isolated() {
        // A constant definition is only valid in the definitions section; it is rejected when fed to
        // the conditions parser. This isolation is the whole point of the section parsers.
        assert!(parse_definitions("@user_id = 1234").is_ok());
        assert!(parse_conditions("@user_id = 1234").is_err());
        // Likewise, a `$`-prefixed display column is rejected by the conditions parser.
        assert!(parse_conditions("$id").is_err());
    }
}
