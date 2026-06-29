//! Runs every corpus case's compiled SQL against real Postgres and DuckDB databases to catch SQL
//! syntax and query-planning bugs that the string-comparison corpus test ([`super::corpus`])
//! can't.
//!
//! Gated behind the `db-tests` cargo feature. Postgres requires the `postgres` binaries on PATH
//! (the dev container provides them — see DEVELOPMENT.md); DuckDB is compiled and statically linked
//! via the bundled `duckdb` crate. Run with:
//!
//! ```text
//! cargo test --features db-tests
//! ```
//!
//! No data is inserted; correctly typed but empty tables are enough to exercise parsing and
//! planning. Each case is compiled under *every* supported dialect and the result is validated
//! with `EXPLAIN`, so we never execute — reserving real execution for future data-driven E2E
//! tests. The schema is created once per engine and all queries run against that one session.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use testcase_markdown::TestCase;

use super::corpus_loader::{
    build_options, has_soloed, is_skipped, is_soloed, load_cases, report, Opts,
};
use super::{get_test_resource, get_test_resource_path};
use crate::Compiler;

/// Every dialect we generate SQL for. Each corpus case is compiled and validated against all of
/// them, regardless of the dialect the case declares for its string-match assertion.
const DIALECTS: [&str; 2] = ["postgres", "duckdb"];

/// The hand-authored DDL files (in resources/test) defining typed, empty tables. Both are loaded
/// into a single database per engine; table names don't collide across the two schemas.
const SCHEMA_DDL_FILES: [&str; 2] = ["issue_schema.sql", "library_schema.sql"];

#[test]
fn test_db_corpus() {
    let env = TestEnv::setup();

    let cases = load_cases();
    let has_soloed_tests = has_soloed(&cases);
    let mut failures = 0usize;

    for case in cases {
        if is_skipped(&case) {
            continue;
        }
        if has_soloed_tests && !is_soloed(&case) {
            continue;
        }
        for dialect in DIALECTS {
            if case.options.db_skip.skips(dialect) {
                continue;
            }
            if let Err(message) = check_case(&env, &case, dialect) {
                println!("{message}");
                failures += 1;
            }
        }
    }

    // Stop the server and clean up before asserting, so a failure doesn't leak the process or the
    // temp directory. (Drop would also handle it during unwind, but this keeps the ordering clear.)
    drop(env);

    if failures > 0 {
        panic!("{failures} DB corpus failure(s). See output above.");
    }
}

/// Compiles a case under `dialect` and validates the resulting SQL against that engine, returning a
/// formatted report on failure.
fn check_case(env: &TestEnv, case: &TestCase<Opts>, dialect: &str) -> Result<(), String> {
    let input = case.args.first().expect("missing input code block").clone();
    let options = build_options(case.options.identifier_resolution, dialect);
    let compile_error = |e: String| {
        report(
            case,
            &[
                ("Dialect", dialect.to_owned()),
                ("Schema", case.options.schema_name.clone()),
                ("Input", input.clone()),
                ("Compile error", e),
            ],
        )
    };
    let compiler = Compiler::new(&case.options.schema_json, options).map_err(compile_error)?;
    let sql = compiler.compile(input.clone()).map_err(compile_error)?.sql;

    let result = match dialect {
        "postgres" => env.explain_postgres(&sql),
        "duckdb" => env.explain_duckdb(&sql),
        other => panic!("Unknown dialect: {other}"),
    };

    result.map_err(|db_error| {
        report(
            case,
            &[
                ("Dialect", dialect.to_owned()),
                ("Schema", case.options.schema_name.clone()),
                ("Input", input.clone()),
                ("SQL", sql.clone()),
                ("DB error", db_error),
            ],
        )
    })
}

/// Owns a temporary Postgres server and an in-memory DuckDB database for the duration of the test.
/// The server is stopped and all temp files removed on drop, so cleanup happens even if a panic
/// unwinds the test; the DuckDB connection is closed when it is dropped along with the struct.
struct TestEnv {
    root: PathBuf,
    pg_data: PathBuf,
    pg_sock: PathBuf,
    duckdb: duckdb::Connection,
}

impl TestEnv {
    fn setup() -> Self {
        let root = std::env::temp_dir().join(format!("querydown_db_test_{}", std::process::id()));
        // Start from a clean slate in case a previous run left files behind.
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create temp root dir");
        let pg_sock = root.join("sock");
        fs::create_dir_all(&pg_sock).expect("create socket dir");

        let env = TestEnv {
            pg_data: root.join("pgdata"),
            duckdb: duckdb::Connection::open_in_memory().expect("open in-memory DuckDB"),
            pg_sock,
            root,
        };
        env.setup_postgres();
        env.setup_duckdb();
        env
    }

    // --- Postgres ---------------------------------------------------------------------------

    fn setup_postgres(&self) {
        run_checked(
            // Pin the locale to `C` (both via the cluster `--locale` and the ambient environment)
            // so the test doesn't depend on whatever `LANG`/`LC_*` the shell forwards in. A
            // forwarded host locale like `en_US.UTF-8` isn't installed in the dev container and
            // would otherwise make `initdb` bail with "invalid locale settings".
            Command::new("initdb")
                .env("LC_ALL", "C")
                .env("LANG", "C")
                .arg("-D")
                .arg(&self.pg_data)
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
        // Listen on a unix socket in our temp dir only (`-h ''` disables TCP), avoiding any clash
        // with a Postgres already running on the host.
        run_checked(
            Command::new("pg_ctl")
                .arg("-D")
                .arg(&self.pg_data)
                .arg("-l")
                .arg(self.root.join("pg.log"))
                .arg("-o")
                .arg(format!("-k {} -h ''", self.pg_sock.display()))
                .args(["-w", "start"]),
            "pg_ctl start",
        );
        for ddl in SCHEMA_DDL_FILES {
            run_checked(
                self.psql_command()
                    .arg("-f")
                    .arg(get_test_resource_path(ddl)),
                &format!("load {ddl} into postgres"),
            );
        }
    }

    fn psql_command(&self) -> Command {
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
        ]);
        cmd
    }

    fn explain_postgres(&self, sql: &str) -> Result<(), String> {
        capture(self.psql_command().args(["-c", &format!("EXPLAIN {sql}")]))
    }

    // --- DuckDB -----------------------------------------------------------------------------

    fn setup_duckdb(&self) {
        for ddl in SCHEMA_DDL_FILES {
            let ddl_sql = get_test_resource(ddl);
            self.duckdb
                .execute_batch(&ddl_sql)
                .unwrap_or_else(|e| panic!("load {ddl} into duckdb failed: {e}"));
        }
    }

    fn explain_duckdb(&self, sql: &str) -> Result<(), String> {
        // `prepare` parses and binds the query (where an unresolved function from a missing
        // extension like ICU would fail); draining the `EXPLAIN` rows forces planning to run too.
        let mut stmt = self
            .duckdb
            .prepare(&format!("EXPLAIN {sql}"))
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
        while rows.next().map_err(|e| e.to_string())?.is_some() {}
        Ok(())
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        let _ = Command::new("pg_ctl")
            .arg("-D")
            .arg(&self.pg_data)
            .args(["-m", "immediate", "-w", "stop"])
            .output();
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Runs a setup command, panicking with its output if it fails.
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

/// Runs a query command and returns `Err` with the database's error message if it didn't succeed.
fn capture(cmd: &mut Command) -> Result<(), String> {
    let output = cmd
        .output()
        .map_err(|e| format!("failed to spawn process: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!("{}\n{}", stderr.trim(), stdout.trim())
            .trim()
            .to_owned());
    }
    Ok(())
}
