//! Unit tests for the default text search feature that assert behavior not captured by the
//! SQL-string corpus — principally disambiguation and error cases.

use super::corpus_loader::build_options;
use super::get_test_resource;
use crate::Compiler;

fn compile(input: &str) -> Result<String, String> {
    let schema_json = get_test_resource("issue_schema.json");
    let options = build_options(crate::options::IdentifierResolution::Flexible, "postgres");
    let compiler = Compiler::new(&schema_json, options).unwrap();
    compiler.compile(input.to_string()).map(|r| r.sql)
}

#[test]
fn bare_word_resolving_to_a_real_column_is_a_column_reference() {
    // `title` is a real column, so as a bare condition it stays a column reference rather than
    // becoming a default text search across all text columns.
    let sql = compile("#issues title $id").unwrap();
    assert!(sql.contains(r#""issues"."title""#), "got: {sql}");
    // A real-column reference does not search `description` the way a text search would.
    assert!(!sql.contains("description"), "got: {sql}");
}

#[test]
fn bare_word_resolving_to_a_computed_column_is_not_a_search() {
    // A computed column behaves like a real column, so a bare reference to it is not a search.
    let sql = compile("#issues.is_open = status:=\"open\"\n#issues is_open $id").unwrap();
    assert!(sql.contains(r#""issues"."status" = 'open'"#), "got: {sql}");
}

#[test]
fn unresolved_bare_word_becomes_a_default_text_search() {
    // `accessibility` is not a column, so it searches every text-like column of the base table.
    let sql = compile("#issues accessibility $id").unwrap();
    assert!(sql.contains(r#""issues"."title""#), "got: {sql}");
    assert!(sql.contains(r#""issues"."description""#), "got: {sql}");
    assert!(sql.contains(r#""issues"."status""#), "got: {sql}");
    assert!(sql.contains("'accessibility'"), "got: {sql}");
}

#[test]
fn bare_word_with_underscore_is_not_a_search_term() {
    // The bare-word form only applies to letter-initial, strictly alphanumeric words. A word with
    // an underscore that does not resolve to a column is an error, not a search.
    let err = compile("#issues not_a_column $id").unwrap_err();
    assert!(err.contains("not_a_column"), "got: {err}");
}

#[test]
fn search_with_no_text_columns_is_rejected() {
    // The `assignments` table has only non-text (integer) columns, so a default text search against
    // it has nothing to search and is reported as an error.
    let err = compile("#assignments hello").unwrap_err();
    assert!(err.contains("text-like"), "got: {err}");
}
