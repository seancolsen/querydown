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
