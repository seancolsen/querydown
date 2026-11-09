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
    compiler
        .compile(input.to_string())
        .map(|r| r.sql)
        .map_err(|e| e.to_string())
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

// Existence checks (`++#t`, `--#t`, `#t:>0`) compile to a semi-join CTE by a specialized path that
// used to clarify the bare `#t` path from the base table, ignoring the scope. These pin each form to
// its flat equivalent, where the scope's head is written in front of the to-many path.

#[test]
fn existence_check_is_scoped_to_the_related_table() {
    assert_equivalent(
        "#issues author{++#comments{id:5}} $id",
        "#issues ++author.#comments{id:5} $id",
    );
    assert_equivalent(
        "#issues author{++#comments} $id",
        "#issues ++author.#comments $id",
    );
    assert_equivalent(
        "#issues author{--#comments} $id",
        "#issues --author.#comments $id",
    );
    assert_equivalent(
        "#issues author{#comments:>0} $id",
        "#issues author.#comments:>0 $id",
    );
    assert_equivalent(
        "#issues author{#comments{id:5}:>0} $id",
        "#issues author.#comments{id:5}:>0 $id",
    );
}

#[test]
fn existence_check_in_scope_links_through_the_scoped_table() {
    // Pin the SQL itself, not just its agreement with the flat form: the semi-join must hang off
    // `users` (via `comments.user`), not off `issues` (via `comments.issue`).
    let sql = compile("#issues author{++#comments{id:5}} $id").unwrap();
    assert!(sql.contains(r#"GROUP BY "comments"."user""#), "{sql}");
    assert!(
        sql.contains(r#"LEFT JOIN "users" ON"#) && sql.contains(r#""users"."id" = "cte0"."pk""#),
        "{sql}"
    );
    assert!(!sql.contains(r#""comments"."issue""#), "{sql}");
}

#[test]
fn count_comparison_in_scope_still_matches_flat_form() {
    // The count comparisons took the general path and were already right; keep them that way.
    assert_equivalent(
        "#issues author{#comments:>3} $id",
        "#issues author.#comments:>3 $id",
    );
    assert_equivalent(
        "#issues author{#comments{id:5}:>1} $id",
        "#issues author.#comments{id:5}:>1 $id",
    );
}

#[test]
fn existence_check_is_scoped_through_multiple_hops() {
    // "Comments on any issue in the same project as issue 42", not "comments on issue 42".
    assert_equivalent(
        "#comments issue.project{++#issues{id:42}} $id",
        "#comments ++issue.project.#issues{id:42} $id",
    );
    // Every hop of the head is kept, not just the first.
    assert_equivalent(
        "#comments issue.project.product{++#projects{id:42}} $id",
        "#comments ++issue.project.product.#projects{id:42} $id",
    );
    // Here the shortest path from the base happens to pass through the scope anyway.
    assert_equivalent(
        "#comments issue.project{++#products{id:7}} $id",
        "#comments ++issue.project.#products{id:7} $id",
    );
}

#[test]
fn existence_check_is_scoped_within_nested_scopes() {
    assert_equivalent(
        "#comments issue{project{++#issues{id:42}}} $id",
        "#comments ++issue.project.#issues{id:42} $id",
    );
}

#[test]
fn existence_check_in_scope_does_not_take_a_self_link_from_the_base() {
    // The shortest path from `#issues` to `#issues` is the `duplicate_of` self-link; the scope must
    // win, making this "issues in the same project as issue 42".
    assert_equivalent(
        "#issues project{++#issues{id:42}} $id",
        "#issues ++project.#issues{id:42} $id",
    );
}

#[test]
fn existence_check_is_scoped_within_or_sets() {
    // An existence-checking scope inside an outer `[ ]`…
    assert_equivalent(
        "#issues [author{++#comments} title:x] $id",
        "#issues [++author.#comments title:x] $id",
    );
    // …and an `[ ]` scope holding an existence check.
    assert_equivalent(
        "#issues author[++#comments team.name:x] $id",
        "#issues [++author.#comments author.team.name:x] $id",
    );
}

#[test]
fn existence_check_in_scope_works_as_a_sort() {
    assert_equivalent(
        "#issues \\\\author{++#comments} $id",
        "#issues \\\\++author.#comments $id",
    );
}

#[test]
fn aggregate_in_scope_is_scoped_to_the_related_table() {
    // Aggregate functions shared the fault: they also clarified the bare path from the base table.
    assert_equivalent(
        "#issues author{#comments.id%max:>5} $id",
        "#issues author.#comments.id%max:>5 $id",
    );
    assert_equivalent(
        "#issues author{#comments.id%count:>5} $id",
        "#issues author.#comments.id%count:>5 $id",
    );
    assert_equivalent(
        "#issues \\\\author{#comments.id%max} $id",
        "#issues \\\\author.#comments.id%max $id",
    );
    // A sort by the aggregated value is still recognized as such within a scope.
    assert_equivalent(
        "#issues author{#comments.body%list(\\\\body):x} $id",
        "#issues author.#comments.body%list(\\\\body):x $id",
    );
    // A local aggregate of the scoped table's own column.
    assert_equivalent(
        "#issues author{username%max:z} $id",
        "#issues author.username%max:z $id",
    );
}
