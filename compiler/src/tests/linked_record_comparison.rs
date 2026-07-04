//! Unit tests for the linked-record comparison feature: a table may define a
//! `__querydown_linked_record_comparison` custom comparison that supplies the meaning of comparing
//! directly against a linked record of that table. These assertions cover behavior not captured by
//! the SQL-string corpus — principally that the shorthand produces exactly the expanded form, plus
//! disambiguation and fall-through cases.

use super::corpus_loader::build_options;
use super::get_test_resource;
use crate::Compiler;

const DEF: &str = "#users.__querydown_linked_record_comparison:@x = username:@x\n";

fn compile(input: &str) -> Result<String, String> {
    let schema_json = get_test_resource("issue_schema.json");
    let options = build_options(crate::options::IdentifierResolution::Flexible, "postgres");
    let compiler = Compiler::new(&schema_json, options).unwrap();
    compiler.compile(input.to_string()).map(|r| r.sql)
}

#[test]
fn shorthand_matches_the_expanded_form() {
    // With the linked-record comparison defined on `users`, `author:alice` must compile to exactly
    // the same SQL as the explicit `author.username:alice`.
    let shorthand = compile(&format!("{DEF}#issues author:alice $id")).unwrap();
    let expanded = compile("#issues author.username:alice $id").unwrap();
    assert_eq!(shorthand, expanded);
}

#[test]
fn operator_can_be_switched() {
    // The body is all-match (`:`), so the shorthand may be called with a switched operator, just
    // like any other custom comparison.
    let shorthand = compile(&format!("{DEF}#issues author:=alice $id")).unwrap();
    let expanded = compile("#issues author.username:=alice $id").unwrap();
    assert_eq!(shorthand, expanded);
    assert!(
        shorthand.contains(r#""users"."username" = 'alice'"#),
        "got: {shorthand}"
    );
}

#[test]
fn right_hand_expansion_binds_each_entry() {
    // An expansion on the right binds the parameter to each entry, exactly as the expanded form does.
    let shorthand = compile(&format!("{DEF}#issues author:[alice bob] $id")).unwrap();
    let expanded = compile("#issues author.username:[alice bob] $id").unwrap();
    assert_eq!(shorthand, expanded);
}

#[test]
fn multi_hop_linked_record_uses_the_comparison() {
    // The left side may be a multi-hop chain of to-one links. `duplicate_of` is a self-referential
    // FK on `issues`, so `duplicate_of.author` still lands on a single `users` record.
    let shorthand = compile(&format!("{DEF}#issues duplicate_of.author:alice $id")).unwrap();
    let expanded = compile("#issues duplicate_of.author.username:alice $id").unwrap();
    assert_eq!(shorthand, expanded);
}

#[test]
fn explicit_column_reference_is_unaffected() {
    // A comparison that already names a column on the linked record is untouched by the feature.
    let with_def = compile(&format!("{DEF}#issues author.email:alice $id")).unwrap();
    let without_def = compile("#issues author.email:alice $id").unwrap();
    assert_eq!(with_def, without_def);
}

#[test]
fn undefined_table_falls_back_to_default_behavior() {
    // Without the definition, `author:alice` keeps its ordinary meaning (comparing the raw foreign
    // key), so defining the comparison is what activates the shorthand.
    let with_def = compile(&format!("{DEF}#issues author:1 $id")).unwrap();
    let without_def = compile("#issues author:1 $id").unwrap();
    // The two differ: with the definition the comparison routes through `username`.
    assert_ne!(with_def, without_def);
    assert!(
        without_def.contains(r#""issues"."author""#),
        "got: {without_def}"
    );
}

#[test]
fn comparison_defined_on_one_table_does_not_leak_to_another() {
    // The comparison is registered on `users`, so a linked record of a different table (here
    // `project`, a `projects` record) is unaffected and keeps its default foreign-key behavior.
    let sql = compile(&format!("{DEF}#issues project:1 $id")).unwrap();
    assert!(sql.contains(r#""issues"."project""#), "got: {sql}");
    assert!(!sql.contains("username"), "got: {sql}");
}
