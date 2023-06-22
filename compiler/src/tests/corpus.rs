#[test]
fn test_corpus() {
    // I'd like to figure out how to lift these imports out of this test fn so that we can have
    // one small test which calls all these other functions. There's some special behavior for
    // imports within integration tests that I don't fully understand yet. That behavior was
    // preventing me from writing these imports at the top of the file like normal.
    use crate::options::{IdentifierResolution, Options};
    use crate::Compiler;
    use crate::Postgres;

    use super::super::utils::get_test_resource;

    use std::path::PathBuf;
    use testcase_markdown::*;
    use toml::{from_str, map::Map, Table, Value};

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Opts {
        schema_json: String,
        identifier_resolution: IdentifierResolution,
        dialect: String,
    }

    impl Default for Opts {
        fn default() -> Self {
            Opts {
                schema_json: get_test_resource("issue_schema.json"),
                identifier_resolution: IdentifierResolution::Flexible,
                dialect: "postgres".to_owned(),
            }
        }
    }

    fn get_schema_json(toml_values: &Map<String, Value>) -> Option<String> {
        let schema_name = toml_values.get("schema").map(|v| v.as_str())??;
        let schema_file_name = match schema_name {
            "issues" => "issue_schema.json",
            "library" => "library_schema.json",
            _ => return None,
        };
        Some(get_test_resource(schema_file_name))
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

    fn get_dialect<'a>(toml_values: &'a Map<String, Value>) -> Option<&'a str> {
        toml_values.get("dialect").map(|v| v.as_str())?
    }

    impl MergeSerialized for Opts {
        fn merge_serialized(&self, source: String) -> Result<Self, String> {
            let values = from_str::<Table>(&source).map_err(|e| e.to_string())?;
            Ok(Opts {
                schema_json: get_schema_json(&values)
                    .unwrap_or_else(|| self.schema_json.to_owned()),
                identifier_resolution: get_identifier_resolution(&values)
                    .unwrap_or(self.identifier_resolution),
                dialect: get_dialect(&values)
                    .map(|d| d.to_owned())
                    .unwrap_or_else(|| self.dialect.clone()),
            })
        }
    }

    /// Removes spaces so that it's easy to compare two SQL strings without worrying about
    /// whitespace
    fn clean(s: String) -> String {
        s.replace("\n", "").replace("\t", "").replace(" ", "")
    }

    fn get_output(case: &TestCase<Opts>, input: &str, expected: &str, actual: &str) -> String {
        [
            "",
            " ╭────────────╮",
            "─┤ Test path: ├──────────────────────────────",
            " ╰────────────╯",
            &case.headings.join("\n"),
            &case.name,
            format!("  (Line: {})", case.line_number).as_str(),
            " ╭────────╮",
            "─┤ Input: ├──────────────────────────────",
            " ╰────────╯",
            &input,
            " ╭─────────────────╮",
            "─┤ Expected value: ├─────────────────────────",
            " ╰─────────────────╯",
            expected,
            " ╭───────────────╮",
            "─┤ Actual value: ├───────────────────────────",
            " ╰───────────────╯",
            actual,
            "────────────────────────────────────────────",
        ]
        .join("\n")
    }

    fn test(mut case: TestCase<Opts>) {
        let expected = case.args.pop().unwrap();
        let input = case.args.pop().unwrap();
        let options = Options {
            identifier_resolution: case.options.identifier_resolution,
            dialect: match case.options.dialect.as_str() {
                "postgres" => Box::new(Postgres()),
                _ => panic!("Unknown dialect"),
            },
        };
        // println!("{:}", case.options.schema_json);
        let compiler = Compiler::new(&case.options.schema_json, options).unwrap();
        let actual = compiler.compile(input.to_owned()).unwrap();
        if clean(actual.clone()) == clean(expected.clone()) {
            return;
        }
        println!("{}", get_output(&case, &input, &expected, &actual));
        panic!("Test corpus failure");
    }

    fn name_or_heading_contains(case: &TestCase<Opts>, s: &str) -> bool {
        case.name.contains(s) || case.headings.iter().any(|h| h.contains(s))
    }

    fn is_soloed(case: &TestCase<Opts>) -> bool {
        name_or_heading_contains(case, "🔦")
    }

    fn is_skipped(case: &TestCase<Opts>) -> bool {
        name_or_heading_contains(case, "⛔")
    }

    fn run() {
        let path = PathBuf::from_iter([env!("CARGO_MANIFEST_DIR"), "src", "tests", "corpus.md"]);
        let content = std::fs::read_to_string(path).unwrap();
        let cases = get_test_cases(content, Opts::default());
        let has_soloed_tests = cases.iter().any(is_soloed);
        for case in cases {
            if is_skipped(&case) {
                continue;
            }
            if has_soloed_tests && !is_soloed(&case) {
                continue;
            }
            test(case)
        }
    }

    run();
}
