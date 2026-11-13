//! Runs conditions and their `!` negations against rows of real Postgres and DuckDB databases,
//! asserting that `!X` finds exactly the records `X` doesn't, NULLs included. The corpus (whose DB
//! test only plans queries) can show the SQL `!` compiles to, but not which rows it keeps.
//!
//! Gated behind the `db-tests` cargo feature; see [`super::db_introspection`] for the environments.

use super::db_introspection::{DuckEnv, PgEnv};
use crate::{Compiler, Dialect, DuckDB, IdentifierResolution, Options, Postgres};

/// Issues, some without a status or a project, in projects some of which have no `is_active`, with
/// comments on some of them. A recent comment is dated today, and an old one well over a week ago.
const DDL: &str = "\
CREATE TABLE projects (id integer PRIMARY KEY, name text, is_active boolean);
CREATE TABLE issues (
  id integer PRIMARY KEY,
  status text,
  project integer REFERENCES projects (id)
);
CREATE TABLE comments (
  id integer PRIMARY KEY,
  issue integer REFERENCES issues (id),
  created_at timestamp
);
INSERT INTO projects VALUES (1, 'Website', true), (2, 'Mobile', NULL);
INSERT INTO issues VALUES
  (1, 'open', 1),
  (2, 'closed', 1),
  (3, NULL, 1),
  (4, 'open', NULL),
  (5, 'closed', 2),
  (6, NULL, NULL);
INSERT INTO comments VALUES
  (1, 1, CAST(CURRENT_DATE AS timestamp)),
  (2, 2, TIMESTAMP '2000-01-01 00:00:00'),
  (3, 5, CAST(CURRENT_DATE AS timestamp));
";

/// The ids of every issue.
const ALL: &str = "1,2,3,4,5,6";

/// Each condition on `#issues`, and the ids of the issues it should find. Its negation should find
/// all the others.
const CASES: [(&str, &str); 14] = [
    // A nullable column.
    (r#"status:="open""#, "1,4"),
    (r"status:open", "1,4"),
    (r"status:~'^c'", "2,5"),
    // A nullable foreign key, compared directly and through a to-one path.
    (r"project:1", "1,2,3"),
    (r#"project.name:="Website""#, "1,2,3"),
    (r"project.name:~web", "1,2,3"),
    (r"project{id:1}", "1,2,3"),
    (r"project.id:[1 2]", "1,2,3,5"),
    // A nullable boolean, through a path, and negated by the `not` function as well.
    (r"project.is_active", "1,2,3"),
    (r"{project.is_active|not}", "4,5,6"),
    // Condition sets, where one operand being NULL can make the whole set NULL.
    (r#"[project.id:1 status:="closed"]"#, "1,2,3,5"),
    (r#"{project.id:1 status:="open"}"#, "1"),
    // An aggregate over no related records is NULL.
    (r"#comments.created_at%max:<1w", "1,5"),
    // A null test is never NULL itself.
    (r"project:@null", "4,6"),
];

#[test]
fn test_negation_postgres() {
    let env = PgEnv::setup("negation", DDL);
    check(Box::new(Postgres()), |sql| env.run_query(sql));
}

#[test]
fn test_negation_duckdb() {
    let env = DuckEnv::setup(DDL);
    check(Box::new(DuckDB()), |sql| env.run_query(sql));
}

/// Compiles each case and its negation for `dialect` and runs them with `run`, which returns a
/// query's lone text value.
fn check(dialect: Box<dyn Dialect>, run: impl Fn(&str) -> String) {
    let schema = run(dialect.introspection_sql());
    let options = Options {
        dialect,
        identifier_resolution: IdentifierResolution::Flexible,
        limit: None,
        offset: None,
    };
    let compiler = Compiler::new(&schema, options).expect("introspected schema should compile");
    let ids = |condition: &str| {
        let query = format!("#issues {condition} $id");
        let sql = compiler
            .compile(query.clone())
            .unwrap_or_else(|e| panic!("{query} failed to compile: {}", e.render(&query)))
            .sql;
        // Collapse the matching ids into one comma-separated value.
        let ids = run(&format!(
            "SELECT COALESCE(string_agg(CAST(id AS text), ',' ORDER BY id), '') FROM ({}) AS t",
            sql.trim_end().trim_end_matches(';')
        ));
        (query, sql, ids)
    };
    let failures: Vec<String> = CASES
        .iter()
        .flat_map(|(condition, expected)| {
            let complement = ALL
                .split(',')
                .filter(|id| !expected.split(',').any(|e| e == *id))
                .collect::<Vec<_>>()
                .join(",");
            [
                (condition.to_string(), expected.to_string()),
                (format!("!{condition}"), complement),
            ]
        })
        .filter_map(|(condition, expected)| {
            let (query, sql, found) = ids(&condition);
            (found != expected)
                .then(|| format!("{query}\n  expected [{expected}], found [{found}]\n{sql}"))
        })
        .collect();
    assert!(failures.is_empty(), "\n{}", failures.join("\n\n"));
}
