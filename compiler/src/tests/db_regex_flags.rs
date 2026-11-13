//! Runs regex comparisons, with every combination of flags, against rows of real Postgres and DuckDB
//! databases, asserting that each dialect finds the same rows. This checks the _meaning_ of the
//! flags, which the corpus (whose DB test only plans queries) can't: the two engines' regex options
//! differ in name and default, and the compiler must map Querydown's flags onto each of them.
//!
//! Gated behind the `db-tests` cargo feature; see [`super::db_introspection`] for the environments.

use super::db_introspection::{DuckEnv, PgEnv};
use crate::{Compiler, Dialect, DuckDB, IdentifierResolution, Options, Postgres};

/// One table of text, some of it spanning lines, along with patterns to match it against.
const DDL: &str = "\
CREATE TABLE notes (id integer PRIMARY KEY, body text, pattern text);
INSERT INTO notes VALUES
  (1, 'Apple', NULL),
  (2, 'apple', NULL),
  (3, 'foo' || chr(10) || 'bar', 'foo.bar'),
  (4, 'FOO' || chr(10) || 'BAR', NULL);
";

/// Each condition on `#notes`, and the ids of the rows it should find.
const CASES: [(&str, &str); 15] = [
    // Case-insensitive unless `c` is given.
    (r"body:~apple", "1,2"),
    (r"body:\c~apple", "2"),
    (r"!body:\c~apple", "1,3,4"),
    // `.` matches a newline only with `s`.
    (r"body:~'foo.bar'", ""),
    (r"body:\s~'foo.bar'", "3,4"),
    (r"body:\cs~'foo.bar'", "3"),
    // `^` and `$` match at line breaks only with `m`.
    (r"body:~'^bar'", ""),
    (r"body:~'^foo$'", ""),
    (r"body:\m~'^bar'", "3,4"),
    (r"body:\m~'foo$'", "3,4"),
    (r"body:\cm~'^BAR$'", "4"),
    // Both together.
    (r"body:\ms~'^foo.bar$'", "3,4"),
    (r"body:\sm~'^o.b'", ""),
    // A pattern from a column gets the same options, and a NULL pattern matches nothing.
    (r"body:~`pattern`", ""),
    (r"body:\s~`pattern`", "3"),
];

#[test]
fn test_regex_flags_postgres() {
    let env = PgEnv::setup("regex_flags", DDL);
    check(Box::new(Postgres()), |sql| env.run_query(sql));
}

#[test]
fn test_regex_flags_duckdb() {
    let env = DuckEnv::setup(DDL);
    check(Box::new(DuckDB()), |sql| env.run_query(sql));
}

/// Compiles each case for `dialect` and runs it with `run`, which returns a query's lone text value.
fn check(dialect: Box<dyn Dialect>, run: impl Fn(&str) -> String) {
    let schema = run(dialect.introspection_sql());
    let options = Options {
        dialect,
        identifier_resolution: IdentifierResolution::Flexible,
        limit: None,
        offset: None,
    };
    let compiler = Compiler::new(&schema, options).expect("introspected schema should compile");
    let failures: Vec<String> = CASES
        .iter()
        .filter_map(|(condition, expected)| {
            let query = format!("#notes {condition} $id");
            let sql = compiler
                .compile(query.clone())
                .unwrap_or_else(|e| panic!("{query} failed to compile: {}", e.render(&query)))
                .sql;
            // Collapse the matching ids into one comma-separated value.
            let ids = run(&format!(
                "SELECT COALESCE(string_agg(CAST(id AS text), ',' ORDER BY id), '') FROM ({}) AS t",
                sql.trim_end().trim_end_matches(';')
            ));
            (ids != *expected)
                .then(|| format!("{query}\n  expected [{expected}], found [{ids}]\n{sql}"))
        })
        .collect();
    assert!(failures.is_empty(), "\n{}", failures.join("\n\n"));
}
