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
fn bare_word_is_a_search_even_when_it_matches_a_column_name() {
    // A bare word always searches; it does not defer to a column of the same name. `title` is a
    // real column, yet `#issues title` searches for the literal text "title" across every text-like
    // column of the base table.
    let sql = compile("#issues title $id").unwrap();
    assert!(sql.contains("'title'"), "got: {sql}");
    assert!(sql.contains(r#""issues"."description""#), "got: {sql}");
    assert!(sql.contains(r#""issues"."status""#), "got: {sql}");
}

#[test]
fn backtick_quoting_forces_a_column_reference() {
    // Backtick-quoting a bare word designates it as a column reference rather than a search term —
    // the parser preserves this distinction, which a bare word alone cannot express.
    let sql = compile("#issues `title` $id").unwrap();
    assert!(sql.contains(r#""issues"."title""#), "got: {sql}");
    assert!(!sql.contains("description"), "got: {sql}");
}

#[test]
fn bare_word_searches_every_text_column() {
    let sql = compile("#issues accessibility $id").unwrap();
    assert!(sql.contains(r#""issues"."title""#), "got: {sql}");
    assert!(sql.contains(r#""issues"."description""#), "got: {sql}");
    assert!(sql.contains(r#""issues"."status""#), "got: {sql}");
    assert!(sql.contains("'accessibility'"), "got: {sql}");
}

#[test]
fn bare_word_with_underscore_is_not_a_search_term() {
    // The bare-search-word form only applies to letter-initial, strictly alphanumeric words. A word
    // with an underscore is a column reference, so an unknown one is an error rather than a search.
    let err = compile("#issues not_a_column $id").unwrap_err();
    assert!(err.contains("not_a_column"), "got: {err}");
}

#[test]
fn comma_shorthand_searches_each_bare_operand() {
    // `foo,bar` is an "OR" of two default text searches: issues containing "foo" or "bar".
    let sql = compile("#issues foo,bar $id").unwrap();
    assert!(sql.contains("'foo'"), "got: {sql}");
    assert!(sql.contains("'bar'"), "got: {sql}");
    // Both operands search; neither is treated as a column reference.
    assert!(!sql.contains(r#""issues"."foo""#), "got: {sql}");
    assert!(!sql.contains(r#""issues"."bar""#), "got: {sql}");
}

#[test]
fn comma_shorthand_mixes_searches_and_comparisons() {
    // A comparison operand keeps its meaning while a bare operand becomes a search. Here `:=` is a
    // case-insensitive text equality (`lower(...) = lower(...)`), distinct from the bare "bar" search.
    let sql = compile("#issues status:=open,bar $id").unwrap();
    assert!(
        sql.contains(r#"lower("issues"."status" COLLATE "C") = lower('open' COLLATE "C")"#),
        "got: {sql}"
    );
    assert!(sql.contains("'bar'"), "got: {sql}");
}

#[test]
fn backtick_quoting_a_word_in_a_bracketed_set_is_a_column_reference() {
    // The bare-vs-backtick distinction also holds inside a boolean `[ ]`/`{ }` condition set.
    let sql = compile("#issues [`status` `title`] $id").unwrap();
    assert!(sql.contains(r#""issues"."status""#), "got: {sql}");
    assert!(sql.contains(r#""issues"."title""#), "got: {sql}");
    assert!(!sql.contains("strpos"), "got: {sql}");
}

#[test]
fn search_with_no_text_columns_is_rejected() {
    // The `assignments` table has only non-text (integer) columns, so a default text search against
    // it has nothing to search and is reported as an error.
    let err = compile("#assignments hello").unwrap_err();
    assert!(err.contains("text-like"), "got: {err}");
}

#[test]
fn negated_bare_word_is_a_negated_search() {
    // `!term` negates a default text search: it matches rows that do *not* contain the term in any
    // text column, so the whole per-column OR is wrapped in `NOT`.
    let sql = compile("#issues !backend $id").unwrap();
    assert!(sql.contains("NOT ("), "got: {sql}");
    assert!(sql.contains("'backend'"), "got: {sql}");
    // The bare word searches, so it is not read as a column reference.
    assert!(!sql.contains(r#""issues"."backend""#), "got: {sql}");
}

#[test]
fn repeated_negation_of_a_search_term_double_negates() {
    // Each `!` adds a `NOT` layer, so `!!term` negates the search twice.
    let sql = compile("#issues !!backend $id").unwrap();
    assert!(sql.contains("NOT NOT ("), "got: {sql}");
}

#[test]
fn negated_search_operand_in_comma_shorthand() {
    // A `!`-prefixed operand of the comma "OR" shorthand is a negated search, while its bare
    // sibling remains an ordinary (positive) search.
    let sql = compile("#issues foo,!bar $id").unwrap();
    assert!(sql.contains("'foo'"), "got: {sql}");
    assert!(sql.contains("NOT ("), "got: {sql}");
    assert!(sql.contains("'bar'"), "got: {sql}");
}

#[test]
fn backtick_quoting_negates_a_column_reference_rather_than_a_search() {
    // Just as a bare word searches, a *negated* bare word negates a search. Backtick-quoting instead
    // negates the column of that name — here the boolean `is_active` column of `projects`.
    let sql = compile("#projects !`is_active` $id").unwrap();
    assert!(sql.contains(r#"NOT "projects"."is_active""#), "got: {sql}");
    assert!(!sql.contains("strpos"), "got: {sql}");
}
