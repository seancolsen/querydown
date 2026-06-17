#[test]
fn test_corpus() {
    use testcase_markdown::TestCase;

    use super::corpus_loader::{
        build_options, clean, has_soloed, is_skipped, is_soloed, load_cases, report, Opts,
    };
    use crate::Compiler;

    fn test(case: TestCase<Opts>) {
        // Args are the fenced code blocks in document order: the Querydown input first, then the
        // expected SQL, then optionally a JSON block with the expected column metadata.
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

        if let Some(expected_json) = expected_json {
            // Compare only the `columnMetadata` portion, parsed into `serde_json::Value` so the
            // comparison is insensitive to whitespace.
            let actual_meta = serde_json::to_value(&result.column_metadata).unwrap();
            let expected_value: serde_json::Value = serde_json::from_str(&expected_json)
                .expect("expected JSON block is not valid JSON");
            let expected_meta = expected_value
                .get("columnMetadata")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            if actual_meta != expected_meta {
                let actual_str = serde_json::to_string_pretty(&actual_meta).unwrap();
                let expected_str = serde_json::to_string_pretty(&expected_meta).unwrap();
                println!(
                    "{}",
                    report(
                        &case,
                        &[
                            ("Input", input.clone()),
                            ("Expected value", expected_str),
                            ("Actual value", actual_str),
                        ],
                    )
                );
                panic!("Test corpus failure (metadata mismatch)");
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
