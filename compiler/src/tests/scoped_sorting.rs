//! Tests for scoped sorting expressions (`\\issue.( \\id \\title )`), which the parser desugars
//! into the equivalent flat sorting expressions (`\\issue.id \\issue.title`). These tests assert
//! that a query written with scoping compiles to exactly the same SQL as its flat counterpart.

use super::corpus_loader::build_options;
use super::get_test_resource;
use crate::Compiler;

fn compile(input: &str) -> Result<String, String> {
    let schema_json = get_test_resource("issue_schema.json");
    let options = build_options(crate::options::IdentifierResolution::Flexible, "postgres");
    let compiler = Compiler::new(&schema_json, options).unwrap();
    compiler.compile(input.to_string()).map(|r| r.sql)
}

/// Asserts that the scoped and flat forms compile to identical SQL (and that they compile at all).
fn assert_equivalent(scoped: &str, flat: &str) {
    let scoped_sql = compile(scoped).unwrap();
    let flat_sql = compile(flat).unwrap();
    assert_eq!(scoped_sql, flat_sql, "for scoped query: {scoped}");
}

#[test]
fn scoping_matches_flat_form() {
    assert_equivalent(
        r"#issues
        \\project.(
          \\is_active
          \\product.name
        )
        \\due_date
        $title",
        r"#issues
        \\project.is_active
        \\project.product.name
        \\due_date
        $title",
    );
}

#[test]
fn scoping_composes_with_scoping() {
    assert_equivalent(
        r"#issues \\project.( \\product.( \\name ) ) $title",
        r"#issues \\project.product.name $title",
    );
}

#[test]
fn scoping_head_may_traverse_to_many_paths() {
    assert_equivalent(
        r"#issues \\#comments.( \\created_at%max ) $title",
        r"#issues \\#comments.created_at%max $title",
    );
}

#[test]
fn scoping_preserves_sort_flags() {
    assert_equivalent(
        r"#issues \\project.( \\name \d ) $title",
        r"#issues \\project.name \d $title",
    );
}

#[test]
fn scoped_literal_is_rejected() {
    // A nested entry with no leading column reference has nowhere to receive the head path.
    assert!(compile(r"#issues \\project.( \\8 ) $title").is_err());
}
