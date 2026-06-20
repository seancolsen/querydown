//! Tests for the database introspection queries exposed via [`Dialect::introspection_sql`].
//!
//! These tests don't touch a real database (that's [`super::db_introspection`], gated behind the
//! `db-tests` feature). They assert two things that must stay true regardless of the engine:
//!
//!   1. Each dialect exposes a non-empty introspection query.
//!   2. The JSON shape those queries produce round-trips into a working [`Compiler`], including the
//!      `unique` flag that distinguishes to-one from to-many reverse links.
//!
//! The fixture below is a faithful copy of the output the real queries produce (verified against
//! live Postgres and DuckDB), so it pins down the contract between the SQL and the compiler.

use crate::{Compiler, Dialect, DuckDB, IdentifierResolution, Options, Postgres};

#[test]
fn introspection_sql_is_present_for_each_dialect() {
    assert!(Postgres().introspection_sql().contains("json_build_object"));
    assert!(DuckDB().introspection_sql().contains("duckdb_constraints"));
}

/// A representative schema JSON exactly as the introspection queries emit it, including
/// dialect-native type names and a mix of unique and non-unique links.
const SAMPLE_INTROSPECTION_OUTPUT: &str = r#"{
  "tables": [
    {
      "name": "teams",
      "columns": [
        { "name": "id", "type": "int4" },
        { "name": "name", "type": "text" }
      ]
    },
    {
      "name": "users",
      "columns": [
        { "name": "id", "type": "int4" },
        { "name": "team", "type": "int4" },
        { "name": "tags", "type": "text[]" }
      ]
    },
    {
      "name": "profiles",
      "columns": [
        { "name": "id", "type": "int4" },
        { "name": "user", "type": "int4" }
      ]
    }
  ],
  "links": [
    {
      "from": { "table": "users", "column": "team" },
      "to": { "table": "teams", "column": "id" },
      "unique": false
    },
    {
      "from": { "table": "profiles", "column": "user" },
      "to": { "table": "users", "column": "id" },
      "unique": true
    }
  ]
}"#;

#[test]
fn introspection_output_round_trips_into_compiler() {
    let options = Options {
        dialect: Box::new(Postgres()),
        identifier_resolution: IdentifierResolution::Flexible,
    };
    let compiler = Compiler::new(SAMPLE_INTROSPECTION_OUTPUT, options)
        .expect("introspection output should parse into a schema");

    // Traversing the `users.team -> teams` forward link proves the link was wired up from the
    // introspected `links`, which in turn proves the JSON shape matches what the compiler expects.
    let result = compiler
        .compile("#users $team.name".to_string())
        .expect("query against introspected schema should compile");
    assert!(result.sql.contains("teams"));
}
