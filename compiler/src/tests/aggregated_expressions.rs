//! Unit tests for aggregating a value computed from a column (`#comments.body|length%avg`), rather
//! than the column itself. Each such pipeline is evaluated like the definition of a computed column
//! on the table hosting the column, so most of these tests assert that the inline form compiles to
//! exactly the same SQL as the equivalent computed column.

use super::corpus_loader::build_options;
use super::get_test_resource;
use crate::{Compiler, Error};

fn compile_located(input: &str) -> Result<String, Error> {
    let schema_json = get_test_resource("issue_schema.json");
    let options = build_options(crate::options::IdentifierResolution::Flexible, "postgres");
    let compiler = Compiler::new(&schema_json, options).unwrap();
    compiler.compile(input.to_string()).map(|r| r.sql)
}

fn compile(input: &str) -> Result<String, String> {
    compile_located(input).map_err(|e| e.message)
}

/// Asserts that `inline` compiles, and to the same SQL as `computed` (which defines a computed
/// column and aggregates it).
fn assert_same_as_computed_column(inline: &str, computed: &str) {
    let inline_sql = compile(inline).unwrap_or_else(|e| panic!("`{inline}` failed: {e}"));
    let computed_sql = compile(computed).unwrap_or_else(|e| panic!("`{computed}` failed: {e}"));
    assert_eq!(inline_sql, computed_sql, "inline: {inline}");
}

/// The text at which compiling `input` fails.
fn error_text(input: &str) -> &str {
    let err = compile_located(input).unwrap_err();
    let span = err.span.expect("the error should be located");
    &input[span.range()]
}

#[test]
fn across_a_to_many_path() {
    assert_same_as_computed_column(
        "#users $username $#comments.body|length%avg",
        "#comments.v = body|length\n#users $username $#comments.v%avg",
    );
}

#[test]
fn counting_distinct_computed_values() {
    assert_same_as_computed_column(
        "#projects $name $#issues.created_at|year%distinct",
        "#issues.v = created_at|year\n#projects $name $#issues.v%distinct",
    );
}

#[test]
fn across_a_to_one_link_after_a_to_many_path() {
    assert_same_as_computed_column(
        "#projects $name $#issues.author.username|length%max",
        "#users.v = username|length\n#projects $name $#issues.author.v%max",
    );
}

#[test]
fn in_a_condition() {
    assert_same_as_computed_column(
        "#projects #issues.created_at|year%distinct:>2 $name",
        "#issues.v = created_at|year\n#projects #issues.v%distinct:>2 $name",
    );
}

#[test]
fn in_a_grouped_query() {
    assert_same_as_computed_column(
        "#issues $status \\g $title|length%max",
        "#issues.v = title|length\n#issues $status \\g $v%max",
    );
}

#[test]
fn in_a_grouped_query_across_a_to_one_link() {
    assert_same_as_computed_column(
        "#issues $status \\g $author.username|length%max",
        "#users.v = username|length\n#issues $status \\g $author.v%max",
    );
}

#[test]
fn with_a_filter_on_the_related_records() {
    assert_same_as_computed_column(
        "#users $username $#comments{created_at:>@2024-01-01}.body|length%avg",
        "#comments.v = body|length\n#users $username $#comments{created_at:>@2024-01-01}.v%avg",
    );
}

#[test]
fn with_several_functions() {
    assert_same_as_computed_column(
        "#projects $name $#issues.title|length|plus(1)%sum",
        "#issues.v = title|length|plus(1)\n#projects $name $#issues.v%sum",
    );
}

#[test]
fn with_an_anonymous_function() {
    assert_same_as_computed_column(
        "#projects $name $#issues.title|length|(@n => @n * 2)%sum",
        "#issues.v = title|length|(@n => @n * 2)\n#projects $name $#issues.v%sum",
    );
}

#[test]
fn applied_to_a_computed_column() {
    assert_same_as_computed_column(
        "#issues.len = title|length\n#projects $name $#issues.len|plus(1)%sum",
        "#issues.v = title|length|plus(1)\n#projects $name $#issues.v%sum",
    );
}

#[test]
fn within_a_scoped_condition_set() {
    assert_same_as_computed_column(
        "#issues author{#comments.body|length%max:>5} $title",
        "#comments.v = body|length\n#issues author{#comments.v%max:>5} $title",
    );
}

#[test]
fn function_arguments_are_columns_of_the_aggregated_records() {
    // `created_at` is the issue's, not the project's.
    let sql = compile("#projects $name $#issues.due_date|minus(created_at)%max").unwrap();
    assert!(
        sql.contains(r#"max("issues"."due_date" - "issues"."created_at")"#),
        "got: {sql}"
    );
    // After a to-one link, they're columns of the record it leads to, just as in a computed
    // column's definition.
    let sql = compile("#issues $status \\g $author.username|concat(email)%list").unwrap();
    assert!(
        sql.contains(r#"concat("users"."username", "users"."email")"#),
        "got: {sql}"
    );
}

#[test]
fn variables_mean_what_they_meant_where_written() {
    // `@u` is bound to the user's `username`, which is joined into the CTE to be used per comment.
    let sql = compile("#users $username|(@u => #comments.body|concat(@u)%list_all)").unwrap();
    assert!(
        sql.contains(r#"array_agg(concat("comments"."body", "users"."username"))"#),
        "got: {sql}"
    );
}

#[test]
fn sorting_by_the_value_sorts_by_the_computed_value() {
    let sql = compile("#users $#comments.body|length%list(\\sd)").unwrap();
    assert!(
        sql.contains(
            r#"array_agg(DISTINCT char_length("comments"."body") ORDER BY char_length("comments"."body") DESC NULLS LAST)"#
        ),
        "got: {sql}"
    );
    let sql = compile("#issues $status \\g $title|length%list(\\s)").unwrap();
    assert!(
        sql.contains(
            r#"array_agg(DISTINCT char_length("issues"."title") ORDER BY char_length("issues"."title") ASC NULLS LAST)"#
        ),
        "got: {sql}"
    );
}

#[test]
fn sorting_by_the_column_is_not_sorting_by_the_value() {
    // The listed values are lengths, so `body` is a sort key of its own, which `list` can't use.
    let err = compile("#users $#comments.body|length%list(\\\\body)").unwrap_err();
    assert!(err.contains("removes duplicate values"), "got: {err}");
    let sql = compile("#users $#comments.body|length%list_all(\\\\body)").unwrap();
    assert!(
        sql.contains(
            r#"array_agg(char_length("comments"."body") ORDER BY "comments"."body" ASC NULLS LAST)"#
        ),
        "got: {sql}"
    );
}

#[test]
fn an_aggregate_of_related_records_may_be_used_per_record() {
    let sql = compile("#issues $status \\g $title|concat(#comments)%list").unwrap();
    assert!(
        sql.contains(r#"array_agg(DISTINCT concat("issues"."title", COALESCE("cte0"."v1", 0)))"#),
        "got: {sql}"
    );
}

#[test]
fn an_aggregate_within_the_aggregated_value_is_rejected() {
    for input in [
        "#issues $status \\g $title|concat(title%max)%list",
        "#projects $#issues.title|concat(title%max)%list",
    ] {
        let err = compile(input).unwrap_err();
        assert!(err.contains("can't be used within the value"), "got: {err}");
        assert_eq!(error_text(input), "title%max");
    }
}

#[test]
fn an_aggregating_computed_column_cannot_be_aggregated() {
    // This used to produce `max(max(…))`, which databases reject.
    for query in ["#users $#comments.mx%max", "#comments $user \\g $mx%max"] {
        let input = format!("#comments.mx = body%max\n{query}");
        let err = compile(&input).unwrap_err();
        assert!(err.contains("can't be used within the value"), "got: {err}");
    }
}

#[test]
fn an_operand_that_is_not_a_column_pipeline_is_rejected() {
    for (input, located) in [
        // An aggregate within the pipeline (the pipe after it is fine on its own).
        (
            "#projects $#issues.created_at%max|year%distinct",
            "#issues.created_at%max|year",
        ),
        ("#projects $(#issues.id + 1)%sum", "(#issues.id + 1)"),
        ("#issues $status \\g $@null%max", "@null"),
    ] {
        let err = compile(input).unwrap_err();
        assert!(
            err.contains("must be applied to a column"),
            "{input} got: {err}"
        );
        assert_eq!(error_text(input), located, "{input}");
    }
    // The pipe after an aggregate is still fine.
    assert!(compile("#projects $#issues.created_at%max|year").is_ok());
}

#[test]
fn a_path_without_a_column_is_still_rejected() {
    let err = compile("#projects $#issues|plus(1)%sum").unwrap_err();
    assert!(err.contains("column must be specified"), "got: {err}");
}
