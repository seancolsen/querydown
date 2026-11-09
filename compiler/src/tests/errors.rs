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
