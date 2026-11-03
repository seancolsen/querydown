//! Tests for the errors the compiler reports: their kinds, messages, and locations.
//!
//! Syntax error messages are tested in depth in the parser crate; here we check that they surface
//! through [`Compiler::compile`] intact, along with the compiler's own (schema and compile) errors.

use crate::{options::IdentifierResolution, Compiler, Error, ErrorKind, Options, Postgres};

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

#[track_caller]
fn compile_error(input: &str) -> Error {
    let err = compile(input).expect_err("input should fail to compile");
    assert_eq!(err.kind, ErrorKind::Compile, "unexpected kind: {err:?}");
    assert_eq!(err.span, None);
    err
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
fn compile_error_has_a_message_but_no_location() {
    let err = compile_error("#issues $nope");
    assert_eq!(
        err.message,
        "Column `nope` not found within table `issues`."
    );
    assert_eq!(err.to_string(), err.message);
    assert_eq!(
        err.render("#issues $nope"),
        "compile error: Column `nope` not found within table `issues`."
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
}

// ------------------------------------------------------------------------------------------------
// Paths
// ------------------------------------------------------------------------------------------------

#[test]
fn path_to_unknown_table() {
    let err = compile_error("#issues ++#nope");
    assert_eq!(err.message, "Table `nope` not found.");
}

#[test]
fn path_to_unrelated_table() {
    let err = compile_with_schema(DIAMOND_SCHEMA, "#a ++#z").unwrap_err();
    assert_eq!(err.kind, ErrorKind::Compile);
    assert_eq!(
        err.message,
        "The table `z` is not related to the table `a`, so `#z` can't be used here."
    );
}

#[test]
fn ambiguous_path_to_table() {
    let err = compile_with_schema(DIAMOND_SCHEMA, "#a ++#c").unwrap_err();
    assert_eq!(err.kind, ErrorKind::Compile);
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
    assert_eq!(
        err.message,
        "The table `blocks` is not related to the table `issues` via a foreign key column named \
         `nope`, so `#blocks(nope)` can't be used here."
    );
}

#[test]
fn path_from_table_to_itself_without_self_reference() {
    let err = compile_error("#users $#users");
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
    for input in [
        r"#issues $#comments.user.username%list(\\created_at)",
        r"#issues $#issue_labels.label.name%list(\\id)",
        r"#issues $#comments.user.username%list(\s \\created_at \d)",
        r"#issues $status \g $title%list(\\id)",
    ] {
        let err = compile_error(input);
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
    for input in [
        r"#issues $#comments.user.username%list_all(\\created_at%min)",
        r"#issues $#comments.created_at%max(\\%count)",
    ] {
        let err = compile_error(input);
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
    let err = compile_error(r"#issues $status \g $title%list(\\id%min)");
    assert!(
        err.message
            .contains("not when listing a column of grouped records"),
        "{}",
        err.message
    );
}

#[test]
fn top_without_sorting_is_an_error() {
    let err = compile_error(r"#issues $#comments.user.username%top");
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
