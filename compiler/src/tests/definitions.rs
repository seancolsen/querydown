//! Unit tests for pre-base-table definitions (constants, functions, and custom comparisons) that
//! assert behavior not captured by the SQL-string corpus — principally the error cases.

use super::corpus_loader::build_options;
use super::get_test_resource;
use crate::{Compiler, Error};

fn compile_located(input: &str) -> Result<String, Error> {
    let schema_json = get_test_resource("issue_schema.json");
    let options = build_options(crate::options::IdentifierResolution::Flexible, "postgres");
    let compiler = Compiler::new(&schema_json, options).unwrap();
    compiler.compile(input.to_string()).map(|r| r.sql)
}

/// Compiles `input`, reporting only the message of any error.
fn compile(input: &str) -> Result<String, String> {
    compile_located(input).map_err(|e| e.message)
}

/// The byte offset in `input` at which compiling it fails, and the text located there.
fn error_location(input: &str) -> (usize, &str) {
    let err = compile_located(input).unwrap_err();
    let span = err.span.expect("the error should be located");
    (span.start.offset, &input[span.range()])
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

#[test]
fn constant_from_subquery_is_inlined_as_scalar_subquery() {
    // A constant defined with `#( ... )` inlines as a parenthesized scalar subquery wherever it is
    // referenced.
    let sql = compile("@latest = #( #comments $created_at%max )\n#issues created_at:>@latest $id")
        .unwrap();
    assert!(
        sql.contains(r#"> (SELECT"#) && sql.contains(r#"max("comments"."created_at")"#),
        "got: {sql}"
    );
}

#[test]
fn user_defined_table_is_usable_as_a_base_table() {
    // A user-defined table compiles to a CTE that the main query reads from as its base table.
    let sql = compile(
        "#project_counts = #( #issues $project \\g $%count -> issue_count )\n\
         #project_counts issue_count:>=10 $project",
    )
    .unwrap();
    assert!(sql.contains(r#""project_counts" AS ("#), "got: {sql}");
    assert!(sql.contains(r#"FROM "project_counts""#), "got: {sql}");
    assert!(
        sql.contains(r#""project_counts"."issue_count" >= 10"#),
        "got: {sql}"
    );
}

#[test]
fn user_defined_table_can_reference_an_earlier_one() {
    // A user-defined table may use a previously-defined user-defined table as its base table.
    let sql = compile(
        "#a = #( #issues $project \\g $%count -> n )\n\
         #b = #( #a n:>=5 $project )\n\
         #b $project",
    )
    .unwrap();
    assert!(sql.contains(r#""a" AS ("#), "got: {sql}");
    assert!(sql.contains(r#""b" AS ("#), "got: {sql}");
    assert!(sql.contains(r#"FROM "a""#), "got: {sql}");
    assert!(sql.contains(r#"FROM "b""#), "got: {sql}");
}

#[test]
fn user_defined_table_with_unknown_base_table_is_rejected() {
    let err = compile("#foo = #( #nonexistent $id )\n#foo $id").unwrap_err();
    assert!(err.contains("nonexistent"), "got: {err}");
}

// Self-referential definitions used to recurse until the stack overflowed, crashing the process (or,
// in the WebAssembly build, the whole module). They are now reported as errors.

#[test]
fn constant_defined_in_terms_of_itself_is_rejected() {
    let err = compile("@a = @a + 1\n#issues $@a").unwrap_err();
    assert_eq!(err, "The constant `@a` is defined in terms of itself.");
    let err = compile("@a = @b\n@b = @a\n#issues $@a").unwrap_err();
    assert_eq!(err, "The constant `@a` is defined in terms of itself.");
}

#[test]
fn self_reference_is_located_where_it_closes_the_cycle() {
    // The reference within the definition itself, rather than the use of the definition.
    assert_eq!(error_location("@a = 1 + @a\n#issues $@a"), (9, "@a"));
    assert_eq!(
        error_location("@@f = @x => @x|f\n#issues $id|f"),
        (14, "|f")
    );
    assert_eq!(
        error_location("#issues.foo = 1 + foo\n#issues $foo"),
        (18, "foo")
    );
}

#[test]
fn recursive_function_is_rejected() {
    let err = compile("@@f = @x => @x|f\n#issues $id|f").unwrap_err();
    assert_eq!(
        err,
        "The function `f` calls itself, but functions can't be recursive."
    );
    let err = compile("@@f = @x => @x|g\n@@g = @x => @x|f\n#issues $id|f").unwrap_err();
    assert_eq!(
        err,
        "The function `f` calls itself, but functions can't be recursive. (The cycle is `f` → `g` \
         → `f`.)"
    );
}

#[test]
fn computed_column_defined_in_terms_of_itself_is_rejected() {
    let err = compile("#issues.foo = foo + 1\n#issues $foo").unwrap_err();
    assert_eq!(
        err,
        "The computed column `foo` on the table `issues` is defined in terms of itself."
    );
    let err = compile("#issues.foo = bar\n#issues.bar = foo\n#issues $foo").unwrap_err();
    assert!(
        err.ends_with("(The cycle is `issues.foo` → `issues.bar` → `issues.foo`.)"),
        "got: {err}"
    );
    // Type inference also expands computed columns (e.g. to decide how `:` compares), and must not
    // recurse forever either.
    assert!(compile("#issues.foo = foo\n#issues foo:1").is_err());
}

#[test]
fn computed_column_defined_in_terms_of_itself_via_an_aggregate_is_rejected() {
    let input = "#projects.a = #issues.b%max\n#issues.b = project.a\n#projects $a";
    let err = compile(input).unwrap_err();
    assert!(
        err.ends_with("(The cycle is `projects.a` → `issues.b` → `projects.a`.)"),
        "got: {err}"
    );
    // The same cycle, entered while inferring the aggregate's type.
    assert!(compile("#projects.a = #issues.b%max\n#issues.b = project.a\n#projects a:1").is_err());
}

#[test]
fn computed_column_across_a_to_many_path_requires_an_aggregate() {
    let err = compile("#issues.big = id:>100\n#projects $#issues.big").unwrap_err();
    assert_eq!(err, "The column `big` requires an aggregate function.");
}

#[test]
fn unknown_column_across_a_to_many_path_is_still_rejected() {
    let err = compile("#issues.big = id:>100\n#projects $#issues.nope%max").unwrap_err();
    assert_eq!(err, "Column `nope` not found within table `issues`.");
}

#[test]
fn custom_comparison_defined_in_terms_of_itself_is_rejected() {
    let err = compile("#issues.c:@x = c:@x\n#issues c:1").unwrap_err();
    assert_eq!(
        err,
        "The custom comparison `c` on the table `issues` is defined in terms of itself."
    );
}

#[test]
fn default_text_search_defined_in_terms_of_itself_is_rejected() {
    // `zzz` is a bare word — i.e. a default text search — within the default text search's own
    // definition.
    let input = "#issues.__querydown_default_text_search:@x = [ title:@x zzz ]\n#issues hello";
    let err = compile(input).unwrap_err();
    assert!(
        err.starts_with(
            "The default text search for the table `issues` is defined in terms of itself."
        ),
        "got: {err}"
    );
}

#[test]
fn function_parameter_may_be_bound_to_a_variable_of_the_same_name() {
    // The argument `@x` refers to the constant, not to the parameter it is bound to. This used to
    // recurse forever.
    let sql = compile("@x = 1\n@@double = @x => @x * 2\n#issues $@x|double").unwrap();
    assert!(sql.contains("1 * 2"), "got: {sql}");
    let sql = compile("@@f = @x => @x|g\n@@g = @x => @x + 1\n#issues $id|f").unwrap();
    assert!(sql.contains(r#""issues"."id" + 1"#), "got: {sql}");
}
