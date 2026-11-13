//! Queries for which the compiler currently emits SQL that a database rejects.
//!
//! Each of these is a meaningful query (not a type error on the author's part) that combines
//! features in a way the compiler doesn't handle. What it _should_ compile to is a design decision
//! in each case, so unlike the "Known failures" in the corpus, no SQL is expected here. The test
//! asserts only the invariant that the compiler must never hand a database SQL it will reject: each
//! query must either fail to compile, with an error that explains why, or compile to SQL that both
//! Postgres and DuckDB can plan.
//!
//! KNOWN FAILURE: every case here fails. When one is resolved, either move it to the corpus (if it
//! now compiles) or to [`super::errors`] (if it's now a compile error).
//!
//! Gated behind the `db-tests` cargo feature, like [`super::db_corpus`].

use super::corpus_loader::build_options;
use super::db_corpus::TestEnv;
use super::get_test_resource;
use crate::{options::IdentifierResolution, Compiler};

/// A description of each problem, and a query that demonstrates it.
const CASES: [(&str, &str); 11] = [
    (
        "In a grouped query, a count of related records is neither grouped nor aggregated across \
         the group. (Should it be an error, or count the records related to the whole group?)",
        r"#issues $status \g $#comments",
    ),
    (
        "In a grouped query, an aggregate of related records is neither grouped nor aggregated \
         across the group.",
        r"#issues $status \g $#comments.created_at%max",
    ),
    (
        "In a grouped query, a list of related records is neither grouped nor aggregated across \
         the group.",
        r"#issues $project \g $#labels.name%list",
    ),
    (
        "A condition on an aggregate of the base table is rendered in `WHERE`. (Should it be an \
         error, or become a `HAVING` clause?)",
        r"#issues %count:>5 $status \g",
    ),
    (
        "A condition comparing an aggregated date of the base table with a duration is rendered \
         in `WHERE`.",
        r"#issues created_at%max:<1w $status \g",
    ),
    (
        "A date compared with a duration that is computed rather than literal isn't recognized as \
         a date/duration comparison.",
        r"#issues created_at:<1w + 1d $id",
    ),
    (
        "A date column from an earlier pipeline stage is untyped, so comparing it with a duration \
         isn't recognized as a date/duration comparison.",
        r"#issues $id $created_at ~~~ created_at:<1w",
    ),
    (
        "A date column of a user-defined table is untyped, so comparing it with a duration isn't \
         recognized as a date/duration comparison.",
        "#recent = #( #issues $id $created_at )\n#recent created_at:<1w",
    ),
    (
        "A constant holding a condition set is compiled as a set of default text searches, rather \
         than expanding the comparison it's used in.",
        "@statuses = [open reopened]\n#issues status:@statuses $id",
    ),
    (
        "A negated entry within a right-hand expansion negates the value rather than the \
         comparison.",
        r"#issues status:[open !closed] $id",
    ),
    (
        "A comma after a comparison begins a new condition, so a value that follows it stands \
         alone as a condition.",
        r"#issues id:1,2 $id",
    ),
];

#[test]
fn compiled_sql_is_accepted_by_the_database() {
    let env = TestEnv::setup("invalid_sql");
    let schema_json = get_test_resource("issue_schema.json");
    let mut failures: Vec<String> = Vec::new();

    for (problem, input) in CASES {
        for dialect in ["postgres", "duckdb"] {
            let options = build_options(IdentifierResolution::Flexible, dialect);
            let compiler = Compiler::new(&schema_json, options).unwrap();
            // A compile error is an acceptable outcome.
            let Ok(result) = compiler.compile(input.to_string()) else {
                continue;
            };
            let outcome = match dialect {
                "postgres" => env.explain_postgres(&result.sql),
                _ => env.explain_duckdb(&result.sql),
            };
            if let Err(db_error) = outcome {
                failures.push(format!(
                    "{problem}\n\n{input}\n\n{dialect} rejects the compiled SQL:\n{db_error}\n\n{}",
                    result.sql
                ));
            }
        }
    }

    // Stop the server before asserting, as in the corpus test.
    drop(env);

    let divider = format!("\n{}\n", "─".repeat(44));
    assert!(
        failures.is_empty(),
        "{} queries compiled to SQL that a database rejects:{divider}{}",
        failures.len(),
        failures.join(&divider)
    );
}
