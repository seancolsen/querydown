#[test]
fn test_corpus() {
    // I'd like to figure out how to lift these imports out of this test fn so that we can have
    // one small test which calls all these other functions. There's some special behavior for
    // imports within integration tests that I don't fully understand yet. That behavior was
    // preventing me from writing these imports at the top of the file like normal.
    use crate::compiler::Compiler;
    use crate::dialects::postgres::Postgres;
    use crate::tests::test_utils::get_test_resource;
    use std::path::PathBuf;
    use testcase_markdown::*;

    #[derive(Default, Clone)]
    struct Options();

    impl MergeSerialized for Options {
        fn merge_serialized(&self, source: String) -> Result<Self, String> {
            Ok(Options())
        }
    }

    /// Removes spaces so that it's easy to compare two SQL strings without worrying about
    /// whitespace
    fn clean(s: String) -> String {
        s.replace("\n", "").replace("\t", "").replace(" ", "")
    }

    fn get_output(case: &TestCase<Options>, input: &str, expected: &str, actual: &str) -> String {
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

    fn test(mut case: TestCase<Options>) {
        let expected = case.args.pop().unwrap();
        let input = case.args.pop().unwrap();
        let schema_json = get_test_resource("issue_schema.json");
        let compiler = Compiler::new(&schema_json, Postgres()).unwrap();
        let actual = compiler.compile(input.to_owned()).unwrap();
        if clean(actual.clone()) == clean(expected.clone()) {
            return;
        }
        println!("{}", get_output(&case, &input, &expected, &actual));
        panic!("Test corpus failure");
    }

    fn name_or_heading_contains(case: &TestCase<Options>, s: &str) -> bool {
        case.name.contains(s) || case.headings.iter().any(|h| h.contains(s))
    }

    fn is_soloed(case: &TestCase<Options>) -> bool {
        name_or_heading_contains(case, "🔦")
    }

    fn is_skipped(case: &TestCase<Options>) -> bool {
        name_or_heading_contains(case, "⛔")
    }

    fn run() {
        let path = PathBuf::from_iter([env!("CARGO_MANIFEST_DIR"), "src", "tests", "corpus.md"]);
        let content = std::fs::read_to_string(path).unwrap();
        let cases = get_test_cases(content, Options::default());
        // let has_soloed_tests = cases.iter().any(|c| c.name.contains('✅'));
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
