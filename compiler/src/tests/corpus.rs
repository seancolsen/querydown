#[test]
fn test_corpus() {
    use testcase_markdown::TestCase;

    use super::corpus_loader::{
        build_options, clean, has_soloed, is_skipped, is_soloed, load_cases, report, Opts,
    };
    use crate::Compiler;

    fn test(case: TestCase<Opts>) {
        // Args are the fenced code blocks in document order: the Querydown input first, then the
        // expected SQL, then optionally a JSON block with the expected annotations.
        let input = case.args.first().expect("missing input code block").clone();
        let expected_sql = case.args.get(1).expect("missing SQL code block").clone();
        let expected_json = case.args.get(2).cloned();
        let options = build_options(case.options.identifier_resolution, &case.options.dialect);
        let compiler = Compiler::new(&case.options.schema_json, options).unwrap();
        let result = compiler.compile(input.clone()).unwrap();

        if clean(&result.sql) != clean(&expected_sql) {
            println!(
                "{}",
                report(
                    &case,
                    &[
                        ("Input", input.clone()),
                        ("Expected value", expected_sql.clone()),
                        ("Actual value", result.sql.clone()),
                    ],
                )
            );
            panic!("Test corpus failure (SQL mismatch)");
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
                println!(
                    "{}",
                    report(
                        &case,
                        &[
                            ("Input", input.clone()),
                            (
                                "Expected value",
                                serde_json::to_string_pretty(&expected).unwrap()
                            ),
                            (
                                "Actual value",
                                serde_json::to_string_pretty(&actual).unwrap()
                            ),
                        ],
                    )
                );
                panic!("Test corpus failure ({key} mismatch)");
            }
        }
    }

    let cases = load_cases();
    let has_soloed_tests = has_soloed(&cases);
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
