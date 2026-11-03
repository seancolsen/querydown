//! Labels attached to grammar rules via Chumsky's `labelled`, so that a syntax error's "expected"
//! set names the construct the user could have written (e.g. "a value") rather than listing every
//! character that could have started it. Each label reads naturally after the word "Expected".
//!
//! A label only replaces the expected set for failures at the very start of the labelled rule; a
//! failure deeper inside it is still described in terms of the inner rules.

/// Insignificant whitespace and comments. These are never worth mentioning, so the error
/// translation drops this label; it exists only to keep the characters that could start
/// whitespace or a comment out of the expected set. (It matches the label Chumsky uses for its own
/// whitespace parsers.)
pub(crate) const WHITESPACE: &str = "whitespace";

// Query structure
pub(crate) const DEFINITION: &str = "a definition";
pub(crate) const BASE_TABLE: &str = "a base table";
pub(crate) const CONDITION: &str = "a condition";
pub(crate) const SORT_EXPR: &str = "a sort expression (`\\\\`)";
pub(crate) const RESULT_COLUMN: &str = "a result column (`$`)";
pub(crate) const RESULT_COLUMN_BODY: &str = "a column, an expression, or `*`";
pub(crate) const PIPELINE_DELIMITER: &str = "`~~~`";

// Expressions
pub(crate) const VALUE: &str = "a value";
pub(crate) const COMPARISON_OPERATOR: &str = "a comparison operator";
pub(crate) const RELATED_TABLE: &str = "a related table (e.g. `#comments`)";
pub(crate) const FUNCTION_NAME: &str = "a function name";
pub(crate) const AGGREGATE_FUNCTION_NAME: &str = "an aggregate function name";
pub(crate) const VARIABLE_NAME: &str = "a variable name";
pub(crate) const FUNCTION_PARAMETER: &str = "a function parameter (e.g. `@x`)";
pub(crate) const FUNCTION_ARROW: &str = "`=>`";
pub(crate) const CASE_VARIANT_SEPARATOR: &str = "`~`";
pub(crate) const CASE_FALLBACK: &str = "`~~` followed by a fallback value";

pub(crate) const NUMBER: &str = "a number";
pub(crate) const DATE: &str = "a date (e.g. `@2024-01-31`)";
pub(crate) const ESCAPE_CHARACTER: &str =
    "an escape character (`\\`, `/`, the quote character, `b`, `f`, `n`, `r`, `t`, or `u`)";
pub(crate) const UNICODE_ESCAPE_DIGITS: &str = "four hexadecimal digits";

// Names
pub(crate) const NAME: &str = "a name";
pub(crate) const TABLE_NAME: &str = "a table name";
pub(crate) const ALIAS: &str = "an alias";

// Flags
pub(crate) const COLUMN_CONTROL_FLAG: &str =
    "a column control flag (`s`, `d`, `n`, `g`, `h`, `p`, or a number)";
pub(crate) const SORT_FLAG: &str = "a sort flag (`d` or `n`)";

// Annotations
pub(crate) const ANNOTATION_KEY: &str = "an annotation key";
pub(crate) const ANNOTATION_VALUE: &str = "an annotation value";
