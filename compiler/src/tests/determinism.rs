//! Tests that compiling the same query always produces the same SQL.

use std::collections::BTreeSet;

use super::corpus_loader::build_options;
use super::get_test_resource;
use crate::{options::IdentifierResolution, Compiler};

/// Compiles `input` many times over, returning each distinct SQL string produced.
fn distinct_outputs(input: &str) -> BTreeSet<String> {
    let schema_json = get_test_resource("issue_schema.json");
    (0..40)
        .map(|_| {
            let options = build_options(IdentifierResolution::Flexible, "postgres");
            let compiler = Compiler::new(&schema_json, options).unwrap();
            compiler.compile(input.to_string()).unwrap().sql
        })
        .collect()
}

/// When a query follows two or more to-one links out of the same table, the joins to the linked
/// tables should be rendered in the same order every time.
///
/// KNOWN FAILURE: sibling joins are kept in a `HashMap` (`JoinTree::dependents`), so their order
/// varies from one compilation to the next. Until that's fixed, the corpus can't hold any case
/// with sibling to-one joins, because its expected SQL would match only some of the time.
#[test]
fn sibling_joins_are_rendered_in_a_stable_order() {
    for input in [
        "#issues $author.username $project.name",
        "#issues author.username:alice project.name:website $id",
        "#comments issue{title:foo} user{username:bar} $id",
        "#issues $author.* $duplicate_of.*",
        "#projects $#issues{author.username:a project.name:b}",
    ] {
        let outputs = distinct_outputs(input);
        assert!(
            outputs.len() == 1,
            "{input} compiled to {} different SQL strings:\n\n{}",
            outputs.len(),
            outputs.into_iter().collect::<Vec<_>>().join("\n\n")
        );
    }
}
