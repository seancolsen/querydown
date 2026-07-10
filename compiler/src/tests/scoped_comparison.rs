//! Tests for scoped comparisons (`issue{title:dashboard}`): a path written immediately before a
//! condition set scopes every entry of that set to the related record the path points at. The
//! scoping is resolved in the compiler (via the scope's path prefix), so anything you could write as
//! a top-level condition on the related table works inside the braces — including a bare default
//! text search, which has no flat-syntax equivalent.

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
/// Used for the cases whose entries all have a flat-syntax equivalent.
fn assert_equivalent(scoped: &str, flat: &str) {
    let scoped_sql = compile(scoped).unwrap();
    let flat_sql = compile(flat).unwrap();
    assert_eq!(scoped_sql, flat_sql, "for scoped query: {scoped}");
}

#[test]
fn and_scope_matches_flat_form() {
    // A `{ }` scope composes its entries with AND, applying the head to each.
    assert_equivalent(
        "#comments issue{title:dashboard}",
        "#comments issue.title:dashboard",
    );
    assert_equivalent(
        "#comments issue{title:dashboard description:dashboard}",
        "#comments {issue.title:dashboard issue.description:dashboard}",
    );
}

#[test]
fn or_scope_matches_flat_form() {
    // An `[ ]` scope composes its entries with OR, applying the head to each.
    assert_equivalent(
        "#comments issue[title:dashboard description:dashboard]",
        "#comments [issue.title:dashboard issue.description:dashboard]",
    );
}

#[test]
fn scope_head_may_traverse_multiple_hops() {
    // The head may be a multi-part path to a single related record.
    assert_equivalent(
        "#comments issue.project{name:dashboard}",
        "#comments issue.project.name:dashboard",
    );
}

#[test]
fn scope_preserves_operators_and_negation() {
    // Whatever the entry is — an exact-match operator, a negation — the head rides in front of its
    // leading column reference exactly as if written inline.
    assert_equivalent(
        "#comments issue{status:=open}",
        "#comments issue.status:=open",
    );
    assert_equivalent(
        "#comments issue{!title:dashboard}",
        "#comments !issue.title:dashboard",
    );
}

#[test]
fn scope_nests_inside_a_condition_set() {
    // A scoped comparison is an ordinary boolean expression, so it works inside an outer `[ ]`/`{ }`.
    assert_equivalent(
        "#comments [issue{title:dashboard} body:workaround]",
        "#comments [issue.title:dashboard body:workaround]",
    );
}

#[test]
fn scopes_nest_within_scopes() {
    // The head of an outer scope applies to a scoped-comparison entry too, so the paths compose.
    assert_equivalent(
        "#comments issue{project{name:dashboard} title:bug}",
        "#comments {issue.project.name:dashboard issue.title:bug}",
    );
}

#[test]
fn bare_default_text_search_is_scoped_to_the_related_table() {
    // This is the case with no flat-syntax equivalent: a bare word inside the scope searches the
    // *related* table's text columns, not the base table's. Scoping `issue{dashboard}` therefore
    // matches searching each of the issue's text columns.
    assert_equivalent(
        "#comments issue{dashboard}",
        "#comments [issue.title:dashboard issue.description:dashboard issue.status:dashboard]",
    );
    // The bare search composes with ordinary comparisons in the same scope.
    assert_equivalent(
        "#comments issue{dashboard status:=open}",
        "#comments {[issue.title:dashboard issue.description:dashboard issue.status:dashboard] issue.status:=open}",
    );
}

#[test]
fn bare_default_text_search_is_scoped_through_multiple_hops() {
    // Through a two-hop head, the search lands on the final related table's text columns. Projects
    // has a single text column (`name`), so the search is a one-entry OR set over it.
    assert_equivalent(
        "#comments issue.project{dashboard}",
        "#comments [issue.project.name:dashboard]",
    );
}

#[test]
fn space_before_brace_is_not_a_scoped_comparison() {
    // With a space, `issue` is a default text-search term on the *base* table and `{ ... }` a
    // separate condition set — the pre-existing behavior — so this differs from the scoped form.
    let spaced = compile("#comments issue {id:7}");
    let scoped = compile("#comments issue{id:7}");
    assert!(spaced.is_ok());
    assert!(scoped.is_ok());
    assert_ne!(spaced.unwrap(), scoped.unwrap());
}
