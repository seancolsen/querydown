//! Tests for result column spec nesting (`$issue.( $id $title )`), which the parser desugars into
//! the equivalent flat result columns (`$issue.id $issue.title`). These tests assert that a query
//! written with nesting compiles to exactly the same SQL as its flat counterpart.

use super::corpus_loader::build_options;
use super::get_test_resource;
use crate::Compiler;

fn compile(input: &str) -> Result<String, String> {
    let schema_json = get_test_resource("issue_schema.json");
    let options = build_options(crate::options::IdentifierResolution::Flexible, "postgres");
    let compiler = Compiler::new(&schema_json, options).unwrap();
    compiler.compile(input.to_string()).map(|r| r.sql)
}

/// Asserts that the nested and flat forms compile to identical SQL (and that they compile at all).
fn assert_equivalent(nested: &str, flat: &str) {
    let nested_sql = compile(nested).unwrap();
    let flat_sql = compile(flat).unwrap();
    assert_eq!(nested_sql, flat_sql, "for nested query: {nested}");
}

#[test]
fn nesting_matches_flat_form() {
    assert_equivalent(
        r"#comments
        $id @{hide:yes} // the comment id
        $created_at
        $issue.(
          $id @{hide:yes} // the issue id
          $title @{width:400}
          $#labels.name%list(\\name)
          $project.name
        )",
        r"#comments
        $id @{hide:yes} // the comment id
        $created_at
        $issue.id @{hide:yes} // the issue id
        $issue.title @{width:400}
        $issue.#labels.name%list(\\name)
        $issue.project.name",
    );
}

#[test]
fn nesting_composes_with_nesting() {
    assert_equivalent(
        "#comments $id $issue.( $title $project.( $name ) )",
        "#comments $id $issue.title $issue.project.name",
    );
}

#[test]
fn nesting_applies_to_globs() {
    assert_equivalent("#issues $id $author.( $* )", "#issues $id $author.*");
}

#[test]
fn nesting_head_may_traverse_to_many_paths() {
    assert_equivalent(
        "#issues $id $#comments.( $created_at%max )",
        "#issues $id $#comments.created_at%max",
    );
}

#[test]
fn nesting_preserves_sorting_flags() {
    assert_equivalent(
        r"#comments $body $issue.( $created_at \sd )",
        r"#comments $body $issue.created_at \sd",
    );
}

#[test]
fn nesting_preserves_grouping_flags() {
    assert_equivalent(
        r"#comments $issue.( $status \g ) $created_at%max",
        r"#comments $issue.status \g $created_at%max",
    );
}

#[test]
fn nested_literal_is_rejected() {
    // A nested entry with no leading column reference has nowhere to receive the head path.
    assert!(compile("#comments $issue.( $8 )").is_err());
}
