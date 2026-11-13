#[test]
fn test_corpus() {
    use std::panic::{catch_unwind, AssertUnwindSafe};

    use testcase_markdown::TestCase;

    use super::corpus_loader::{
        build_case_options, clean, has_soloed, is_skipped, is_soloed, load_cases, report, Opts,
    };
    use crate::Compiler;

    /// Checks one case, returning a formatted report if it fails.
    fn test(case: &TestCase<Opts>) -> Result<(), String> {
        // Args are the fenced code blocks in document order: the Querydown input first, then the
        // expected SQL, then optionally a JSON block with the expected annotations.
        let input = case.args.first().expect("missing input code block").clone();
        let expected_sql = case.args.get(1).expect("missing SQL code block").clone();
        let expected_json = case.args.get(2).cloned();
        let options = build_case_options(&case.options, &case.options.dialect);
        let compiler = Compiler::new(&case.options.schema_json, options).unwrap();
        // A compiler panic is a failure of this case, not of the whole corpus.
        let outcome = catch_unwind(AssertUnwindSafe(|| compiler.compile(input.clone())));
        let result = match outcome {
            Ok(Ok(result)) => result,
            Ok(Err(error)) => {
                return Err(report(
                    case,
                    &[
                        ("Input", input.clone()),
                        ("Expected value", expected_sql.clone()),
                        ("Compile error", error.render(&input)),
                    ],
                ));
            }
            Err(_) => {
                return Err(report(
                    case,
                    &[
                        ("Input", input.clone()),
                        ("Expected value", expected_sql.clone()),
                        ("Compiler panic", "See the panic message above.".to_owned()),
                    ],
                ));
            }
        };

        if clean(&result.sql) != clean(&expected_sql) {
            return Err(report(
                case,
                &[
                    ("Input", input.clone()),
                    ("Expected value", expected_sql.clone()),
                    ("Actual value", result.sql.clone()),
                ],
            ));
        }

        // Compare the annotations, parsed into `serde_json::Value` so the comparison is insensitive
        // to whitespace. The column annotations are compared only if the case's JSON block lists
        // them, whereas the query annotation is always compared: a case without one expects `null`.
        let expected: serde_json::Value = expected_json.map_or_else(
            || serde_json::json!({}),
            |json| serde_json::from_str(&json).expect("expected JSON block is not valid JSON"),
        );
        let mut comparisons = vec![(
            "queryAnnotation",
            serde_json::to_value(&result.query_annotation).unwrap(),
        )];
        if expected.get("columnAnnotations").is_some() {
            comparisons.push((
                "columnAnnotations",
                serde_json::to_value(&result.column_annotations).unwrap(),
            ));
        }
        for (key, actual) in comparisons {
            let expected = expected.get(key).cloned().unwrap_or_default();
            if actual != expected {
                return Err(report(
                    case,
                    &[
                        ("Input", input.clone()),
                        (
                            &format!("Expected {key}"),
                            serde_json::to_string_pretty(&expected).unwrap(),
                        ),
                        (
                            &format!("Actual {key}"),
                            serde_json::to_string_pretty(&actual).unwrap(),
                        ),
                    ],
                ));
            }
        }
        Ok(())
    }

    // Every case runs, even after one fails, so that a single run reports all the failures.
    let cases = load_cases();
    let has_soloed_tests = has_soloed(&cases);
    let mut failures = 0usize;
    for case in &cases {
        if is_skipped(case) {
            continue;
        }
        if has_soloed_tests && !is_soloed(case) {
            continue;
        }
        if let Err(message) = test(case) {
            println!("{message}");
            failures += 1;
        }
    }
    if failures > 0 {
        panic!("{failures} corpus failure(s). See output above.");
    }
}
