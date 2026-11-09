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
pub mod highlight;
pub mod tokens;

pub use error::{Error, ErrorKind, Label, Position, Section, Span};

use chumsky::prelude::*;
use parser::{conditions, definitions, pad, query, result_columns, sorting, Psr, State};

/// Parses a complete Querydown query — definitions, base table, and transformations — into an
/// [`ast::Query`].
pub fn parse(input: &str) -> Result<ast::Query, Error> {
    run(query(), input, None)
}

/// Parses just the definitions section of a query (constants, functions, custom comparisons, and
/// computed columns) in isolation, without a base table or any transformations.
///
/// This is one of four section parsers — alongside [`parse_conditions`], [`parse_sorting`], and
/// [`parse_display`] — that let a consumer parse each part of a query independently and then
/// reassemble them via [`ast::Query::from_parts`] before compiling. Parsing the sections separately
/// keeps them isolated: text entered for one section cannot introduce constructs that belong to
/// another (e.g. a filtering section cannot smuggle in constant definitions).
///
/// The locations in the resulting syntax tree, and in any error, are relative to `input` and name
/// the [`Section`] it is.
pub fn parse_definitions(input: &str) -> Result<ast::Definitions, Error> {
    run(definitions(), input, Some(Section::Definitions))
}

/// Parses just the filtering section of a query in isolation: a whitespace-separated sequence of
/// boolean expressions, combined with `AND`, into an [`ast::ConditionSet`].
///
/// See [`parse_definitions`] for how the section parsers fit together.
pub fn parse_conditions(input: &str) -> Result<ast::ConditionSet, Error> {
    run(conditions(), input, Some(Section::Conditions))
}

/// Parses just the sorting section of a query in isolation: a sequence of standalone `\\` sorting
/// expressions into a `Vec<`[`ast::SortExpr`]`>`.
///
/// See [`parse_definitions`] for how the section parsers fit together.
pub fn parse_sorting(input: &str) -> Result<Vec<ast::SortExpr>, Error> {
    run(sorting(), input, Some(Section::Sorting))
}

/// Parses just the display section of a query in isolation: the `$`-prefixed result column
/// statements into a `Vec<`[`ast::ResultColumnStatement`]`>`.
///
/// See [`parse_definitions`] for how the section parsers fit together.
pub fn parse_display(input: &str) -> Result<Vec<ast::ResultColumnStatement>, Error> {
    run(result_columns(), input, Some(Section::Display))
}

/// Wraps a parser so that it reads the whole of its input, allowing surrounding whitespace/comments.
fn whole<'src, T>(parser: impl Psr<'src, T>) -> impl Psr<'src, T> {
    pad()
        .ignore_then(parser)
        .then_ignore(pad())
        .then_ignore(end())
}

/// Runs a parser over the whole of `input` — the text of the given `section` of a query, or of a
/// whole query — allowing surrounding whitespace/comments and requiring that the entire input is
/// consumed. A failure is reported as an [`ErrorKind::Syntax`] [`Error`] locating the problem within
/// `input`.
fn run<'src, T>(
    parser: impl Psr<'src, T>,
    input: &'src str,
    section: Option<Section>,
) -> Result<T, Error> {
    let mut state = State::new(input, section);
    whole(parser)
        .parse_with_state(input, &mut state)
        .into_result()
        .map_err(|errors| diagnose::diagnose(input, errors).with_section(section))
}

/// Parses `input` with the given grammar, returning the highlighting marks the parser recorded
/// (see the [`highlight`] module). When the parse fails, these are the marks of the constructs it
/// completed before failing.
fn record_marks(input: &str, grammar: highlight::Grammar) -> Vec<highlight::Token> {
    fn record<'src, T>(parser: impl Psr<'src, T>, input: &'src str) -> Vec<highlight::Token> {
        // Locations don't matter here, so the state needn't know the section.
        let mut state = State::new(input, None);
        // Only the marks matter here, not whether the parse succeeded.
        let _ = whole(parser).parse_with_state(input, &mut state);
        state.marks
    }
    use highlight::Grammar as G;
    match grammar {
        G::Query => record(query(), input),
        G::Definitions => record(definitions(), input),
        G::Conditions => record(conditions(), input),
        G::Sorting => record(sorting(), input),
        G::Display => record(result_columns(), input),
    }
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

    /// The text of `source` at `loc`.
    fn at(source: &str, loc: Loc) -> &str {
        &source[loc.span().expect("the node should have a location").range()]
    }

    #[test]
    fn test_nodes_record_their_locations() {
        let source =
            "#issues\n  !author.name:x\n  $%count $title|abs(2) * 3 $#comments(user).id%max";
        let query = parse(source).unwrap();
        assert_eq!(at(source, query.base_table_loc), "#issues");
        let transformation = &query.transformations[0];

        let negation = &transformation.conditions.entries[0];
        assert_eq!(at(source, negation.loc), "!author.name:x");
        assert_eq!(negation.loc.span().unwrap().start.line, 2);
        assert_eq!(negation.loc.section(), None);
        let ExprKind::Not(comparison) = &negation.kind else {
            panic!("expected a negation, got {negation:?}");
        };
        assert_eq!(at(source, comparison.loc), "author.name:x");
        let ExprKind::Comparison(comparison) = &comparison.kind else {
            panic!("expected a comparison, got {comparison:?}");
        };
        let ComparisonSide::Expr(left) = &comparison.left else {
            panic!("expected an expression, got {:?}", comparison.left);
        };
        let ExprKind::Path(parts) = &left.kind else {
            panic!("expected a path, got {left:?}");
        };
        assert_eq!(at(source, parts[1].loc), "name");

        let columns: Vec<&Expr> = transformation
            .result_columns
            .iter()
            .map(|statement| match statement {
                ResultColumnStatement::Spec(spec) => &spec.expr,
                other => panic!("expected a column spec, got {other:?}"),
            })
            .collect();
        assert_eq!(at(source, columns[0].loc), "%count");
        // A binary operation spans both operands; a pipe spans its input and the function applied.
        assert_eq!(at(source, columns[1].loc), "title|abs(2) * 3");
        let ExprKind::Product(call, _) = &columns[1].kind else {
            panic!("expected a product, got {:?}", columns[1]);
        };
        assert_eq!(at(source, call.loc), "title|abs(2)");
        let ExprKind::Call(call) = &call.kind else {
            panic!("expected a call, got {call:?}");
        };
        assert_eq!(at(source, call.name_loc), "|abs");
        // A `#table` path part includes its linking column.
        let ExprKind::Call(aggregate) = &columns[2].kind else {
            panic!("expected a call, got {:?}", columns[2]);
        };
        assert_eq!(at(source, aggregate.name_loc), "%max");
        let ExprKind::Path(parts) = &aggregate.args[0].kind else {
            panic!("expected a path, got {:?}", aggregate.args[0]);
        };
        assert_eq!(at(source, parts[0].loc), "#comments(user)");
    }

    #[test]
    fn test_section_locations_name_their_section() {
        let source = "status:open\n  title:x";
        let conditions = parse_conditions(source).unwrap();
        let loc = conditions.entries[1].loc;
        assert_eq!(at(source, loc), "title:x");
        assert_eq!(loc.section(), Some(Section::Conditions));

        // So do syntax errors.
        let err = parse_display("$id $").unwrap_err();
        assert_eq!(err.section, Some(Section::Display));
        assert_eq!(parse("#issues $id $").unwrap_err().section, None);
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
