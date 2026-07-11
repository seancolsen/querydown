//! Unit tests for the text equality operators `:=` (case-insensitive) and `:==` (case-sensitive),
//! asserting the distinction not otherwise obvious from the SQL-string corpus.

use super::corpus_loader::build_options;
use super::get_test_resource;
use crate::Compiler;

fn compile(input: &str, dialect: &str) -> Result<String, String> {
    let schema_json = get_test_resource("issue_schema.json");
    let options = build_options(crate::options::IdentifierResolution::Flexible, dialect);
    let compiler = Compiler::new(&schema_json, options).unwrap();
    compiler.compile(input.to_string()).map(|r| r.sql)
}

#[test]
fn case_insensitive_equality_lowercases_both_sides_on_postgres() {
    // `:=` on a text column compares case-insensitively, mirroring the `:` operator's
    // `lower(... COLLATE "C")` normalization.
    let sql = compile("#issues status:=open", "postgres").unwrap();
    assert!(
        sql.contains(r#"lower("issues"."status" COLLATE "C") = lower('open' COLLATE "C")"#),
        "got: {sql}"
    );
}

#[test]
fn case_insensitive_equality_is_accent_insensitive_on_duckdb() {
    // On DuckDB, `:=` additionally strips accents, matching `text_contains`.
    let sql = compile("#issues status:=open", "duckdb").unwrap();
    assert!(
        sql.contains(r#"lower(strip_accents("issues"."status")) = lower(strip_accents('open'))"#),
        "got: {sql}"
    );
}

#[test]
fn case_sensitive_equality_is_a_plain_comparison() {
    // `:==` forces exact, case-sensitive equality even for text — a plain `=` with no normalization.
    let sql = compile("#issues status:==open", "postgres").unwrap();
    assert!(sql.contains(r#""issues"."status" = 'open'"#), "got: {sql}");
    assert!(!sql.contains("lower("), "got: {sql}");
}

#[test]
fn both_operators_fall_back_to_plain_equality_for_non_text() {
    // Neither operator normalizes when the left-hand side is not text; both reduce to `=`.
    for input in ["#issues id:=50", "#issues id:==50"] {
        let sql = compile(input, "postgres").unwrap();
        assert!(sql.contains(r#""issues"."id" = 50"#), "got: {sql}");
        assert!(!sql.contains("lower("), "got: {sql}");
    }
}
