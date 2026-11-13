//! Tests of how string literals reach the SQL, where the corpus can't tell: it compares SQL with
//! whitespace removed.

use super::corpus_loader::build_options;
use super::get_test_resource;
use crate::Compiler;

fn compile(input: &str) -> String {
    let schema_json = get_test_resource("issue_schema.json");
    let options = build_options(crate::options::IdentifierResolution::Flexible, "postgres");
    let compiler = Compiler::new(&schema_json, options).unwrap();
    compiler.compile(input.to_string()).unwrap().sql
}

#[test]
fn multi_line_string_is_not_indented() {
    // The `WHERE` conditions are indented, but not the second line of the string within them.
    let sql = compile("#issues title:\"a\nb\" $id");
    assert!(sql.contains("('a\nb' COLLATE"), "got: {sql}");
}
