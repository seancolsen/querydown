//! Shared machinery for loading and parsing the corpus test cases in `corpus.md`.
//!
//! Both the string-match test ([`super::corpus`]) and the database test
//! ([`super::db_corpus`], compiled under the `db-tests` feature) parse the corpus identically
//! through this module.

use std::path::PathBuf;

use testcase_markdown::{get_test_cases, MergeSerialized, TestCase};
use toml::{from_str, map::Map, Table, Value};

use super::get_test_resource;
use crate::options::{IdentifierResolution, Options};
use crate::{DuckDB, Postgres};

/// Per-case configuration, parsed from the optional ```` ```toml options ```` block and inherited
/// down the heading hierarchy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opts {
    pub schema_json: String,
    /// The name of the schema (`"issues"` or `"library"`), tracked alongside the JSON so the
    /// database test can report which schema a failing case used.
    #[cfg_attr(not(feature = "db-tests"), allow(dead_code))]
    pub schema_name: String,
    pub identifier_resolution: IdentifierResolution,
    pub dialect: String,
    /// Controls whether this case is executed against real databases. See [`DbSkip`].
    #[cfg_attr(not(feature = "db-tests"), allow(dead_code))]
    pub db_skip: DbSkip,
}

/// How a corpus case opts out of database execution. Most cases run on every engine, but some
/// produce SQL that intentionally won't plan against the minimal test schema (or on one engine),
/// and can exclude themselves while still asserting their SQL string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DbSkip {
    /// Run on every supported engine (the default).
    None,
    /// Skip database execution entirely (`db_skip = true`).
    All,
    /// Skip only the named dialects (`db_skip = ["duckdb"]`).
    Dialects(Vec<String>),
}

impl DbSkip {
    #[cfg_attr(not(feature = "db-tests"), allow(dead_code))]
    pub fn skips(&self, dialect: &str) -> bool {
        match self {
            DbSkip::None => false,
            DbSkip::All => true,
            DbSkip::Dialects(dialects) => dialects.iter().any(|d| d == dialect),
        }
    }
}

impl Default for Opts {
    fn default() -> Self {
        Opts {
            schema_json: get_test_resource("issue_schema.json"),
            schema_name: "issues".to_owned(),
            identifier_resolution: IdentifierResolution::Flexible,
            dialect: "postgres".to_owned(),
            db_skip: DbSkip::None,
        }
    }
}

fn get_schema(toml_values: &Map<String, Value>) -> Option<(String, String)> {
    let schema_name = toml_values.get("schema").map(|v| v.as_str())??;
    let schema_file_name = match schema_name {
        "issues" => "issue_schema.json",
        "library" => "library_schema.json",
        _ => return None,
    };
    Some((schema_name.to_owned(), get_test_resource(schema_file_name)))
}

fn get_identifier_resolution(toml_values: &Map<String, Value>) -> Option<IdentifierResolution> {
    let identifier_resolution = toml_values
        .get("identifier_resolution")
        .map(|v| v.as_str())??;
    match identifier_resolution {
        "strict" => Some(IdentifierResolution::Strict),
        "flexible" => Some(IdentifierResolution::Flexible),
        _ => None,
    }
}

fn get_dialect(toml_values: &Map<String, Value>) -> Option<&str> {
    toml_values.get("dialect").map(|v| v.as_str())?
}

fn get_db_skip(toml_values: &Map<String, Value>) -> Option<DbSkip> {
    match toml_values.get("db_skip")? {
        Value::Boolean(true) => Some(DbSkip::All),
        Value::Boolean(false) => Some(DbSkip::None),
        Value::Array(items) => Some(DbSkip::Dialects(
            items
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_owned()))
                .collect(),
        )),
        _ => None,
    }
}

impl MergeSerialized for Opts {
    fn merge_serialized(&self, source: String) -> Result<Self, String> {
        let values = from_str::<Table>(&source).map_err(|e| e.to_string())?;
        let (schema_name, schema_json) = get_schema(&values)
            .unwrap_or_else(|| (self.schema_name.clone(), self.schema_json.clone()));
        Ok(Opts {
            schema_json,
            schema_name,
            identifier_resolution: get_identifier_resolution(&values)
                .unwrap_or(self.identifier_resolution),
            dialect: get_dialect(&values)
                .map(|d| d.to_owned())
                .unwrap_or_else(|| self.dialect.clone()),
            db_skip: get_db_skip(&values).unwrap_or_else(|| self.db_skip.clone()),
        })
    }
}

/// Reads and parses every case in `corpus.md`.
pub fn load_cases() -> Vec<TestCase<Opts>> {
    let path = PathBuf::from_iter([env!("CARGO_MANIFEST_DIR"), "src", "tests", "corpus.md"]);
    let content = std::fs::read_to_string(path).unwrap();
    get_test_cases(content, Opts::default())
}

/// Builds compiler [`Options`] for a given dialect, independent of the dialect a case declares.
pub fn build_options(identifier_resolution: IdentifierResolution, dialect: &str) -> Options {
    Options {
        identifier_resolution,
        dialect: match dialect {
            "postgres" => Box::new(Postgres()),
            "duckdb" => Box::new(DuckDB()),
            other => panic!("Unknown dialect: {other}"),
        },
    }
}

/// Removes whitespace so two SQL strings can be compared without worrying about formatting.
pub fn clean(s: &str) -> String {
    s.replace(['\n', '\t', ' '], "")
}

fn name_or_heading_contains(case: &TestCase<Opts>, s: &str) -> bool {
    case.name.contains(s) || case.headings.iter().any(|h| h.contains(s))
}

pub fn is_soloed(case: &TestCase<Opts>) -> bool {
    name_or_heading_contains(case, "🔦")
}

pub fn is_skipped(case: &TestCase<Opts>) -> bool {
    name_or_heading_contains(case, "⛔")
}

pub fn has_soloed(cases: &[TestCase<Opts>]) -> bool {
    cases.iter().any(is_soloed)
}

/// Formats a failure report with the case's location followed by labeled sections. Each label is
/// drawn in a box to make failures easy to scan when developing the corpus by hand.
pub fn report(case: &TestCase<Opts>, sections: &[(&str, String)]) -> String {
    let mut lines: Vec<String> = vec![
        String::new(),
        boxed_label("Test path"),
        case.headings.join("\n"),
        case.name.clone(),
        format!("  (Line: {})", case.line_number),
    ];
    for (label, content) in sections {
        lines.push(boxed_label(label));
        lines.push(content.clone());
    }
    lines.push("─".repeat(44));
    lines.join("\n")
}

/// Draws a label inside a box, e.g.
///
/// ```text
///  ╭───────────╮
/// ─┤ Test path ├──────────────────────────────
///  ╰───────────╯
/// ```
fn boxed_label(label: &str) -> String {
    let inner = "─".repeat(label.chars().count() + 2);
    let middle = format!("─┤ {label} ├");
    let trailing = 44usize.saturating_sub(middle.chars().count());
    format!(" ╭{inner}╮\n{middle}{}\n ╰{inner}╯", "─".repeat(trailing),)
}
