//! Unit tests for grouping and aggregation that don't fit the SQL-string corpus, principally the
//! validation rule that, in a grouped query, every result column must be a grouping column or an
//! aggregate.

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
fn standalone_count_is_count_star() {
    let sql = compile("#issues $status \\g $%count").unwrap();
    assert!(sql.contains("count(*)"), "got: {sql}");
    assert!(sql.contains("GROUP BY \"issues\".\"status\""), "got: {sql}");
}

#[test]
fn ungrouped_unaggregated_column_is_rejected() {
    // `title` is neither grouped nor aggregated, so the grouped query is invalid.
    let err = compile("#issues $status \\g $title $%count").unwrap_err();
    assert!(err.contains("grouping column"), "got: {err}");
}

#[test]
fn grouping_column_need_not_be_aggregated() {
    // The grouped column itself is exempt from the "must be aggregated" rule.
    assert!(compile("#issues $status \\g $%count").is_ok());
}

#[test]
fn multiple_grouping_columns_are_allowed() {
    let sql = compile("#issues $status \\g $author \\g $%count").unwrap();
    assert!(
        sql.contains("GROUP BY \"issues\".\"status\", \"issues\".\"author\""),
        "got: {sql}"
    );
}

#[test]
fn aggregate_without_grouping_aggregates_whole_table() {
    // An aggregate with no grouping is valid SQL (it collapses the whole table to one row) and
    // emits no GROUP BY clause.
    let sql = compile("#issues $%count").unwrap();
    assert!(sql.contains("count(*)"), "got: {sql}");
    assert!(!sql.contains("GROUP BY"), "got: {sql}");
}

#[test]
fn standalone_non_count_aggregate_is_rejected() {
    // Only `count` is meaningful as a standalone aggregate; `%max` needs an argument.
    let err = compile("#issues $status \\g $%max").unwrap_err();
    assert!(err.contains("requires an argument"), "got: {err}");
}

#[test]
fn ungrouped_query_with_naked_column_is_allowed() {
    // Without any grouping, a plain column is fine.
    assert!(compile("#issues $title").is_ok());
}

#[test]
fn count_of_related_records_is_coalesced_to_zero() {
    // A to-many count is computed in a CTE joined via LEFT JOIN, so issues with no comments would
    // otherwise yield NULL. The count is coalesced to zero so those rows show 0 instead.
    let sql = compile("#issues $title $#comments").unwrap();
    assert!(sql.contains("COALESCE(\"cte0\".\"v1\", 0)"), "got: {sql}");
}

#[test]
fn non_count_aggregate_over_related_records_is_not_coalesced() {
    // Only counts are coalesced to zero; a `max` over a to-many path should remain NULL when there
    // are no related records.
    let sql = compile("#issues $id $#comments.created_at%max").unwrap();
    assert!(!sql.contains("COALESCE"), "got: {sql}");
}
