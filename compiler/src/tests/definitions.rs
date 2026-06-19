//! Unit tests for pre-base-table definitions (constants, functions, and custom comparisons) that
//! assert behavior not captured by the SQL-string corpus — principally the error cases.

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
fn unknown_variable_is_rejected() {
    let err = compile("#issues author:@missing").unwrap_err();
    assert!(err.contains("missing"), "got: {err}");
}

#[test]
fn constant_is_inlined() {
    let sql = compile("@n = 1234\n#issues author:@n").unwrap();
    assert!(sql.contains("= 1234"), "got: {sql}");
}

#[test]
fn function_wrong_arg_count_is_rejected() {
    // `add` expects two parameters, but the call supplies only one (the piped-in value).
    let err = compile("@@add = @a @b => @a + @b\n#issues $id|add").unwrap_err();
    assert!(err.contains("argument"), "got: {err}");
}

#[test]
fn user_function_does_not_shadow_builtin() {
    // A user-defined function named like a built-in is ignored in favor of the built-in.
    let sql = compile("@@floor = @x => @x * 2\n#issues $id|floor").unwrap();
    assert!(sql.contains("FLOOR"), "got: {sql}");
    assert!(!sql.contains("* 2"), "got: {sql}");
}

#[test]
fn custom_comparison_definition_with_unknown_table_is_rejected() {
    let err = compile("#nonexistent.foo:@x = id:@x\n#issues $title").unwrap_err();
    assert!(err.contains("nonexistent"), "got: {err}");
}

#[test]
fn custom_comparison_operator_cannot_be_switched_when_body_uses_other_operators() {
    // The body uses `:=`, so the comparison must be called exactly as defined (with `:`). Calling
    // it with the regex operator is rejected.
    let input = "#issues.participant:@x = author.username:=@x\n#issues participant:~david";
    let err = compile(input).unwrap_err();
    assert!(err.contains("participant"), "got: {err}");
}

#[test]
fn custom_comparison_operator_can_be_switched_when_body_is_all_match() {
    // The body uses only `:`, so the comparison may be called with a switched operator.
    let input = "#issues.commenter:@x = ++#comments{user.username:@x}\n#issues commenter:=alice";
    assert!(compile(input).is_ok());
}

#[test]
fn sections_parsed_in_isolation_compile_like_a_whole_query() {
    // Parsing each section independently and reassembling with `Query::from_parts`, then compiling
    // via `compile_query`, produces the same SQL as compiling the equivalent whole-query string.
    use querydown_parser::ast::Query;
    use querydown_parser::{parse_conditions, parse_definitions, parse_display, parse_sorting};

    let schema_json = get_test_resource("issue_schema.json");
    let options = build_options(crate::options::IdentifierResolution::Flexible, "postgres");
    let compiler = Compiler::new(&schema_json, options).unwrap();

    let query = Query::from_parts(
        "issues".to_string(),
        parse_definitions("@n = 1234").unwrap(),
        parse_conditions("author:@n").unwrap(),
        parse_sorting(r"\\id \d").unwrap(),
        parse_display("$id $title").unwrap(),
    );
    let from_sections = compiler.compile_query(query).unwrap().sql;

    let whole = compile("@n = 1234\n#issues author:@n \\\\id \\d $id $title").unwrap();

    assert_eq!(from_sections, whole);
}
