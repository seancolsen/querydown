//! Runs each dialect's [`introspection_sql`](crate::Dialect::introspection_sql) query against a
//! real database and verifies the JSON it returns round-trips into a working [`Compiler`].
//!
//! Gated behind the `db-tests` cargo feature. Postgres requires the `postgres` binaries on PATH
//! (the dev container provides them — see DEVELOPMENT.md); DuckDB is compiled and statically linked
//! via the bundled `duckdb` crate. Run with:
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

use serde_json::{json, Value};

use crate::{Compiler, Dialect, DuckDB, IdentifierResolution, Options, Postgres};

/// DDL accepted by both Postgres and DuckDB, with a non-unique FK (`users.team`) and a unique FK
/// (`profiles.user`) so we can assert both the link wiring and the `unique` flag. The `billing`
/// schema has tables too, one of which has the same name as a table in the current schema, and a
/// link between them.
const DDL: &str = "\
CREATE TABLE teams (id integer PRIMARY KEY, name text);
CREATE TABLE users (id integer PRIMARY KEY, name text, team integer REFERENCES teams(id));
CREATE TABLE profiles (id integer PRIMARY KEY, \"user\" integer UNIQUE REFERENCES users(id), settings json);
CREATE SCHEMA billing;
CREATE TABLE billing.users (id integer PRIMARY KEY, email text);
CREATE TABLE billing.invoices (id integer PRIMARY KEY, \"user\" integer REFERENCES billing.users(id));
";

/// A link from a table in another schema to one in the current schema, which Postgres allows.
const POSTGRES_DDL: &str = "\
CREATE TABLE billing.payments (id integer PRIMARY KEY, team integer REFERENCES public.teams(id));
";

/// Another database, attached to the current one, with tables in two schemas.
const DUCKDB_DDL: &str = "\
ATTACH ':memory:' AS archive;
CREATE TABLE archive.main.orders (id integer PRIMARY KEY, total integer);
USE archive;
CREATE TABLE lines (id integer, \"order\" integer REFERENCES orders(id));
USE memory;
CREATE SCHEMA archive.old;
CREATE TABLE archive.old.orders (id integer);
";

#[test]
fn test_postgres_introspection() {
    let env = PgEnv::setup("introspect", &format!("{DDL}{POSTGRES_DDL}"));
    let json = env.run_query(Postgres().introspection_sql());
    let compiler = assert_introspection(&json, Box::new(Postgres()));
    let link = link_from(&json, "payments", &["billing"], "team");
    assert_eq!(link["to"], json!({ "table": "teams", "column": "id" }));
    assert_eq!(link["unique"], Value::Bool(false));
    let sql = compiler
        .compile("#teams $name $#/billing/payments".to_string())
        .expect("a link to another schema should compile")
        .sql;
    assert!(sql.contains("\"billing\".\"payments\""), "{sql}");
    env.run_query(&format!("EXPLAIN {sql}"));
}

#[test]
fn test_duckdb_introspection() {
    let env = DuckEnv::setup(&format!("{DDL}{DUCKDB_DDL}"));
    let json = env.run_query(DuckDB().introspection_sql());
    let compiler = assert_introspection(&json, Box::new(DuckDB()));
    // The tables of an attached database are within its schemas.
    let value: Value = serde_json::from_str(&json).unwrap();
    let schemas_of = |name: &str| -> Vec<Value> {
        let mut schemas: Vec<Value> = value["tables"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|t| t["name"] == name)
            .map(|t| t["schema"].clone())
            .collect();
        schemas.sort_by_key(|schema| schema.to_string());
        schemas
    };
    assert_eq!(
        schemas_of("orders"),
        [json!(["archive", "main"]), json!(["archive", "old"])]
    );
    let link = link_from(&json, "lines", &["archive", "main"], "order");
    assert_eq!(
        link["to"],
        json!({ "table": "orders", "schema": ["archive", "main"], "column": "id" })
    );
    for query in [
        "#/archive/main/orders $total $#/archive/main/lines",
        "#/archive/old/orders $id",
    ] {
        let sql = compiler.compile(query.to_string()).unwrap().sql;
        env.prepare(&sql);
    }
}

/// The link from `table` (in `schema`, empty for the current schema) via `column`.
fn link_from(json: &str, table: &str, schema: &[&str], column: &str) -> Value {
    let value: Value = serde_json::from_str(json).unwrap();
    let schema_matches = |link: &Value| match &link["from"]["schema"] {
        Value::Null => schema.is_empty(),
        written => *written == json!(schema),
    };
    value["links"]
        .as_array()
        .expect("`links` should be an array")
        .iter()
        .find(|l| l["from"]["table"] == table && l["from"]["column"] == column && schema_matches(l))
        .unwrap_or_else(|| panic!("missing link {schema:?} {table}.{column} in {json}"))
        .clone()
}

/// Shared assertions over the introspection JSON, independent of which engine produced it.
/// Returns a compiler for the introspected schema.
fn assert_introspection(json: &str, dialect: Box<dyn Dialect>) -> Compiler {
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
        limit: None,
        offset: None,
    };
    let compiler = Compiler::new(json, options).expect("introspected schema should compile");
    let result = compiler
        .compile("#users $team.name".to_string())
        .expect("query against introspected schema should compile");
    assert!(result.sql.contains("teams"));

    // A JSON column is known to be JSON, so `to_json` leaves it as it is.
    let sql = compiler
        .compile("#profiles $settings|to_json".to_string())
        .expect("a query of a JSON column should compile")
        .sql;
    assert!(!sql.contains("CAST"), "{sql}");

    // A table in the current schema has no `schema`, and one in another schema has one. Only the
    // tables in the current schema are written without a schema.
    let tables = value["tables"].as_array().unwrap();
    let table = |name: &str, schema: Value| {
        tables
            .iter()
            .find(|t| t["name"] == name && t["schema"] == schema)
            .unwrap_or_else(|| panic!("missing table {schema} {name} in {json}"))
    };
    table("users", Value::Null);
    assert_eq!(
        table("users", json!(["billing"]))["columns"][1]["name"],
        "email"
    );
    let link = link_from(json, "invoices", &["billing"], "user");
    assert_eq!(
        link["to"],
        json!({ "table": "users", "schema": ["billing"], "column": "id" })
    );
    assert_eq!(link["unique"], Value::Bool(false));
    // The system's own schemas aren't included.
    for t in tables {
        let schema = t["schema"].to_string();
        assert!(
            !schema.contains("pg_") && !schema.contains("information_schema"),
            "unexpected table {schema} {} in {json}",
            t["name"]
        );
    }
    let sql = compiler
        .compile("#/billing/users $email $#/billing/invoices".to_string())
        .expect("a query of another schema should compile")
        .sql;
    assert!(
        sql.contains("FROM \"billing\".\"users\" AS \"users\""),
        "{sql}"
    );
    compiler
}

// --- Postgres ---------------------------------------------------------------------------------

/// A throwaway Postgres server, loaded with some DDL, which is stopped and removed on drop. Shared
/// with other DB tests that need a database of their own.
pub(super) struct PgEnv {
    root: PathBuf,
    pg_data: PathBuf,
    pg_sock: PathBuf,
}

impl PgEnv {
    /// Starts a server in a temp dir named after `label` and loads `ddl` into it.
    pub(super) fn setup(label: &str, ddl: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("querydown_{label}_pg_{}", std::process::id()));
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
        run_checked(env.psql().args(["-c", ddl]), "load DDL into postgres");
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

    /// Runs `sql`, returning its output (unaligned, without headers), and panicking on any error.
    pub(super) fn run_query(&self, sql: &str) -> String {
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

/// An in-memory DuckDB database, loaded with some DDL. Shared with other DB tests.
pub(super) struct DuckEnv {
    conn: duckdb::Connection,
}

impl DuckEnv {
    pub(super) fn setup(ddl: &str) -> Self {
        let conn = duckdb::Connection::open_in_memory().expect("open in-memory DuckDB");
        conn.execute_batch(ddl).expect("load DDL into duckdb");
        DuckEnv { conn }
    }

    /// Prepares `sql` (parsing, binding, and planning it), panicking on any error.
    pub(super) fn prepare(&self, sql: &str) {
        self.conn
            .prepare(sql)
            .unwrap_or_else(|e| panic!("duckdb couldn't prepare the query: {e}\n{sql}"));
    }

    /// Runs `sql`, which must yield a single row with a single text column, returning that text and
    /// panicking on any error.
    pub(super) fn run_query(&self, sql: &str) -> String {
        self.conn
            .query_row(sql, [], |row| row.get::<_, String>(0))
            .unwrap_or_else(|e| panic!("duckdb query failed: {e}\n{sql}"))
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
