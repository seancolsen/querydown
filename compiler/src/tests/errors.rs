//! Tests for the errors the compiler reports: their kinds, messages, and locations.
//!
//! Syntax error messages are tested in depth in the parser crate; here we check that they surface
//! through [`Compiler::compile`] intact, along with the compiler's own (schema and compile) errors.

use crate::{
    options::IdentifierResolution, Compiler, Error, ErrorKind, Options, Postgres, Section,
};

use super::get_test_resource;

fn options() -> Options {
    Options {
        dialect: Box::new(Postgres()),
        identifier_resolution: IdentifierResolution::Flexible,
        limit: None,
        offset: None,
    }
}

fn compile_with_schema(schema_json: &str, input: &str) -> Result<String, Error> {
    let compiler = Compiler::new(schema_json, options())?;
    compiler.compile(input.to_string()).map(|r| r.sql)
}

fn compile(input: &str) -> Result<String, Error> {
    compile_with_schema(&get_test_resource("issue_schema.json"), input)
}

/// Compiles `input`, which should fail with a compile error located within it.
#[track_caller]
fn compile_error(input: &str) -> Error {
    let err = compile(input).expect_err("input should fail to compile");
    assert_eq!(err.kind, ErrorKind::Compile, "unexpected kind: {err:?}");
    assert!(
        err.span.is_some(),
        "compile error without a location: {err:?}"
    );
    assert_eq!(err.section, None);
    err
}

/// The text of `source` that `err` is located at.
#[track_caller]
fn located<'a>(source: &'a str, err: &Error) -> &'a str {
    let span = err.span.expect("the error should have a location");
    &source[span.range()]
}

/// A schema in which `c` is reachable from `a` in two equally short ways (via `x` and via `y`), and
/// `z` is unrelated to everything.
const DIAMOND_SCHEMA: &str = r#"{
  "tables": [
    { "name": "a", "columns": [{ "name": "id", "type": "integer" }] },
    { "name": "x", "columns": [{ "name": "id", "type": "integer" }, { "name": "a", "type": "integer" }] },
    { "name": "y", "columns": [{ "name": "id", "type": "integer" }, { "name": "a", "type": "integer" }] },
    { "name": "c", "columns": [
      { "name": "id", "type": "integer" },
      { "name": "x", "type": "integer" },
      { "name": "y", "type": "integer" }
    ] },
    { "name": "z", "columns": [{ "name": "id", "type": "integer" }] }
  ],
  "links": [
    { "from": { "table": "x", "column": "a" }, "to": { "table": "a", "column": "id" }, "unique": false },
    { "from": { "table": "y", "column": "a" }, "to": { "table": "a", "column": "id" }, "unique": false },
    { "from": { "table": "c", "column": "x" }, "to": { "table": "x", "column": "id" }, "unique": false },
    { "from": { "table": "c", "column": "y" }, "to": { "table": "y", "column": "id" }, "unique": false }
  ]
}"#;

// ------------------------------------------------------------------------------------------------
// Error kinds and locations
// ------------------------------------------------------------------------------------------------

#[test]
fn syntax_error_is_located_within_the_input() {
    let input = "#issues\n  status:open\n  title:)";
    let err = compile(input).unwrap_err();
    assert_eq!(err.kind, ErrorKind::Syntax);
    let span = err.span.expect("syntax errors have a span");
    assert_eq!((span.start.line, span.start.column), (3, 9));
    assert_eq!(&input[span.range()], ")");
    assert_eq!(
        err.to_string(),
        "Unexpected `)`, which has no matching `(`. Expected a value. (line 3, column 9)"
    );
    assert!(err.render(input).contains("3 |   title:)\n  |         ^"));
}

#[test]
fn compile_error_is_located_within_the_input() {
    let input = "#issues\n  status:open\n  $nope|abs";
    let err = compile_error(input);
    assert_eq!(
        err.message,
        "Column `nope` not found within table `issues`."
    );
    assert_eq!(located(input, &err), "nope");
    let span = err.span.unwrap();
    assert_eq!((span.start.line, span.start.column), (3, 4));
    assert_eq!(
        err.to_string(),
        "Column `nope` not found within table `issues`. (line 3, column 4)"
    );
    assert_eq!(
        err.render(input),
        "compile error: Column `nope` not found within table `issues`.\n \
         --> line 3, column 4\n  \
         |\n\
         3 |   $nope|abs\n  \
         |    ^^^^"
    );
}

#[test]
fn error_is_a_std_error() {
    fn run() -> Result<String, Box<dyn std::error::Error>> {
        Ok(compile("#issues $title->")?)
    }
    let err = run().unwrap_err();
    assert!(err.to_string().starts_with("Unexpected end of input."));
}

#[test]
fn invalid_schema_json_is_a_schema_error() {
    let err = compile_with_schema(r#"{"tables": ["#, "#issues").unwrap_err();
    assert_eq!(err.kind, ErrorKind::Schema);
    assert_eq!(err.span, None);
    // The detail from the JSON parser is preserved, so the problem can be found in the schema.
    assert!(
        err.message.starts_with("The schema is not valid: "),
        "{err}"
    );
    assert!(err.message.contains("line 1"), "{err}");
}

#[test]
fn schema_with_wrong_structure_is_a_schema_error() {
    let err = compile_with_schema(r#"{"tables": []}"#, "#issues").unwrap_err();
    assert_eq!(err.kind, ErrorKind::Schema);
    assert!(err.message.contains("missing field `links`"), "{err}");
}

#[test]
fn schema_link_to_unknown_table_is_a_schema_error() {
    let schema = r#"{
      "tables": [{ "name": "a", "columns": [{ "name": "id", "type": "integer" }] }],
      "links": [{ "from": { "table": "a", "column": "id" }, "to": { "table": "b", "column": "id" }, "unique": false }]
    }"#;
    let err = compile_with_schema(schema, "#a").unwrap_err();
    assert_eq!(err.kind, ErrorKind::Schema);
    assert_eq!(
        err.message,
        "The schema has a link involving the table `b`, which is not among the schema's tables."
    );
}

#[test]
fn compile_query_reports_compile_errors() {
    let compiler = Compiler::new(&get_test_resource("issue_schema.json"), options()).unwrap();
    let query = crate::parse("#nope $id").unwrap();
    let err = compiler.compile_query(query).unwrap_err();
    assert_eq!(err.kind, ErrorKind::Compile);
    assert_eq!(err.message, "Base table `nope` does not exist.");
    assert_eq!(located("#nope $id", &err), "#nope");
}

#[test]
fn compile_error_in_a_section_names_the_section() {
    let compiler = Compiler::new(&get_test_resource("issue_schema.json"), options()).unwrap();
    let definitions = "@open = status:open\n@mine = author.nope:1";
    let conditions = "@open";
    let display = "$id\n$title|nope";
    let sections = |conditions: &str, display: &str| {
        crate::ast::Query::from_parts(
            "issues".to_string(),
            crate::parse_definitions(definitions).unwrap(),
            crate::parse_conditions(conditions).unwrap(),
            crate::parse_sorting("").unwrap(),
            crate::parse_display(display).unwrap(),
        )
    };

    let err = compiler
        .compile_query(sections(conditions, display))
        .unwrap_err();
    assert_eq!(err.message, "Scalar function `nope` does not exist.");
    assert_eq!(err.section, Some(Section::Display));
    assert_eq!(located(display, &err), "|nope");
    assert_eq!(err.span.unwrap().start.line, 2);

    // An error within a definition is located there, even though the definition is used elsewhere.
    let err = compiler
        .compile_query(sections("@mine", "$id"))
        .unwrap_err();
    assert_eq!(err.section, Some(Section::Definitions));
    assert_eq!(located(definitions, &err), "nope");
}

#[test]
fn compile_error_about_an_assembled_base_table_has_no_location() {
    // The base table of a query assembled from sections is supplied directly, not parsed, so there
    // is nowhere in the source to point to.
    let compiler = Compiler::new(&get_test_resource("issue_schema.json"), options()).unwrap();
    let query = crate::ast::Query::from_parts(
        "nope".to_string(),
        Default::default(),
        crate::parse_conditions("status:open").unwrap(),
        vec![],
        vec![],
    );
    let err = compiler.compile_query(query).unwrap_err();
    assert_eq!(err.message, "Base table `nope` does not exist.");
    assert_eq!((err.span, err.section), (None, None));
}

/// An error within a computed column's definition is located in the definition, wherever the
/// computed column is used. (When it was reached through a scope, the error's span used to run from
/// the scope's path in the query to the end of the path in the definition: two unrelated places,
/// the first of which comes after the second.)
#[test]
fn error_in_a_computed_column_reached_through_a_scope_is_located_in_the_definition() {
    let definition = "#issues.is_open = status[open reopened]\n";

    let input = format!("{definition}#issues `is_open` $id");
    assert_eq!(located(&input, &compile_error(&input)), "status");

    let input = format!("{definition}#comments issue{{`is_open`}} $id");
    let err = compile_error(&input);
    let span = err.span.unwrap();
    assert!(
        span.start.offset <= span.end.offset,
        "the span starts after it ends: {span:?}"
    );
    assert_eq!(located(&input, &err), "status");
}

// ------------------------------------------------------------------------------------------------
// Paths
// ------------------------------------------------------------------------------------------------

#[test]
fn path_to_unknown_table() {
    let err = compile_error("#issues ++#nope");
    assert_eq!(err.message, "Table `nope` not found.");
    assert_eq!(located("#issues ++#nope", &err), "#nope");
}

#[test]
fn path_to_unrelated_table() {
    let err = compile_with_schema(DIAMOND_SCHEMA, "#a ++#z").unwrap_err();
    assert_eq!(err.kind, ErrorKind::Compile);
    assert_eq!(located("#a ++#z", &err), "#z");
    assert_eq!(
        err.message,
        "The table `z` is not related to the table `a`, so `#z` can't be used here."
    );
}

#[test]
fn ambiguous_path_to_table() {
    let err = compile_with_schema(DIAMOND_SCHEMA, "#a ++#c{id:1}").unwrap_err();
    assert_eq!(err.kind, ErrorKind::Compile);
    // The location excludes the condition set, which isn't the problem.
    assert_eq!(located("#a ++#c{id:1}", &err), "#c");
    assert_eq!(
        err.message,
        "The table `c` is related to the table `a` in more than one equally direct way, so `#c` \
         is ambiguous here."
    );
}

#[test]
fn ambiguous_linking_column_suggests_naming_it() {
    // `blocks` refers to `issues` via both `blocker` and `blocking`. This used to silently pick one.
    let err = compile_error("#issues --#blocks");
    assert_eq!(located("#issues --#blocks", &err), "#blocks");
    assert_eq!(
        err.message,
        "The table `blocks` is related to the table `issues` in more than one equally direct way, \
         so `#blocks` is ambiguous here. Name the linking column to use, as in `#blocks(blocker)` \
         or `#blocks(blocking)`."
    );
}

#[test]
fn path_to_table_that_holds_one_related_record() {
    // Each issue has one project, so `#projects` (many related records) is a mistake. This used to
    // compile, silently yielding the `project` column.
    let err = compile_error("#issues $title $#projects");
    assert_eq!(err.kind, ErrorKind::Compile);
    assert_eq!(located("#issues $title $#projects", &err), "#projects");
    assert_eq!(
        err.message,
        "Each record of the table `issues` is related to only one record of the table `projects`, \
         but `#projects` refers to many related records, so it can't be used here."
    );
    assert_eq!(
        err.hint.as_deref(),
        Some(
            "To refer to the one related record, follow the linking column instead, as in \
             `project`."
        )
    );

    // The suggestion follows every link of the route.
    let err = compile_error("#issues ++#clients");
    assert_eq!(
        err.hint.as_deref(),
        Some(
            "To refer to the one related record, follow the linking column instead, as in \
             `project.product.client`."
        )
    );

    // Naming the linking column doesn't make one record into many.
    let err = compile_error("#issues $#users(author)");
    assert_eq!(
        err.message,
        "Each record of the table `issues` is related to only one record of the table `users`, \
         but `#users(author)` refers to many related records, so it can't be used here."
    );
}

#[test]
fn path_to_table_skips_routes_to_one_record() {
    // An issue has one author, but that's not what `#users` means. The routes to *many* users go
    // through `assignments` and through `comments`, and neither is shorter than the other.
    let err = compile_error("#issues $#users");
    assert_eq!(located("#issues $#users", &err), "#users");
    assert_eq!(
        err.message,
        "The table `users` is related to the table `issues` in more than one equally direct way, \
         so `#users` is ambiguous here."
    );

    // When only one such route is shortest, it's the one taken, however short the route to the one
    // related record is.
    assert_eq!(
        compile("#comments $#users").unwrap(),
        compile("#comments $issue.#assignments.#users").unwrap(),
    );

    // Once a path leads to many records, a later `#table` part may follow a link to one record.
    assert!(compile("#issues $#assignments.#users").is_ok());
    assert!(compile("#issues $#comments.#users.id%distinct").is_ok());
}

#[test]
fn path_via_unknown_linking_column() {
    let err = compile_error("#issues $#blocks(nope)");
    assert_eq!(located("#issues $#blocks(nope)", &err), "#blocks(nope)");
    assert_eq!(
        err.message,
        "The table `blocks` is not related to the table `issues` via a foreign key column named \
         `nope`, so `#blocks(nope)` can't be used here."
    );
}

#[test]
fn path_from_table_to_itself_without_self_reference() {
    let err = compile_error("#users $#users");
    assert_eq!(located("#users $#users", &err), "#users");
    assert_eq!(
        err.message,
        "The table `users` has no foreign key that refers to `users` itself, so `#users` can't be \
         used here. To reach other `users` records through a related table, name that table \
         first, as in `#other_table.#users`."
    );
}

#[test]
fn table_with_one_path_is_an_error_rather_than_a_crash() {
    // This syntax parses, but isn't implemented by the compiler. It used to panic (via `todo!()`),
    // which in the WebAssembly build aborts the whole module.
    let err = compile_error("#issues $author.>>users");
    assert_eq!(located("#issues $author.>>users", &err), ">>users");
    assert_eq!(
        err.message,
        "Paths to a single related record via `>>` (as in `>>users`) are not yet supported."
    );
}

/// A table produced by a pipeline stage or a user-defined table has no relationships, so a to-many
/// path from one can't be resolved. (This used to panic, on an `unwrap` while clarifying the path.)
#[test]
fn to_many_path_from_a_table_without_relationships_is_an_error_rather_than_a_crash() {
    for (input, at) in [
        ("#issues $id $author ~~~ ++#comments", "#comments"),
        ("#issues $id $author ~~~ $#comments", "#comments"),
        ("#t = #( #issues $id $author )\n#t ++#comments", "#comments"),
        (
            "#t = #( #issues $id $author )\n#t $id $#comments.created_at%max",
            "#comments",
        ),
    ] {
        let err = compile_error(input);
        assert_eq!(located(input, &err), at, "for {input}");
        assert!(
            err.message.starts_with(
                "`#comments` can't be used here, because the result of an earlier pipeline stage"
            ),
            "for {input}: {}",
            err.message
        );
    }
}

// ------------------------------------------------------------------------------------------------
// Sorting within aggregate functions
// ------------------------------------------------------------------------------------------------

#[test]
fn list_sorted_by_a_per_record_expression_is_an_error() {
    // `list` removes duplicates, which may disagree on a per-record sort key. This used to fall
    // back silently to a non-distinct `array_agg`.
    for (input, sort) in [
        (
            r"#issues $#comments.user.username%list(\\created_at)",
            r"\\created_at",
        ),
        (r"#issues $#issue_labels.label.name%list(\\id)", r"\\id"),
        (
            r"#issues $#comments.user.username%list(\s \\created_at \d)",
            r"\\created_at \d",
        ),
        (r"#issues $status \g $title%list(\\id)", r"\\id"),
    ] {
        let err = compile_error(input);
        assert_eq!(located(input, &err), sort, "for {input}");
        assert!(
            err.message
                .starts_with("The `list` function removes duplicate values")
                && err.message.contains("use `list_all` instead"),
            "for {input}: {}",
            err.message
        );
    }
}

#[test]
fn per_record_sorts_remain_allowed_outside_list() {
    assert!(compile(r"#issues $#comments.user.username%list_all(\\created_at)").is_ok());
    assert!(compile(r"#issues $#comments.user.username%list_all(\s)").is_ok());
}

#[test]
fn aggregated_sort_key_outside_list_is_an_error() {
    for (input, sort) in [
        (
            r"#issues $#comments.user.username%list_all(\\created_at%min)",
            r"\\created_at%min",
        ),
        (r"#issues $#comments.created_at%max(\\%count)", r"\\%count"),
    ] {
        let err = compile_error(input);
        assert_eq!(located(input, &err), sort, "for {input}");
        assert!(
            err.message
                .starts_with("Aggregated sorting expressions (e.g. `\\\\created_at%min` or"),
            "for {input}: {}",
            err.message
        );
    }
}

#[test]
fn aggregated_sort_key_in_grouped_list_is_an_error() {
    let input = r"#issues $status \g $title%list(\\id%min)";
    let err = compile_error(input);
    assert_eq!(located(input, &err), r"\\id%min");
    assert!(
        err.message
            .contains("not when listing a column of grouped records"),
        "{}",
        err.message
    );
}

#[test]
fn top_without_sorting_is_an_error() {
    let input = r"#issues $#comments.user.username%top";
    let err = compile_error(input);
    assert_eq!(located(input, &err), r"#comments.user.username%top");
    assert!(
        err.message
            .starts_with("The `top` function collects the values that tie for first place"),
        "{}",
        err.message
    );
}

#[test]
fn top_sorted_by_a_per_record_expression_is_an_error() {
    for input in [
        r"#issues $#comments.user.username%top(\\created_at)",
        r"#issues $#comments.user.username%top(\\%count \d \\created_at)",
    ] {
        let err = compile_error(input);
        assert_eq!(located(input, &err), r"\\created_at", "for {input}");
        assert!(
            err.message
                .starts_with("The `top` function removes duplicate values"),
            "for {input}: {}",
            err.message
        );
    }
}

#[test]
fn top_in_grouped_query_is_an_error() {
    for input in [
        r"#issues $status \g $title%top(\sd)",
        r"#issues $status \g $title%top(\\id%min)",
    ] {
        let err = compile_error(input);
        assert!(
            located(input, &err).starts_with("title%top("),
            "for {input}"
        );
        assert!(
            err.message
                .contains("not when collecting a column of grouped records"),
            "for {input}: {}",
            err.message
        );
    }
}

#[test]
fn value_sort_on_a_standalone_aggregate_is_an_error() {
    let err = compile_error(r"#issues $%count(\s)");
    assert_eq!(located(r"#issues $%count(\s)", &err), r"\s");
    assert_eq!(
        err.message,
        "`\\s` sorts by the value being aggregated, but `%count` is used on its own, without a value."
    );
}

#[test]
fn sort_by_a_path_to_the_listed_value_keeps_distinct() {
    // A sort path that reaches the aggregated column via the same route is a sort by the value
    // itself, so `list` keeps its `DISTINCT` (rather than joining the table a second time).
    let sql = compile(r"#issues $#comments.user.username%list(\\user.username \d)").unwrap();
    assert!(
        sql.contains(
            r#"array_agg(DISTINCT "users"."username" ORDER BY "users"."username" DESC NULLS LAST)"#
        ),
        "{sql}"
    );
    assert!(!sql.contains("users_1"), "{sql}");
    // Flexible identifier resolution means letter case doesn't defeat the match.
    let sql = compile(r"#issues $#comments.user.username%list(\\User.UserName)").unwrap();
    assert!(sql.contains("array_agg(DISTINCT"), "{sql}");
}

// ------------------------------------------------------------------------------------------------
// Window functions where SQL doesn't permit them
// ------------------------------------------------------------------------------------------------

/// Asserts that each input fails to compile with a message starting with `prefix`, located at the
/// given text. These all used to compile to SQL that the database rejected.
#[track_caller]
fn assert_window_errors(inputs: &[(&str, &str)], prefix: &str) {
    for (input, location) in inputs {
        let err = compile_error(input);
        assert!(
            err.message.starts_with(prefix),
            "for {input}: {}",
            err.message
        );
        assert_eq!(located(input, &err), *location, "for {input}");
    }
}

#[test]
fn window_reached_indirectly_from_a_condition_is_an_error() {
    assert_window_errors(
        &[
            (
                "#comments.n = %%(issue\\p user\\p)%count\n#comments n:3",
                "%%(issue\\p user\\p)%count",
            ),
            (
                "#issues.latest:@x = %%(project\\p created_at\\sd)%row_number:@x\n#issues latest:1",
                "%%(project\\p created_at\\sd)%row_number",
            ),
            (
                "#issues project{%%(product\\p)%count:3}",
                "%%(product\\p)%count",
            ),
        ],
        "A window function can only be used in a condition when it's written directly",
    );
}

#[test]
fn window_with_related_records_is_an_error() {
    assert_window_errors(
        &[
            (
                "#comments.n = %%(issue\\p user\\p)%count\n#issues $#comments{n:3}.user.username%list",
                "%%(issue\\p user\\p)%count",
            ),
            (
                "#comments.n = %%(issue\\p user\\p)%count\n#issues $#comments.n%max",
                "%%(issue\\p user\\p)%count",
            ),
            (
                "#issues ++#comments{%%(user\\p)%count:3}",
                "%%(user\\p)%count",
            ),
            (
                r"#issues $#comments.user.username%list_all(\\%%(user\p)%count)",
                r"%%(user\p)%count",
            ),
        ],
        "Window functions can't be used with related records",
    );
}

#[test]
fn window_within_an_aggregate_is_an_error() {
    assert_window_errors(
        &[(
            "#comments.n = %%(issue\\p user\\p)%count\n#comments $issue \\g $n%max",
            "%%(issue\\p user\\p)%count",
        )],
        "Window functions can't be used within an aggregate function",
    );
}

#[test]
fn window_within_a_window_is_an_error() {
    assert_window_errors(
        &[
            (
                r"#issues $%%(project\p)%max(%%(project\p)%count)",
                r"%%(project\p)%count",
            ),
            (
                r"#issues $%%(%%(project\p)%count\p)%row_number",
                r"%%(project\p)%count",
            ),
        ],
        "Window functions can't be nested within another window function",
    );
}

#[test]
fn window_in_a_grouped_stage_is_an_error() {
    assert_window_errors(
        &[
            (r"#issues \\%%()%rank $project \g $%count", "%rank"),
            (r"#issues $%%(project\p)%count \g $%count", "%count"),
        ],
        "A query can't combine `\\g` grouping with a window function",
    );
}

#[test]
fn window_conditions_combine_with_grouping() {
    // Window conditions filter the rows before they're grouped, so they work in a grouped stage.
    assert!(
        compile(r"#issues %%(project\p created_at\sd)%row_number:1 $status \g $%count").is_ok()
    );
}

// ------------------------------------------------------------------------------------------------
// Conditions and values that SQL would reject
// ------------------------------------------------------------------------------------------------

/// Asserts that each input fails to compile with an error located at the given text, whose message
/// starts with `prefix`.
#[track_caller]
fn assert_errors(inputs: &[(&str, &str)], prefix: &str) {
    for (input, at) in inputs {
        let err = compile_error(input);
        assert_eq!(located(input, &err), *at, "for {input}");
        assert!(
            err.message.starts_with(prefix),
            "for {input}: {}",
            err.message
        );
    }
}

#[test]
fn aggregate_of_the_queried_records_in_a_condition_is_an_error() {
    // SQL doesn't permit an aggregate in `WHERE`.
    assert_errors(
        &[
            (r"#issues %count:>5 $status \g", "%count"),
            (r"#issues created_at%max:<1w $status \g", "created_at%max"),
            (
                r"#issues !author.last_seen%max:@null $id",
                "author.last_seen%max",
            ),
            // In a later stage of a pipeline, and in a condition of a window-wrapped stage.
            (r"#issues $id ~~~ id%max:5", "id%max"),
            (r"#issues id%max:5 %%(project\p)%row_number:1 $id", "id%max"),
            // Reached through a computed column or a function's body.
            (
                "#issues.newest = created_at%max\n#issues newest:<1w",
                "created_at%max",
            ),
            ("@@many = @n => %count:>@n\n#issues 5|many", "%count"),
        ],
        "An aggregate of the records being queried (like `%count` or `created_at%max`) can't be \
         used in a condition",
    );
    // An aggregate of *related* records is fine in a condition, as is an aggregate of the queried
    // records anywhere else.
    assert!(compile("#issues #comments.created_at%max:<1w $id").is_ok());
    assert!(compile(r"#issues $status \g $%count:>5 $created_at%max:<1w").is_ok());
    assert!(compile(r"#issues \\%count \d $status \g").is_ok());
    // That includes the sorting of an aggregate within a condition.
    assert!(compile(r"#issues #comments.user.username%list(\\%count \d):x $id").is_ok());
}

#[test]
fn aggregate_of_related_records_in_their_own_filter_is_an_error() {
    assert_errors(
        &[
            ("#projects ++#issues{%count:>5}", "%count"),
            ("#projects $#issues{created_at%max:<1w}", "created_at%max"),
            (
                "#clients $#projects.#issues{author.last_seen%max:<1w}.title%list",
                "author.last_seen%max",
            ),
        ],
        "An aggregate of the related records themselves (like `%count` or `created_at%max`) \
         can't be used within their own filter",
    );
    // An aggregate of records related to *them* is fine.
    assert!(compile("#projects ++#issues{#comments:>5}").is_ok());
    assert!(compile("#projects ++#issues{#comments.created_at%max:<1w}").is_ok());
}

#[test]
fn literal_value_as_a_condition_is_an_error() {
    assert_errors(
        &[("#issues 5", "5"), ("#issues id:1,2 $id", "2")],
        "A number can't stand alone as a condition.",
    );
    assert_errors(
        &[("#issues [@2024-01-01 title:x]", "@2024-01-01")],
        "A date can't stand alone as a condition.",
    );
    // A literal that arrives through a constant is no more a condition.
    assert_errors(
        &[("@n = 1w\n#issues @n", "@n")],
        "A duration can't stand alone as a condition.",
    );
    // A string is a search term, also through a constant.
    assert!(compile("@term = \"foo\"\n#issues @term \"bar\" !@term").is_ok());
}

#[test]
fn negated_value_is_an_error() {
    assert_errors(
        &[
            ("#issues status:[open !closed] $id", "!closed"),
            ("#issues id:[1 !2] $id", "!2"),
            ("#issues $!\"x\"", "!\"x\""),
            (
                "@closed = \"closed\"\n#issues status:[open !@closed]",
                "!@closed",
            ),
        ],
        "`!` negates a condition, but this is a value.",
    );
    // Negated conditions within a set are fine, as are negated search terms.
    assert!(compile("#projects is_active:[@true !`is_pro_bono`] $id").is_ok());
    assert!(compile("#issues [status:open !status:closed] !foo [bar !baz] $id").is_ok());
}

#[test]
fn set_of_values_that_is_not_an_expansion_is_an_error() {
    assert_errors(
        &[
            // Held by a constant.
            (
                "@statuses = [open reopened]\n#issues status:@statuses $id",
                "@statuses",
            ),
            (
                "@statuses = [open reopened]\n#issues [title status]:@statuses",
                "@statuses",
            ),
            // Passed as an argument.
            ("@@f = @v => status:@v\n#issues [open closed]|f", "@v"),
            // Wrapped in parentheses, or joined by a comma within them.
            ("#issues title:([open closed])", "([open closed])"),
            ("#issues id:(1,2)", "(1,2)"),
            // Passed to a custom comparison.
            (
                "@statuses = [open reopened]\n\
                 #issues.comment:@x = ++#comments{body:@x}\n\
                 #issues comment:@statuses",
                "@x",
            ),
        ],
        "A set of values can only expand a comparison when it's written directly in the comparison",
    );
    // A set held by a constant is still a condition in its own right.
    assert!(compile("@terms = [open reopened]\n#issues @terms !@terms $id").is_ok());
    // And a set of conditions can still be compared, as the boolean it is.
    assert!(compile("@ok = [is_active is_pro_bono]\n#projects is_active:@ok").is_ok());
}

// ------------------------------------------------------------------------------------------------
// Variables
// ------------------------------------------------------------------------------------------------

#[test]
fn constant_defined_in_terms_of_itself_is_an_error() {
    for input in [
        "@a = @a + 1\n#issues $@a",
        "@a = @b\n@b = @a\n#issues $@a",
        "@a = @b\n@b = @a\n#issues title:@a",
    ] {
        let err = compile_error(input);
        assert!(
            err.message.contains("is defined in terms of itself"),
            "for {input}: {}",
            err.message
        );
    }
}

#[test]
fn recursive_function_is_an_error() {
    for input in [
        "@@f = @x => @x|f\n#issues $id|f",
        "@@f = @x => @x|g\n@@g = @x => @x|f\n#issues $id|f",
        // Recursion through a local assignment, which is expanded lazily.
        "@@f = @x =>\n  @s = @x|f\n  @s\n#issues $id|f",
    ] {
        let err = compile_error(input);
        assert!(
            err.message.contains("functions can't be recursive"),
            "for {input}: {}",
            err.message
        );
    }
    // Applying a function to its own result isn't recursion.
    assert!(compile("@@inc = @x => @x + 1\n#issues $id|inc|inc|inc $@@inc(@@inc(id))").is_ok());
}

#[test]
fn definitions_do_not_see_the_variables_of_their_callers() {
    // A named function's body sees the constants and its own parameters, not the variables that
    // happen to be in scope where it's called.
    let input = "@@f = @y => @x + @y\n#issues $id|(@x => @x|f)";
    let err = compile_error(input);
    assert_eq!(located(input, &err), "@x");
    assert_eq!(err.message, "Unknown variable `@x`.");
    // Likewise a computed column and a custom comparison.
    for input in [
        "#issues.n = id + @x\n#issues $id|(@x => n)",
        "#issues.near:@v = id:@v + @x\n#issues 1|(@x => near:2)",
    ] {
        let err = compile_error(input);
        assert_eq!(err.message, "Unknown variable `@x`.", "for {input}");
    }
    // An anonymous function, being written within an expression, does see what's around it.
    assert!(compile("#issues $id|(@x => @x|(@y => @x + @y))").is_ok());
}

#[test]
fn argument_whose_record_cannot_be_reached_is_an_error() {
    assert_errors(
        &[
            // Two levels of related records away from the record the argument belongs to.
            (
                "@@f = @u => ++#issues{++#comments{user:@u}}\n#projects product|f $id",
                "@u",
            ),
            // Related records of a different record: the author's comments, not the issue's.
            (
                "@@f = @u => ++author.#comments{issue:@u}\n#issues id|f $id",
                "@u",
            ),
            (
                "#users.wrote:@x = ++#comments{body:@x}\n#issues author.wrote:`title` $id",
                "@x",
            ),
        ],
        "A value passed to a function or a custom comparison is being used somewhere that its \
         record can't be reached from",
    );
    // A value that doesn't refer to its record can be used anywhere.
    assert!(
        compile("@@f = @u => ++#issues{++#comments{user:@u}}\n#projects 5|f @now|f $id").is_ok()
    );
}

// ------------------------------------------------------------------------------------------------
// Unions
// ------------------------------------------------------------------------------------------------

#[test]
fn union_member_with_unknown_base_table_is_located_at_it() {
    let input = "#issues $id +++ #nope $id";
    let err = compile_error(input);
    assert_eq!(err.message, "Base table `nope` does not exist.");
    assert_eq!(located(input, &err), "#nope");
}

#[test]
fn union_members_must_have_the_same_number_of_columns() {
    let input = "#issues $id $title +++ #projects $id +++ #users $id $username";
    let err = compile_error(input);
    assert_eq!(
        err.message,
        "Each query in a union (`+++`) must have the same number of result columns. This query \
         has 1 column, but the first has 2 columns."
    );
    assert_eq!(located(input, &err), "#projects");
    // Columns emitted by a glob, or by default, count too.
    let input = "#issues $id +++ #projects";
    let err = compile_error(input);
    assert_eq!(located(input, &err), "#projects");
    assert!(err.message.contains("has 5 columns"), "{}", err.message);
    compile("#projects $* +++ #projects").unwrap();
}

#[test]
fn union_members_cannot_be_sorted() {
    let message = "A query within a union (`+++`) can't be sorted, because the union doesn't keep \
                   the order of its queries' rows. To sort the union's result, add a pipeline \
                   stage after it, e.g. `~~~ \\\\created_at`.";
    for (input, at) in [
        (r"#issues \\created_at $id +++ #projects $id", "created_at"),
        (r"#issues $id +++ #projects $id \s", "id"),
        (
            r"#issues $id +++ #projects $*(name \sd) +++ #users $id",
            "name",
        ),
    ] {
        let err = compile_error(input);
        assert_eq!(err.message, message, "{input}");
        assert_eq!(located(input, &err), at, "{input}");
    }
    // Sorting the union's result in a later stage is fine.
    compile(r"#issues $id +++ #projects $id ~~~ \\id").unwrap();
}

// ------------------------------------------------------------------------------------------------
// Arguments of built-in functions
// ------------------------------------------------------------------------------------------------

#[test]
fn function_with_optional_arguments_given_too_few_or_too_many_is_an_error() {
    assert_errors(
        &[
            (r"#issues $id|round(1 2)", "|round"),
            (r"#issues $title|regex_extract", "|regex_extract"),
            (r#"#issues $title|regex_replace("a")"#, "|regex_replace"),
            (
                r#"#issues $title|regex_replace("a" "b" "c" "d")"#,
                "|regex_replace",
            ),
        ],
        "Expected between ",
    );
    assert_errors(
        &[(r"#issues $@@array()", "@@array")],
        "Expected at least one argument.",
    );
}

#[test]
fn function_argument_that_must_be_a_literal_is_an_error_otherwise() {
    assert_errors(
        &[
            (r#"#issues $title|regex_replace("a" "b" status)"#, "status"),
            (r#"#issues $title|regex_extract("a" 0 status)"#, "status"),
        ],
        "The `flags` argument must be a literal string, as in `\"cm\"`.",
    );
    assert_errors(
        &[(r#"#issues $title|regex_extract("a" id)"#, "id")],
        "The `group` argument must be a literal number, as in `1`.",
    );
    assert_errors(
        &[
            ("@g = -1\n#issues $title|regex_extract(\"a\" @g)", "@g"),
            (r#"#issues $title|regex_extract("a" 1.5)"#, "1.5"),
        ],
        "The `group` argument must be a whole number of 0 or more.",
    );
    // A literal that arrives through a constant or a parameter is still a literal.
    compile("@flags = \"cm\"\n#issues $title|regex_replace(\"a\" \"b\" @flags)").unwrap();
    compile("@@group = @x @n => @x|regex_extract(\"(a)\" @n)\n#issues $title|group(1)").unwrap();
}

#[test]
fn unknown_or_repeated_regex_flag_is_an_error() {
    assert_errors(
        &[(r#"#issues $title|regex_replace("a" "b" "cx")"#, r#""cx""#)],
        "Unknown regex flag `x`. The flags are `c` (case-sensitive),",
    );
    assert_errors(
        &[(r#"#issues $title|regex_extract("a" 0 "i")"#, r#""i""#)],
        "Regex matching is already case-insensitive, so there is no `i` flag.",
    );
    assert_errors(
        &[(r#"#issues $title|regex_replace("a" "b" "mcm")"#, r#""mcm""#)],
        "Repeated regex flag `m`.",
    );
}

// ------------------------------------------------------------------------------------------------
// Functions given as arguments
// ------------------------------------------------------------------------------------------------

#[test]
fn function_given_to_a_function_that_takes_none_is_an_error() {
    assert_errors(
        &[
            (r#"#issues $title|uppercase(@t => 1)"#, "@t => 1"),
            (r#"#issues $title|concat("a" @t => @t)"#, "@t => @t"),
            (r#"#issues $@@max(1 @t => 2)"#, "@t => 2"),
            (r#"#issues $title|(@x => @x)(@w => 1)"#, "@w => 1"),
            // A user-defined function, even one that doesn't use its parameter.
            ("@@one = @x @f => 1\n#issues $title|one(@w => 1)", "@w => 1"),
            // The functions that take one take it as their second argument only.
            (r#"#issues $@@map(@w => 1 @w => 2)"#, "@w => 1"),
        ],
        "A function (like `@e => @e:>0`) can only be given to `map`, `filter`, `some`, `every`, \
         or `reduce`",
    );
}

#[test]
fn function_that_takes_a_function_given_something_else_is_an_error() {
    assert_errors(
        &[
            (r#"#issues $title|split(" ")|map(1)"#, "1"),
            (r#"#issues $title|split(" ")|filter"#, "|filter"),
            (r#"#issues $title|split(" ")|some(@x)"#, "@x"),
        ],
        "`",
    );
    let err = compile_error(r#"#issues $title|split(" ")|every(`title`)"#);
    assert_eq!(
        err.message,
        "`every` needs a function to apply to each element of the array, as in \
         `every(@e => …)`."
    );
    assert_errors(
        &[(r#"#issues $title|split(" ")|map(@w => 1 2)"#, "|map")],
        "Expected exactly two arguments.",
    );
}

#[test]
fn function_with_the_wrong_number_of_parameters_is_an_error() {
    assert_errors(
        &[
            (
                r#"#issues $title|split(" ")|map(@a @b @c => 1)"#,
                "@a @b @c => 1",
            ),
            (
                r#"#issues $title|split(" ")|every(@a @b @c => @a)"#,
                "@a @b @c => @a",
            ),
        ],
        "The function given to `",
    );
    assert_errors(
        &[
            (r#"#issues $title|split(" ")|reduce(@a => @a)"#, "@a => @a"),
            (
                r#"#issues $title|split(" ")|reduce(@a @b @c => @a)"#,
                "@a @b @c => @a",
            ),
        ],
        "The function given to `reduce` takes two parameters: the value so far, and the next \
         element.",
    );
}

#[test]
fn element_parameter_within_a_query_of_its_own_is_an_error() {
    // Related records are found by a query of their own, which can't see the array.
    assert_errors(
        &[
            (
                r#"#issues $title|split(" ")|some(@w => #comments{body:@w})"#,
                "@w",
            ),
            (
                "#issues.has_comment:@x = ++#comments{body:@x}\n\
                 #issues $title|split(\" \")|some(@w => has_comment:@w)",
                "@w",
            ),
        ],
        "`@w` stands for an element of an array, so it can't be used within a filter of related \
         records",
    );
    // A related record, or an aggregate of related records that doesn't use the parameter, is
    // fine.
    compile(r#"#issues $title|split(" ")|some(@w => author{username:@w})"#).unwrap();
    compile(r#"#issues $title|split(" ")|some(@w => #comments.body%max:==@w)"#).unwrap();
}

#[test]
fn aggregate_or_window_within_a_function_argument_is_an_error() {
    assert_errors(
        &[
            (
                r#"#issues $title|split(" ")|some(@w => `title`%max:==@w)"#,
                "`title`%max",
            ),
            (
                r#"#issues $status \g $title%list|map(@t => %count)"#,
                "%count",
            ),
        ],
        "An aggregate of the records being queried (like `%count` or `created_at%max`) can't be \
         used within a function given to",
    );
    assert_errors(
        &[(
            r#"#issues $title|split(" ")|map(@w => %%(id\s)%row_number)"#,
            r"%%(id\s)%row_number",
        )],
        "Window functions can't be used within a function given to",
    );
    // The array may be an aggregate in a grouped query.
    compile(r#"#issues $status \g $title%list|map(@t => @t|uppercase)"#).unwrap();
}

#[test]
fn subquery_within_a_function_argument_is_an_error_in_duckdb() {
    let input = "@m = #( #issues $title%max )\n#issues $title|split(\" \")|some(@w => @w:==@m)";
    // Postgres evaluates the function in a subquery of its own, which may contain another.
    compile(input).unwrap();
    let options = Options {
        dialect: Box::new(crate::DuckDB()),
        ..options()
    };
    let compiler = Compiler::new(&get_test_resource("issue_schema.json"), options).unwrap();
    let err = compiler.compile(input.to_string()).unwrap_err();
    assert_eq!(err.kind, ErrorKind::Compile);
    assert_eq!(located(input, &err), "#( #issues $title%max )");
    assert_eq!(
        err.message,
        "In DuckDB, a function given to `map`, `filter`, `some`, `every`, or `reduce` can't use a \
         subquery (`#( … )`), because it becomes a DuckDB lambda, which can't contain one."
    );
}
