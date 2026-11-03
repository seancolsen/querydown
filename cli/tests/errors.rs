//! Tests of how the CLI reports failures.

use std::process::{Command, Output};

const SCHEMA: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../compiler/resources/test/issue_schema.json"
);

fn querydown(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_querydown"))
        .args(args)
        .output()
        .expect("failed to run the CLI")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

#[test]
fn success_prints_sql_and_exits_zero() {
    let output = querydown(&["compile", "-s", SCHEMA, "#issues $id"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("SELECT"));
}

#[test]
fn syntax_error_prints_a_diagnostic_to_stderr() {
    let output = querydown(&["compile", "-s", SCHEMA, "#issues foo:)"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        [
            "syntax error: Unexpected `)`, which has no matching `(`. Expected a value.",
            " --> line 1, column 13",
            "  |",
            "1 | #issues foo:)",
            "  |             ^",
            "  |",
            "  = hint: Remove this `)`, or add a `(` where the group should begin.",
            "",
        ]
        .join("\n")
    );
}

#[test]
fn compile_error_prints_its_message_to_stderr() {
    let output = querydown(&["compile", "-s", SCHEMA, "#issues $nope"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "compile error: Column `nope` not found within table `issues`.\n"
    );
}

#[test]
fn json_format_prints_the_error_as_json_to_stdout() {
    let output = querydown(&["compile", "-s", SCHEMA, "-f", "json", "#issues $title->"]);
    assert_eq!(output.status.code(), Some(1));
    let json: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    let error = &json["error"];
    assert_eq!(error["kind"], "syntax");
    assert_eq!(
        error["message"],
        "Unexpected end of input. Expected an alias."
    );
    assert_eq!(error["span"]["start"]["line"], 1);
    assert_eq!(error["span"]["start"]["column"], 17);
    assert_eq!(error["span"]["start"]["offset"], 16);
    assert_eq!(error["expected"][0], "an alias");
    assert_eq!(error["found"], "end of input");
    assert!(error["hint"].is_null());
}

#[test]
fn missing_schema_file_is_reported_without_panicking() {
    let output = querydown(&["compile", "-s", "/nonexistent/schema.json", "#issues"]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = stderr(&output);
    assert!(
        stderr.starts_with("error: could not read the schema file `/nonexistent/schema.json`"),
        "{stderr}"
    );
    assert!(!stderr.contains("panicked"));
}
