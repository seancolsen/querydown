//! Tests the Querydown examples in the documentation, so that the docs can't show code that
//! doesn't work.
//!
//! The documentation is Markdown: the pages of the docs site (`docs/` and `site/content/`) and the
//! repository's `README.md`. Every fenced code block in those files whose language is `qd` is an
//! example. So is every `.qd` file in those places and in the site's code (`site/src/`), such as
//! the query that the playground opens with. By default, an example must compile against the sample issue-tracker schema under
//! every dialect. Words after `qd` in the block's info string change what's expected of it:
//!
//! | Info string         | Meaning                                                              |
//! | ------------------- | -------------------------------------------------------------------- |
//! | `qd`                | The example must compile.                                            |
//! | `qd error`          | The example must *fail* to compile.                                  |
//! | `qd fragment`       | Not a whole query, so it isn't tested.                               |
//! | `qd limit=50`       | Compile with the `limit` option (likewise `offset=100`).             |
//!
//! A code block after an example can state what the example compiles to, which is then checked too
//! (ignoring whitespace):
//!
//! | Info string         | Must match the example's…                                            |
//! | ------------------- | -------------------------------------------------------------------- |
//! | `sql output`        | SQL for Postgres                                                     |
//! | `sql output duckdb` | SQL for DuckDB                                                       |
//! | `json output`       | annotations, in the same form as the corpus (see [`super::corpus`])  |
//!
//! The site's renderer (`site/src/lib/server/markdown.ts`) reads the same info strings, and the
//! conventions are documented for authors in `docs/README.md`. Keep all three in step.
//!
//! With the `db-tests` feature, [`super::db_corpus`] also runs each example's SQL against real
//! databases.

use std::fs;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};

use super::corpus_loader::{build_options, clean};
use super::get_test_resource;
use crate::options::{IdentifierResolution, Options};
use crate::{CompileResult, Compiler, Error};

/// The dialects that every example must compile under.
pub const DIALECTS: [&str; 2] = ["postgres", "duckdb"];

/// The files and directories (relative to the repository root) that hold examples, as Markdown
/// files or `.qd` files.
const SOURCES: [&str; 4] = ["README.md", "docs", "site/content", "site/src"];

/// What's expected of an example when it's compiled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expectation {
    Compiles,
    Fails,
}

/// A code block that states the output of the example before it.
#[derive(Debug)]
pub struct ExpectedOutput {
    pub content: String,
    pub line: usize,
}

/// A testable Querydown example from the documentation.
#[derive(Debug)]
pub struct DocExample {
    /// The Markdown file, relative to the repository root.
    pub file: String,
    /// The line of the example's opening fence.
    pub line: usize,
    pub source: String,
    pub expectation: Expectation,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    /// The expected SQL, for each dialect that the docs state it for.
    pub expected_sql: Vec<(&'static str, ExpectedOutput)>,
    pub expected_annotations: Option<ExpectedOutput>,
}

impl DocExample {
    pub fn location(&self) -> String {
        format!("{}:{}", self.file, self.line)
    }

    pub fn compile(&self, dialect: &str) -> Result<CompileResult, Error> {
        let options = Options {
            limit: self.limit,
            offset: self.offset,
            ..build_options(IdentifierResolution::Flexible, dialect)
        };
        let compiler = Compiler::new(&get_test_resource("issue_schema.json"), options)?;
        compiler.compile(self.source.clone())
    }
}

// -------------------------------------------------------------------------------------------------
// Reading the Markdown
// -------------------------------------------------------------------------------------------------

/// A fenced code block.
struct Fence {
    info: String,
    content: String,
    line: usize,
}

/// Finds the fenced code blocks in a Markdown document.
///
/// This understands as much of Markdown as it needs to: a fence is a line of three or more
/// backticks or tildes (after any indentation, so that a block may sit within a list), and it's
/// closed by a line of at least as many of the same character. So a longer fence may hold shorter
/// ones, which is how a document shows Markdown source without its examples being tested.
fn fences(markdown: &str) -> Vec<Fence> {
    struct Open {
        marker: char,
        length: usize,
        indent: usize,
        info: String,
        line: usize,
        content: Vec<String>,
    }

    let mut result = Vec::new();
    let mut open: Option<Open> = None;
    for (index, line) in markdown.lines().enumerate() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        let marker = trimmed.chars().next().filter(|c| *c == '`' || *c == '~');
        let length = marker.map_or(0, |m| trimmed.chars().take_while(|c| *c == m).count());
        let rest = trimmed[length..].trim();

        if let Some(block) = &mut open {
            if marker == Some(block.marker) && length >= block.length && rest.is_empty() {
                let block = open.take().unwrap();
                result.push(Fence {
                    info: block.info,
                    content: block.content.join("\n"),
                    line: block.line,
                });
            } else {
                // Drop the indentation that the block as a whole has.
                let strip = line
                    .chars()
                    .take(block.indent)
                    .take_while(|c| *c == ' ')
                    .count();
                block.content.push(line[strip..].to_owned());
            }
        } else if let Some(marker) = marker {
            // The info string of a backtick fence can't contain a backtick. (Such a line is inline
            // code that happens to start the line.)
            if length >= 3 && !(marker == '`' && rest.contains('`')) {
                open = Some(Open {
                    marker,
                    length,
                    indent,
                    info: rest.to_owned(),
                    line: index + 1,
                    content: Vec::new(),
                });
            }
        }
    }
    result
}

/// Extracts the examples from one Markdown document. Panics on an info string it doesn't
/// understand, so that a typo can't silently stop an example from being tested.
fn examples_in(file: &str, markdown: &str) -> Vec<DocExample> {
    let mut examples: Vec<DocExample> = Vec::new();
    // Whether the example that an `output` block would refer to is one that produces output.
    let mut can_take_output = false;

    for fence in fences(markdown) {
        let location = format!("{file}:{}", fence.line);
        let mut words = fence.info.split_whitespace();
        let language = words.next().unwrap_or("");
        let flags: Vec<&str> = words.collect();
        let output = || ExpectedOutput {
            content: fence.content.clone(),
            line: fence.line,
        };

        match language {
            "qd" => {
                let mut example = DocExample {
                    file: file.to_owned(),
                    line: fence.line,
                    source: fence.content.clone(),
                    expectation: Expectation::Compiles,
                    limit: None,
                    offset: None,
                    expected_sql: Vec::new(),
                    expected_annotations: None,
                };
                let mut is_fragment = false;
                for flag in flags {
                    let count = |value: &str| {
                        value.parse::<u64>().unwrap_or_else(|_| {
                            panic!("{location}: `{flag}` needs a whole number.")
                        })
                    };
                    match flag.split_once('=') {
                        None if flag == "error" => example.expectation = Expectation::Fails,
                        None if flag == "fragment" => is_fragment = true,
                        Some(("limit", value)) => example.limit = Some(count(value)),
                        Some(("offset", value)) => example.offset = Some(count(value)),
                        _ => panic!("{location}: unknown flag `{flag}` on a `qd` code block."),
                    }
                }
                can_take_output = !is_fragment && example.expectation == Expectation::Compiles;
                if !is_fragment {
                    examples.push(example);
                }
            }
            "sql" | "json" if flags.first() == Some(&"output") => {
                if !can_take_output {
                    panic!("{location}: an `output` block must follow an example that compiles.");
                }
                let example = examples.last_mut().unwrap();
                match (language, &flags[1..]) {
                    ("sql", []) => example.expected_sql.push(("postgres", output())),
                    ("sql", ["postgres"]) => example.expected_sql.push(("postgres", output())),
                    ("sql", ["duckdb"]) => example.expected_sql.push(("duckdb", output())),
                    ("json", []) => example.expected_annotations = Some(output()),
                    _ => panic!("{location}: unknown flags on an `output` block: {flags:?}."),
                }
            }
            _ => {}
        }
    }
    examples
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the compiler crate sits within the repository")
        .to_owned()
}

/// Finds the files within `path` that hold examples: Markdown files and `.qd` files.
fn example_files(path: &Path, files: &mut Vec<PathBuf>) {
    if path.is_dir() {
        let mut entries: Vec<PathBuf> = fs::read_dir(path)
            .unwrap_or_else(|e| panic!("could not read {}: {e}", path.display()))
            .map(|entry| entry.unwrap().path())
            .collect();
        entries.sort();
        for entry in entries {
            example_files(&entry, files);
        }
    } else if path
        .extension()
        .is_some_and(|extension| extension == "md" || extension == "qd")
    {
        files.push(path.to_owned());
    }
}

/// Reads every example from the documentation.
pub fn load_examples() -> Vec<DocExample> {
    let root = repo_root();
    let mut files = Vec::new();
    for source in SOURCES {
        let path = root.join(source);
        assert!(path.exists(), "{} does not exist", path.display());
        example_files(&path, &mut files);
    }
    let examples: Vec<DocExample> = files
        .iter()
        .flat_map(|path| {
            let file = path.strip_prefix(&root).unwrap().display().to_string();
            let content = fs::read_to_string(path).unwrap();
            if path.extension().is_some_and(|extension| extension == "qd") {
                // A `.qd` file is one whole query, which must compile.
                vec![DocExample {
                    file,
                    line: 1,
                    source: content,
                    expectation: Expectation::Compiles,
                    limit: None,
                    offset: None,
                    expected_sql: Vec::new(),
                    expected_annotations: None,
                }]
            } else {
                examples_in(&file, &content)
            }
        })
        .collect();
    // Guards against the docs moving without this loader following them.
    assert!(
        examples.len() > 100,
        "expected to find the documentation's examples, but found only {}",
        examples.len()
    );
    examples
}

// -------------------------------------------------------------------------------------------------
// The test
// -------------------------------------------------------------------------------------------------

fn failure(example: &DocExample, dialect: &str, problem: &str, detail: &str) -> String {
    format!(
        "\n── {} ({dialect}) ──\n{problem}\n\n{}\n\n{detail}\n",
        example.location(),
        example.source
    )
}

/// Checks one example under one dialect, returning a report if it isn't as the docs say.
fn check(example: &DocExample, dialect: &str) -> Result<(), String> {
    // A compiler panic is a failure of this example, not of the whole test.
    let outcome = catch_unwind(AssertUnwindSafe(|| example.compile(dialect)))
        .map_err(|_| failure(example, dialect, "The compiler panicked.", ""))?;

    let result = match (outcome, example.expectation) {
        (Ok(result), Expectation::Compiles) => result,
        (Err(_), Expectation::Fails) => return Ok(()),
        (Err(error), Expectation::Compiles) => {
            return Err(failure(
                example,
                dialect,
                "This example doesn't compile.",
                &error.render(&example.source),
            ));
        }
        (Ok(result), Expectation::Fails) => {
            return Err(failure(
                example,
                dialect,
                "This example is marked `error`, but it compiles.",
                &result.sql,
            ));
        }
    };

    for (expected_dialect, expected) in &example.expected_sql {
        if *expected_dialect == dialect && clean(&expected.content) != clean(&result.sql) {
            return Err(failure(
                example,
                dialect,
                &format!(
                    "The SQL stated at line {} isn't what this example compiles to.",
                    expected.line
                ),
                &format!("Actual SQL:\n{}", result.sql),
            ));
        }
    }

    if let Some(expected) = &example.expected_annotations {
        let stated: serde_json::Value = serde_json::from_str(&expected.content)
            .unwrap_or_else(|e| panic!("{}:{}: invalid JSON: {e}", example.file, expected.line));
        let actual = serde_json::to_value(&result).unwrap();
        // Only the keys that the docs state are compared, so a block may show just the part of the
        // output that it's about.
        for (key, stated_value) in stated.as_object().expect("a JSON object") {
            if actual.get(key) != Some(stated_value) {
                return Err(failure(
                    example,
                    dialect,
                    &format!(
                        "The `{key}` stated at line {} isn't what this example produces.",
                        expected.line
                    ),
                    &format!(
                        "Actual output:\n{}",
                        serde_json::to_string_pretty(&actual[key]).unwrap()
                    ),
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn test_docs_examples() {
    // Every example runs, even after one fails, so that a single run reports all the failures.
    let mut failures = 0usize;
    for example in load_examples() {
        for dialect in DIALECTS {
            if let Err(report) = check(&example, dialect) {
                println!("{report}");
                failures += 1;
            }
        }
    }
    if failures > 0 {
        panic!("{failures} documentation example failure(s). See output above.");
    }
}

#[test]
fn fences_are_found() {
    let markdown = [
        "Intro with `inline` code.",
        "",
        "```qd error",
        "#issues $nope",
        "```",
        "",
        "- A list item:",
        "",
        "    ```qd",
        "    #issues",
        "      $id",
        "    ```",
        "",
        "````md",
        "```qd",
        "not an example",
        "```",
        "````",
        "",
        "```` ```qd ```` is inline code.",
    ]
    .join("\n");
    let found: Vec<(String, String, usize)> = fences(&markdown)
        .into_iter()
        .map(|fence| (fence.info, fence.content, fence.line))
        .collect();
    assert_eq!(
        found,
        vec![
            ("qd error".to_owned(), "#issues $nope".to_owned(), 3),
            ("qd".to_owned(), "#issues\n  $id".to_owned(), 9),
            ("md".to_owned(), "```qd\nnot an example\n```".to_owned(), 14),
        ]
    );
}

#[test]
fn examples_are_read_from_fences() {
    let markdown = [
        "```qd limit=5 offset=10",
        "#issues",
        "```",
        "```sql output",
        "SELECT 1;",
        "```",
        "```sql output duckdb",
        "SELECT 2;",
        "```",
        "```json output",
        "{}",
        "```",
        "```qd fragment",
        "(@x => @x)",
        "```",
        "```qd error",
        "#nope",
        "```",
        "```sql",
        "SELECT 'not checked';",
        "```",
    ]
    .join("\n");
    let examples = examples_in("test.md", &markdown);
    assert_eq!(examples.len(), 2);
    assert_eq!(examples[0].source, "#issues");
    assert_eq!((examples[0].limit, examples[0].offset), (Some(5), Some(10)));
    assert_eq!(examples[0].expectation, Expectation::Compiles);
    let stated: Vec<(&str, &str)> = examples[0]
        .expected_sql
        .iter()
        .map(|(dialect, output)| (*dialect, output.content.as_str()))
        .collect();
    assert_eq!(
        stated,
        vec![("postgres", "SELECT 1;"), ("duckdb", "SELECT 2;")]
    );
    assert!(examples[0].expected_annotations.is_some());
    assert_eq!(examples[1].expectation, Expectation::Fails);
}

#[test]
fn the_playground_query_is_tested() {
    // Guards against the playground's starting query moving without this loader following it.
    let file = "site/src/routes/playground/starting-query.qd";
    assert!(
        load_examples().iter().any(|example| example.file == file),
        "expected {file} to be among the examples"
    );
}
