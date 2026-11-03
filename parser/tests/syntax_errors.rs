//! Tests for the quality of syntax errors: their messages, locations, and hints.
//!
//! Each case asserts the full output of `Error::render`, which covers the message, the line and
//! column, the underlined span, any secondary labels, and any hint. When a change to the grammar or
//! to the error translation alters one of these, review the new output for clarity before updating
//! the expectation.

use querydown_parser::{
    parse, parse_conditions, parse_display, parse_sorting, Error, ErrorKind, Position,
};

#[track_caller]
fn assert_render<T: std::fmt::Debug>(
    parser: fn(&str) -> Result<T, Error>,
    input: &str,
    expected: &[&str],
) {
    let err = parser(input).expect_err("input should fail to parse");
    assert_eq!(err.kind, ErrorKind::Syntax);
    let actual = err.render(input);
    let expected = expected.join("\n");
    assert!(
        actual == expected,
        "rendered error mismatch for input {input:?}\n--- expected:\n{expected}\n--- actual:\n{actual}\n"
    );
}

#[test]
fn unexpected_closing_bracket() {
    assert_render(
        parse,
        "#issues foo:)",
        &[
            "syntax error: Unexpected `)`, which has no matching `(`. Expected a value.",
            " --> line 1, column 13",
            "  |",
            "1 | #issues foo:)",
            "  |             ^",
            "  |",
            "  = hint: Remove this `)`, or add a `(` where the group should begin.",
        ],
    );
}

#[test]
fn missing_value_after_comparison_operator() {
    assert_render(
        parse,
        "#issues title:open status:",
        &[
            "syntax error: Unexpected end of input. Expected a value.",
            " --> line 1, column 27",
            "  |",
            "1 | #issues title:open status:",
            "  |                           ^",
        ],
    );
}

#[test]
fn missing_result_column_after_dollar() {
    assert_render(
        parse,
        "#issues $",
        &[
            "syntax error: Unexpected end of input. Expected a column, an expression, or `*`.",
            " --> line 1, column 10",
            "  |",
            "1 | #issues $",
            "  |          ^",
        ],
    );
}

#[test]
fn missing_alias() {
    assert_render(
        parse,
        "#issues $title->",
        &[
            "syntax error: Unexpected end of input. Expected an alias.",
            " --> line 1, column 17",
            "  |",
            "1 | #issues $title->",
            "  |                 ^",
        ],
    );
}

#[test]
fn missing_function_name_after_pipe() {
    assert_render(
        parse,
        "#issues $title|",
        &[
            "syntax error: Unexpected end of input. Expected a function name.",
            " --> line 1, column 16",
            "  |",
            "1 | #issues $title|",
            "  |                ^",
        ],
    );
}

#[test]
fn missing_related_table_after_has_quantity() {
    assert_render(
        parse,
        "#issues ++",
        &[
            "syntax error: Unexpected end of input. Expected a related table (e.g. `#comments`).",
            " --> line 1, column 11",
            "  |",
            "1 | #issues ++",
            "  |           ^",
        ],
    );
}

#[test]
fn invalid_column_control_flag() {
    assert_render(
        parse,
        "#issues $title \\x",
        &[
            "syntax error: Unexpected `x`. Expected a column control flag (`s`, `d`, `n`, `g`, `h`, `p`, or a number).",
            " --> line 1, column 17",
            "  |",
            "1 | #issues $title \\x",
            "  |                 ^",
        ],
    );
}

#[test]
fn invalid_sort_flag() {
    assert_render(
        parse,
        "#issues \\\\title \\x",
        &[
            "syntax error: Unexpected `x`. Expected a sort flag (`d` or `n`).",
            " --> line 1, column 18",
            "  |",
            "1 | #issues \\\\title \\x",
            "  |                  ^",
        ],
    );
}

#[test]
fn missing_base_table_sigil() {
    assert_render(
        parse,
        "issues $title",
        &[
            "syntax error: Unexpected `issues`. Expected a definition or a base table.",
            " --> line 1, column 1",
            "  |",
            "1 | issues $title",
            "  | ^^^^^^",
            "  |",
            "  = hint: Table names are written with a `#` prefix. Did you mean `#issues`?",
        ],
    );
}

#[test]
fn empty_query() {
    assert_render(
        parse,
        "",
        &[
            "syntax error: Unexpected end of input. Expected a definition or a base table.",
            " --> line 1, column 1",
            "  |",
            "1 | ",
            "  | ^",
            "  |",
            "  = hint: A query must begin with its base table, written with a `#` prefix, e.g. `#issues`.",
        ],
    );
}

#[test]
fn unterminated_string() {
    assert_render(
        parse,
        "#issues title:\"abc",
        &[
            "syntax error: Unterminated string. Expected a closing `\"`.",
            " --> line 1, column 15",
            "  |",
            "1 | #issues title:\"abc",
            "  |               ^",
            "  |",
            "  = hint: Add a `\"` where the string should end. To include a literal `\"` within the string, escape it as `\\\"`.",
        ],
    );
}

#[test]
fn unterminated_quoted_identifier() {
    assert_render(
        parse,
        "#issues $title->`Title",
        &[
            "syntax error: Unterminated quoted identifier. Expected a closing `` ` ``.",
            " --> line 1, column 17",
            "  |",
            "1 | #issues $title->`Title",
            "  |                 ^",
            "  |",
            "  = hint: Add a `` ` `` where the quoted identifier should end. To include a literal `` ` `` within the quoted identifier, escape it as `` \\` ``.",
        ],
    );
}

#[test]
fn unterminated_block_comment() {
    assert_render(
        parse,
        "#issues /* note",
        &[
            "syntax error: Unterminated block comment. Expected a closing `*/`.",
            " --> line 1, column 9",
            "  |",
            "1 | #issues /* note",
            "  |         ^^",
        ],
    );
}

#[test]
fn unclosed_bracket() {
    assert_render(
        parse,
        "#issues labels:[bug regression",
        &[
            "syntax error: Unclosed `[`. Expected a matching `]` before the end of input.",
            " --> line 1, column 16",
            "  |",
            "1 | #issues labels:[bug regression",
            "  |                ^",
            "  |                               - expected `]` here or earlier",
        ],
    );
}

#[test]
fn unclosed_bracket_before_another_error() {
    assert_render(
        parse,
        "#issues foo:(1 $x",
        &[
            "syntax error: Unexpected `$`. Expected a comparison operator or `)`.",
            " --> line 1, column 16",
            "  |",
            "1 | #issues foo:(1 $x",
            "  |                ^",
            "  |             - this `(` is never closed",
        ],
    );
}

#[test]
fn mismatched_brackets() {
    assert_render(
        parse,
        "#issues\n  status:open\n  ++#labels{name:[bug regression}\n$title",
        &[
            "syntax error: Mismatched `}`. Expected `]` to close the `[` opened earlier.",
            " --> line 3, column 33",
            "  |",
            "3 |   ++#labels{name:[bug regression}",
            "  |                                 ^",
            "  |                  - `[` opened here",
        ],
    );
}

#[test]
fn error_on_a_later_line() {
    assert_render(
        parse,
        "#issues\n  status:open\n  title:)\n$title",
        &[
            "syntax error: Unexpected `)`, which has no matching `(`. Expected a value.",
            " --> line 3, column 9",
            "  |",
            "3 |   title:)",
            "  |         ^",
            "  |",
            "  = hint: Remove this `)`, or add a `(` where the group should begin.",
        ],
    );
}

#[test]
fn duplicate_duration_part() {
    assert_render(
        parse,
        "#issues created_at:<1y2y",
        &[
            "syntax error: A duration can't have more than one year part.",
            " --> line 1, column 21",
            "  |",
            "1 | #issues created_at:<1y2y",
            "  |                     ^^^^",
        ],
    );
}

#[test]
fn nested_entry_without_leading_column() {
    assert_render(
        parse,
        "#issues $author.( $8 )",
        &[
            "syntax error: An entry nested within `.( )` must begin with a column reference so that the path before `.( )` can be applied to it.",
            " --> line 1, column 10",
            "  |",
            "1 | #issues $author.( $8 )",
            "  |          ^^^^^^^^^^^^^",
        ],
    );
}

#[test]
fn invalid_escape_character() {
    assert_render(
        parse,
        "#issues title:'\\q'",
        &[
            "syntax error: Unexpected `q`. Expected an escape character (`\\`, `/`, the quote character, `b`, `f`, `n`, `r`, `t`, or `u`).",
            " --> line 1, column 17",
            "  |",
            "1 | #issues title:'\\q'",
            "  |                 ^",
        ],
    );
}

#[test]
fn invalid_unicode_escape() {
    assert_render(
        parse,
        "#issues title:'\\uD800'",
        &[
            "syntax error: `\\uD800` is not a valid unicode character.",
            " --> line 1, column 18",
            "  |",
            "1 | #issues title:'\\uD800'",
            "  |                  ^^^^",
        ],
    );
}

#[test]
fn missing_case_fallback() {
    assert_render(
        parse,
        "#issues $? status:open ~ 1",
        &[
            "syntax error: Unexpected end of input. Expected a comparison operator, a value, or `~~` followed by a fallback value.",
            " --> line 1, column 27",
            "  |",
            "1 | #issues $? status:open ~ 1",
            "  |                           ^",
        ],
    );
}

#[test]
fn anonymous_function_missing_arrow() {
    assert_render(
        parse,
        "#issues $id|(@x)",
        &[
            "syntax error: Unexpected `)`. Expected `=>`.",
            " --> line 1, column 16",
            "  |",
            "1 | #issues $id|(@x)",
            "  |                ^",
        ],
    );
}

#[test]
fn missing_function_parameter() {
    assert_render(
        parse,
        "@@f = => 1\n#issues",
        &[
            "syntax error: Unexpected `=`. Expected a function parameter (e.g. `@x`).",
            " --> line 1, column 7",
            "  |",
            "1 | @@f = => 1",
            "  |       ^",
        ],
    );
}

#[test]
fn display_column_in_conditions_section() {
    assert_render(
        parse_conditions,
        "status:open $id",
        &[
            "syntax error: Unexpected `$`. Expected a condition or the end of the input.",
            " --> line 1, column 13",
            "  |",
            "1 | status:open $id",
            "  |             ^",
        ],
    );
}

#[test]
fn display_column_in_sorting_section() {
    assert_render(
        parse_sorting,
        "\\\\title $id",
        &[
            "syntax error: Unexpected `$`. Expected a sort expression (`\\\\`) or the end of the input.",
            " --> line 1, column 9",
            "  |",
            "1 | \\\\title $id",
            "  |         ^",
        ],
    );
}

#[test]
fn condition_in_display_section() {
    assert_render(
        parse_display,
        "$title status:open",
        &[
            "syntax error: Unexpected `status`. Expected a result column (`$`) or the end of the input.",
            " --> line 1, column 8",
            "  |",
            "1 | $title status:open",
            "  |        ^^^^^^",
        ],
    );
}

#[test]
fn non_ascii_input() {
    assert_render(
        parse,
        "#issues título:'ñ' é:)",
        &[
            "syntax error: Unexpected `)`, which has no matching `(`. Expected a value.",
            " --> line 1, column 22",
            "  |",
            "1 | #issues título:'ñ' é:)",
            "  |                      ^",
            "  |",
            "  = hint: Remove this `)`, or add a `(` where the group should begin.",
        ],
    );
}

// ------------------------------------------------------------------------------------------------
// The structured fields, which consumers use to build their own presentation
// ------------------------------------------------------------------------------------------------

#[test]
fn structured_fields_of_an_unexpected_token() {
    let input = "#issues\n  title:open status:>";
    let err = parse(input).unwrap_err();
    assert_eq!(err.kind, ErrorKind::Syntax);
    assert_eq!(err.message, "Unexpected end of input. Expected a value.");
    assert_eq!(err.expected, vec!["a value".to_string()]);
    assert_eq!(err.found.as_deref(), Some("end of input"));
    assert_eq!(err.hint, None);
    assert!(err.labels.is_empty());
    let span = err.span.unwrap();
    assert_eq!(
        span.start,
        Position {
            offset: 29,
            line: 2,
            column: 22
        }
    );
    assert!(span.is_empty());
    assert_eq!(
        err.to_string(),
        format!("{} (line 2, column 22)", err.message)
    );
}

#[test]
fn found_word_is_reported_whole_and_spanned() {
    let input = "#issues $title status";
    let err = parse(input).unwrap_err();
    assert_eq!(err.found.as_deref(), Some("`status`"));
    assert_eq!(&input[err.span.unwrap().range()], "status");
}

#[test]
fn end_of_input_is_located_after_the_last_significant_character() {
    // Trailing whitespace and newlines are skipped, so the error points at the end of the text the
    // user actually wrote rather than at an empty line below it.
    let input = "#issues $title->  \n\n";
    let span = parse(input).unwrap_err().span.unwrap();
    assert_eq!((span.start.line, span.start.column), (1, 17));
}

#[test]
fn mismatched_bracket_has_a_label_at_the_opening_bracket() {
    let input = "#issues foo:[1 2)";
    let err = parse(input).unwrap_err();
    assert_eq!(&input[err.span.unwrap().range()], ")");
    assert_eq!(err.expected, vec!["`]`".to_string()]);
    assert_eq!(err.found.as_deref(), Some("`)`"));
    assert_eq!(err.labels.len(), 1);
    assert_eq!(&input[err.labels[0].span.range()], "[");
    assert_eq!(err.labels[0].message, "`[` opened here");
}

#[test]
fn spans_of_section_errors_are_relative_to_the_section() {
    let err = parse_conditions("status:open $id").unwrap_err();
    assert_eq!(err.span.unwrap().start.offset, 12);
}

#[test]
fn error_serializes_to_json() {
    let err = parse("#issues foo:)").unwrap_err();
    let json = serde_json::to_value(&err).unwrap();
    assert_eq!(json["kind"], "syntax");
    assert_eq!(json["span"]["start"]["line"], 1);
    assert_eq!(json["span"]["start"]["column"], 13);
    assert_eq!(json["span"]["start"]["offset"], 12);
    assert_eq!(json["found"], "`)`");
    assert_eq!(json["expected"][0], "a value");
}

#[test]
fn delimiters_in_strings_and_comments_are_ignored() {
    // Brackets inside strings and comments must not be mistaken for unbalanced delimiters. These
    // inputs are valid...
    assert!(parse("#issues title:'(' // ]\n body:\"[\" /* { */").is_ok());
    // ...and in this invalid one, the real problem (the missing value) is reported.
    let err = parse("#issues title:'(' // ]\n body:").unwrap_err();
    assert_eq!(err.message, "Unexpected end of input. Expected a value.");
}
