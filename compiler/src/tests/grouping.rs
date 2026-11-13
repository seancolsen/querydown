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
    compiler
        .compile(input.to_string())
        .map(|r| r.sql)
        .map_err(|e| e.to_string())
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

/// The validation rule for result columns applies to sorting too: SQL can't sort the groups by a
/// value that is neither grouped nor aggregated. (Databases reject such an `ORDER BY`: "column
/// must appear in the GROUP BY clause or be used in an aggregate".)
#[test]
fn ungrouped_unaggregated_sort_is_rejected() {
    // Sorting by a grouping expression or an aggregate is fine.
    assert!(compile(r"#issues \\status $status \g $%count").is_ok());
    assert!(compile(r"#issues \\created_at%max $status \g $%count").is_ok());
    assert!(compile(r"#issues \\created_at|year $created_at|year \g $%count").is_ok());
    // To sort by a grouping column that counts related records, flag the column itself: a
    // standalone sort would count them again, separately.
    assert!(compile(r"#issues $#comments \gs $%count").is_ok());

    for input in [
        r"#issues \\title $status \g $%count",
        r"#issues \\author.username \d $status \g $%count",
        r"#issues \\status|uppercase $status \g $%count",
    ] {
        let err = compile(input).unwrap_err();
        assert!(
            err.contains("can only be sorted by a grouping column"),
            "for {input}: {err}"
        );
    }
    // Without grouping, anything goes.
    assert!(compile(r"#issues \\title $status").is_ok());
}

/// A count or aggregate of related records has a value for each record of the base table, so in a
/// grouped query it's no more an aggregate than a plain column is. (It used to pass validation,
/// producing SQL that databases reject.)
#[test]
fn ungrouped_aggregate_of_related_records_is_rejected() {
    for input in [
        r"#issues $status \g $#comments",
        r"#issues $status \g $#comments.created_at%max",
        r"#issues $project \g $#labels.name%list",
        r"#issues $status \g $++#comments",
        r"#issues $status \g $#comments + 1 $%count",
        r"#issues \\#comments \d $status \g $%count",
    ] {
        let err = compile(input).unwrap_err();
        assert!(
            err.starts_with("A count or aggregate of related records"),
            "for {input}: {err}"
        );
    }
}

#[test]
fn aggregate_of_related_records_can_be_grouped_or_aggregated() {
    // It can be a grouping column...
    let sql = compile(r"#issues $#comments \g $%count").unwrap();
    assert!(
        sql.contains("GROUP BY COALESCE(\"cte0\".\"v1\", 0)"),
        "got: {sql}"
    );
    // ...and, named by a computed column, it can be aggregated across each group.
    let sql = compile("#issues.n = #comments\n#issues $status \\g $n%sum").unwrap();
    assert!(
        sql.contains("sum(COALESCE(\"cte0\".\"v1\", 0))"),
        "got: {sql}"
    );
}
