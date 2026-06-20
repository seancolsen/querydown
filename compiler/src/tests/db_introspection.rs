//! Runs each dialect's [`introspection_sql`](crate::Dialect::introspection_sql) query against a
//! real database and verifies the JSON it returns round-trips into a working [`Compiler`].
//!
//! Gated behind the `db-tests` cargo feature because it requires the `postgres` and `duckdb`
//! binaries on PATH (the dev container provides them — see DEVELOPMENT.md). Run with:
//!
//! ```text
//! cargo test --features db-tests
//! ```
//!
//! Unlike [`super::db_corpus`], this test needs a schema with real foreign-key and unique
//! constraints so it can exercise link discovery and the `unique` flag, so it builds its own small
//! schema rather than reusing the corpus DDL (which is intentionally constraint-free).

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;

use crate::{Compiler, Dialect, DuckDB, IdentifierResolution, Options, Postgres};

/// DDL accepted by both Postgres and DuckDB, with a non-unique FK (`users.team`) and a unique FK
/// (`profiles.user`) so we can assert both the link wiring and the `unique` flag.
const DDL: &str = "\
CREATE TABLE teams (id integer PRIMARY KEY, name text);
CREATE TABLE users (id integer PRIMARY KEY, name text, team integer REFERENCES teams(id));
CREATE TABLE profiles (id integer PRIMARY KEY, \"user\" integer UNIQUE REFERENCES users(id));
";

#[test]
fn test_postgres_introspection() {
    let env = PgEnv::setup();
    let json = env.run_query(Postgres().introspection_sql());
    drop(env);
    assert_introspection(&json, Box::new(Postgres()));
}

#[test]
fn test_duckdb_introspection() {
    let env = DuckEnv::setup();
    let json = env.run_query(DuckDB().introspection_sql());
    drop(env);
    assert_introspection(&json, Box::new(DuckDB()));
}

/// Shared assertions over the introspection JSON, independent of which engine produced it.
fn assert_introspection(json: &str, dialect: Box<dyn Dialect>) {
    let value: Value =
        serde_json::from_str(json).unwrap_or_else(|e| panic!("invalid JSON: {e}\n{json}"));

    let table_names: Vec<&str> = value["tables"]
        .as_array()
        .expect("`tables` should be an array")
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    for expected in ["teams", "users", "profiles"] {
        assert!(
            table_names.contains(&expected),
            "missing table {expected} in {json}"
        );
    }

    let links = value["links"]
        .as_array()
        .expect("`links` should be an array");
    let find_link = |table: &str, column: &str| -> &Value {
        links
            .iter()
            .find(|l| l["from"]["table"] == table && l["from"]["column"] == column)
            .unwrap_or_else(|| panic!("missing link {table}.{column} in {json}"))
    };

    let team_link = find_link("users", "team");
    assert_eq!(team_link["to"]["table"], "teams");
    assert_eq!(team_link["to"]["column"], "id");
    assert_eq!(team_link["unique"], Value::Bool(false));

    let profile_link = find_link("profiles", "user");
    assert_eq!(profile_link["to"]["table"], "users");
    assert_eq!(profile_link["unique"], Value::Bool(true));

    // The whole point: the introspected JSON must build a working compiler.
    let options = Options {
        dialect,
        identifier_resolution: IdentifierResolution::Flexible,
    };
    let compiler = Compiler::new(json, options).expect("introspected schema should compile");
    let result = compiler
        .compile("#users $team.name".to_string())
        .expect("query against introspected schema should compile");
    assert!(result.sql.contains("teams"));
}

// --- Postgres ---------------------------------------------------------------------------------

struct PgEnv {
    root: PathBuf,
    pg_data: PathBuf,
    pg_sock: PathBuf,
}

impl PgEnv {
    fn setup() -> Self {
        let root =
            std::env::temp_dir().join(format!("querydown_introspect_pg_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let pg_sock = root.join("sock");
        fs::create_dir_all(&pg_sock).expect("create socket dir");
        let env = PgEnv {
            pg_data: root.join("pgdata"),
            pg_sock,
            root,
        };
        run_checked(
            Command::new("initdb")
                .env("LC_ALL", "C")
                .env("LANG", "C")
                .arg("-D")
                .arg(&env.pg_data)
                .args([
                    "-U",
                    "postgres",
                    "--auth=trust",
                    "--no-sync",
                    "-E",
                    "UTF8",
                    "--locale=C",
                ]),
            "initdb",
        );
        run_checked(
            Command::new("pg_ctl")
                .arg("-D")
                .arg(&env.pg_data)
                .arg("-l")
                .arg(env.root.join("pg.log"))
                .arg("-o")
                .arg(format!("-k {} -h ''", env.pg_sock.display()))
                .args(["-w", "start"]),
            "pg_ctl start",
        );
        run_checked(env.psql().args(["-c", DDL]), "load DDL into postgres");
        env
    }

    fn psql(&self) -> Command {
        let mut cmd = Command::new("psql");
        cmd.arg("-h").arg(&self.pg_sock).args([
            "-U",
            "postgres",
            "-d",
            "postgres",
            "-v",
            "ON_ERROR_STOP=1",
            "-X",
            "-q",
            "-t",
            "-A",
        ]);
        cmd
    }

    fn run_query(&self, sql: &str) -> String {
        capture(self.psql().args(["-c", sql]))
    }
}

impl Drop for PgEnv {
    fn drop(&mut self) {
        let _ = Command::new("pg_ctl")
            .arg("-D")
            .arg(&self.pg_data)
            .args(["-m", "immediate", "-w", "stop"])
            .output();
        let _ = fs::remove_dir_all(&self.root);
    }
}

// --- DuckDB -----------------------------------------------------------------------------------

struct DuckEnv {
    root: PathBuf,
    file: PathBuf,
}

impl DuckEnv {
    fn setup() -> Self {
        let root =
            std::env::temp_dir().join(format!("querydown_introspect_duck_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create temp root dir");
        let env = DuckEnv {
            file: root.join("introspect.duckdb"),
            root,
        };
        run_checked(
            Command::new("duckdb").arg(&env.file).arg("-c").arg(DDL),
            "load DDL into duckdb",
        );
        env
    }

    fn run_query(&self, sql: &str) -> String {
        capture(
            Command::new("duckdb")
                .arg(&self.file)
                .args(["-noheader", "-list", "-c", sql]),
        )
    }
}

impl Drop for DuckEnv {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

// --- helpers ----------------------------------------------------------------------------------

fn run_checked(cmd: &mut Command, what: &str) {
    let output = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to run {what}: {e}"));
    if !output.status.success() {
        panic!(
            "{what} failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }
}

/// Runs a query command and returns its trimmed stdout, panicking on any error.
fn capture(cmd: &mut Command) -> String {
    let output = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn process: {e}"));
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() || stderr.contains("Error:") {
        panic!("query failed:\n{stderr}");
    }
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}
