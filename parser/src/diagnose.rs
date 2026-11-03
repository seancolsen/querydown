//! Translates the parser's raw failures into helpful [`Error`]s.
//!
//! Two sources of information are combined:
//!
//! 1. The [`Rich`] error that Chumsky reports for the furthest point the parser reached, which
//!    knows what was found there and what the grammar would have accepted instead. The grammar
//!    labels its rules (see the `labels` module) so that the "expected" set describes concepts like
//!    "a value" rather than individual characters, and this module filters out whatever
//!    character-level noise remains.
//! 2. A simple lexical [`scan`] of the source for unbalanced brackets, unterminated strings, and
//!    unterminated block comments. These are among the most common syntax mistakes, and the scan
//!    can explain them far more precisely than the parser can — e.g. by pointing at the bracket that
//!    was never closed rather than at the end of the input.

use chumsky::error::{Rich, RichPattern, RichReason};

use crate::error::{Error, ErrorKind, Span};
use crate::labels;

/// Builds the [`Error`] for a failed parse of `source`, given the errors Chumsky reported.
pub(crate) fn diagnose(source: &str, errors: Vec<Rich<'_, char>>) -> Error {
    let from_parser = errors
        .into_iter()
        .next()
        .map(|rich| from_rich(source, rich))
        // Chumsky always reports at least one error on failure, but just in case:
        .unwrap_or_else(|| {
            Error::new(ErrorKind::Syntax, "Invalid Querydown code.")
                .with_span(Span::from_range(source, 0..0))
        });
    let problem = scan(source);
    reconcile(source, from_parser, problem)
}

// ------------------------------------------------------------------------------------------------
// Translating Chumsky's errors
// ------------------------------------------------------------------------------------------------

fn from_rich(source: &str, rich: Rich<'_, char>) -> Error {
    let span = rich.span();
    let (start, end) = (span.start, span.end);
    match rich.reason() {
        RichReason::Custom(message) => {
            let mut message = message.clone();
            if !message.ends_with('.') {
                message.push('.');
            }
            Error::new(ErrorKind::Syntax, message).with_span(Span::from_range(source, start..end))
        }
        RichReason::ExpectedFound { expected, found } => {
            let expected = describe_expected(expected);
            match found {
                Some(c) => {
                    let (found_text, end) = found_token(source, start, **c);
                    let found = code(&found_text);
                    let mut err = syntax_error(
                        source,
                        start..end,
                        format!("Unexpected {found}."),
                        expected,
                        found,
                    );
                    err.hint = hint_for(&err, &found_text);
                    err
                }
                None => {
                    // The input ended too soon. Point just past the last significant character
                    // rather than at the very end, which may be after trailing whitespace or on a
                    // line of its own.
                    let at = source[..start].trim_end().len();
                    let mut err = syntax_error(
                        source,
                        at..at,
                        "Unexpected end of input.".to_string(),
                        expected,
                        "end of input".to_string(),
                    );
                    err.hint = hint_for(&err, "");
                    err
                }
            }
        }
    }
}

fn syntax_error(
    source: &str,
    range: std::ops::Range<usize>,
    mut message: String,
    expected: Vec<String>,
    found: String,
) -> Error {
    if !expected.is_empty() {
        message.push_str(&format!(" Expected {}.", join_alternatives(&expected)));
    }
    let mut err = Error::new(ErrorKind::Syntax, message).with_span(Span::from_range(source, range));
    err.expected = expected;
    err.found = Some(found);
    err
}

/// Describes the token found at byte offset `start` (whose first character is `c`), returning its
/// text and the byte offset at which it ends. A word is reported whole (e.g. `issues` rather than
/// just `i`) since that is what the user will recognize.
fn found_token(source: &str, start: usize, c: char) -> (String, usize) {
    let is_word_char = |c: char| c.is_alphanumeric() || c == '_';
    if is_word_char(c) {
        let len = source[start..]
            .find(|c: char| !is_word_char(c))
            .unwrap_or(source.len() - start);
        (source[start..start + len].to_string(), start + len)
    } else {
        (c.to_string(), start + c.len_utf8())
    }
}

/// Turns Chumsky's set of expected patterns into a deduplicated list of human-readable
/// descriptions.
///
/// The grammar labels the constructs a user would think in terms of (see [`labels`]). Whatever
/// isn't labelled is low-level detail — the individual characters that could have continued an
/// operator, a number, a path, etc. — which only adds noise when a label is also available. So if
/// any labels are present, raw characters are dropped, except for closing brackets, which are
/// essential to understanding many errors.
fn describe_expected(patterns: &[RichPattern<'_, char>]) -> Vec<String> {
    let has_labels = patterns.iter().any(|p| match p {
        RichPattern::Label(l) => l.as_ref() != labels::WHITESPACE && builtin_label(l).is_none(),
        _ => false,
    });
    let mut out: Vec<String> = Vec::new();
    let mut expects_end = false;
    for pattern in patterns {
        let description = match pattern {
            RichPattern::Label(l) if l.as_ref() == labels::WHITESPACE => None,
            RichPattern::Label(l) => match builtin_label(l) {
                Some(_) if has_labels => None,
                Some(description) => description.map(str::to_string),
                None => Some(l.to_string()),
            },
            RichPattern::Token(c) => {
                let c = **c;
                let keep = if has_labels {
                    matches!(c, ')' | ']' | '}')
                } else {
                    !c.is_whitespace()
                };
                keep.then(|| code(&c.to_string()))
            }
            RichPattern::Identifier(s) => Some(code(s)),
            RichPattern::EndOfInput => {
                expects_end = true;
                None
            }
            RichPattern::Any | RichPattern::SomethingElse => None,
            _ => None,
        };
        if let Some(d) = description {
            if !out.contains(&d) {
                out.push(d);
            }
        }
    }
    if expects_end {
        out.push("the end of the input".to_string());
    }
    out
}

/// Recognizes the labels that Chumsky's own text parsers use. Like raw characters, these are
/// low-level detail, so they are dropped whenever one of our own labels is available. Otherwise they
/// are translated into our vocabulary (or dropped, if they only describe whitespace).
///
/// Returns `None` if `label` is not one of Chumsky's, and `Some(translation)` if it is.
fn builtin_label(label: &str) -> Option<Option<&'static str>> {
    match label {
        "identifier" => Some(Some(labels::NAME)),
        "int" | "digit" | "non-zero digit" | "number" => Some(Some("a number")),
        "inline whitespace" | "newline" => Some(None),
        _ => None,
    }
}

/// Formats `text` as inline code, Markdown-style: `` `text` ``. Text containing a backtick is
/// wrapped in double backticks and spaces (`` `` ` `` ``), as Markdown requires.
fn code(text: &str) -> String {
    if text.contains('`') {
        format!("`` {text} ``")
    } else {
        format!("`{text}`")
    }
}

/// Joins alternatives into an English list: `a`, `a or b`, `a, b, or c`.
fn join_alternatives(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [a, b] => format!("{a} or {b}"),
        [init @ .., last] => format!("{}, or {last}", init.join(", ")),
    }
}

/// Suggests a fix for a few common mistakes, given the error and the text of the found token.
fn hint_for(err: &Error, found: &str) -> Option<String> {
    let expects = |label: &str| err.expected.iter().any(|e| e == label);
    let is_word = found
        .chars()
        .next()
        .is_some_and(|c| c.is_alphabetic() || c == '_');
    if expects(labels::BASE_TABLE) {
        return Some(if is_word {
            format!("Table names are written with a `#` prefix. Did you mean `#{found}`?")
        } else {
            "A query must begin with its base table, written with a `#` prefix, e.g. `#issues`."
                .to_string()
        });
    }
    None
}

// ------------------------------------------------------------------------------------------------
// Scanning for unbalanced delimiters
// ------------------------------------------------------------------------------------------------

/// A lexical problem with the source's delimiters. Offsets are in bytes.
#[derive(Debug, Clone, PartialEq)]
enum Problem {
    /// An opening bracket that is never closed.
    Unclosed { open: char, at: usize },
    /// A closing bracket with no corresponding opening bracket.
    UnexpectedClose { close: char, at: usize },
    /// A closing bracket that doesn't match the most recent unclosed opening bracket.
    Mismatched {
        open: char,
        open_at: usize,
        close: char,
        close_at: usize,
    },
    /// A string (or quoted identifier) that is never closed.
    UnterminatedString { quote: char, at: usize },
    /// A `/* block comment */` that is never closed.
    UnterminatedComment { at: usize },
}

fn closer_for(open: char) -> char {
    match open {
        '(' => ')',
        '[' => ']',
        _ => '}',
    }
}

fn opener_for(close: char) -> char {
    match close {
        ')' => '(',
        ']' => '[',
        _ => '{',
    }
}

/// Finds the first delimiter problem in `source`, if any, skipping over the contents of strings,
/// quoted identifiers, and comments in the same way the parser does.
fn scan(source: &str) -> Option<Problem> {
    let bytes = source.as_bytes();
    let mut stack: Vec<(char, usize)> = Vec::new();
    let mut i = 0;
    // All of the delimiters we care about are ASCII, so it is safe to scan byte-by-byte: in UTF-8,
    // the bytes of a multi-byte character never fall in the ASCII range.
    while i < bytes.len() {
        let c = bytes[i] as char;
        let next = bytes.get(i + 1).map(|b| *b as char);
        match (c, next) {
            ('/', Some('/')) => {
                i = source[i..].find('\n').map_or(bytes.len(), |n| i + n);
                continue;
            }
            ('/', Some('*')) => {
                let at = i;
                let mut depth = 0;
                loop {
                    if i >= bytes.len() {
                        return Some(Problem::UnterminatedComment { at });
                    }
                    if source[i..].starts_with("/*") {
                        depth += 1;
                        i += 2;
                    } else if source[i..].starts_with("*/") {
                        depth -= 1;
                        i += 2;
                        if depth == 0 {
                            break;
                        }
                    } else {
                        i += 1;
                    }
                }
                continue;
            }
            ('"' | '\'' | '`', _) => {
                let at = i;
                i += 1;
                loop {
                    match bytes.get(i).map(|b| *b as char) {
                        None => return Some(Problem::UnterminatedString { quote: c, at }),
                        Some('\\') => i += 2,
                        Some(q) if q == c => break,
                        Some(_) => i += 1,
                    }
                }
            }
            ('(' | '[' | '{', _) => stack.push((c, i)),
            (')' | ']' | '}', _) => match stack.pop() {
                None => return Some(Problem::UnexpectedClose { close: c, at: i }),
                Some((open, open_at)) if closer_for(open) != c => {
                    return Some(Problem::Mismatched {
                        open,
                        open_at,
                        close: c,
                        close_at: i,
                    })
                }
                Some(_) => {}
            },
            _ => {}
        }
        i += 1;
    }
    stack
        .last()
        .map(|&(open, at)| Problem::Unclosed { open, at })
}

/// Decides how to report a failed parse, given the parser's own error and the first delimiter
/// problem (if any).
///
/// The delimiter problem is preferred when it explains the parser's error — i.e. when the parser
/// stumbled at the very spot of the problem, or (for a problem only detectable at the end of the
/// input) when the parser ran out of input. Otherwise the parser found an earlier, unrelated
/// mistake, which is reported instead.
fn reconcile(source: &str, parser_error: Error, problem: Option<Problem>) -> Error {
    let Some(problem) = problem else {
        return parser_error;
    };
    let span = |range: std::ops::Range<usize>| Span::from_range(source, range);
    let parser_offset = parser_error.span.map_or(0, |s| s.start.offset);
    let parser_hit_end = parser_error.found.as_deref() == Some("end of input");

    match problem {
        Problem::UnexpectedClose { close, at } if parser_offset >= at => {
            let open = opener_for(close);
            let mut message = format!("Unexpected `{close}`, which has no matching `{open}`.");
            // The parser usually stumbles on the stray bracket itself, in which case what it
            // expected instead is still worth knowing.
            let expected = if parser_offset == at {
                parser_error.expected
            } else {
                Vec::new()
            };
            if !expected.is_empty() {
                message.push_str(&format!(" Expected {}.", join_alternatives(&expected)));
            }
            let mut err = Error::new(ErrorKind::Syntax, message)
                .with_span(span(at..at + 1))
                .with_hint(format!(
                    "Remove this `{close}`, or add a `{open}` where the group should begin."
                ));
            err.expected = expected;
            err.found = Some(format!("`{close}`"));
            err
        }
        Problem::Mismatched {
            open,
            open_at,
            close,
            close_at,
        } if parser_offset >= close_at => {
            let expected = closer_for(open);
            let mut err = Error::new(
                ErrorKind::Syntax,
                format!(
                    "Mismatched `{close}`. Expected `{expected}` to close the `{open}` opened \
                     earlier."
                ),
            )
            .with_span(span(close_at..close_at + 1))
            .with_label(span(open_at..open_at + 1), format!("`{open}` opened here"));
            err.expected = vec![format!("`{expected}`")];
            err.found = Some(format!("`{close}`"));
            err
        }
        Problem::Unclosed { open, at } if parser_hit_end => {
            let close = closer_for(open);
            let end = source.trim_end().len();
            let mut err = Error::new(
                ErrorKind::Syntax,
                format!(
                    "Unclosed `{open}`. Expected a matching `{close}` before the end of input."
                ),
            )
            .with_span(span(at..at + 1))
            .with_label(
                span(end..end),
                format!("expected `{close}` here or earlier"),
            );
            err.expected = vec![format!("`{close}`")];
            err.found = Some("end of input".to_string());
            err
        }
        Problem::UnterminatedString { quote, at } if parser_hit_end => {
            let what = if quote == '`' {
                "quoted identifier"
            } else {
                "string"
            };
            let quote_code = code(&quote.to_string());
            let escaped_code = code(&format!("\\{quote}"));
            let mut err = Error::new(
                ErrorKind::Syntax,
                format!("Unterminated {what}. Expected a closing {quote_code}."),
            )
            .with_span(span(at..at + 1))
            .with_hint(format!(
                "Add a {quote_code} where the {what} should end. To include a literal \
                 {quote_code} within the {what}, escape it as {escaped_code}."
            ));
            err.expected = vec![quote_code];
            err.found = Some("end of input".to_string());
            err
        }
        Problem::UnterminatedComment { at } if parser_hit_end => {
            let mut err = Error::new(
                ErrorKind::Syntax,
                "Unterminated block comment. Expected a closing `*/`.",
            )
            .with_span(span(at..at + 2));
            err.expected = vec!["`*/`".to_string()];
            err.found = Some("end of input".to_string());
            err
        }
        // The parser found an unrelated, earlier error. If a bracket before it was never closed,
        // that's still worth pointing out, as it may well be the real culprit.
        Problem::Unclosed { open, at } if at < parser_offset => {
            parser_error.with_label(span(at..at + 1), format!("this `{open}` is never closed"))
        }
        _ => parser_error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan() {
        use Problem::*;
        assert_eq!(scan("#t foo:[1 2] {a:(1)}"), None);
        assert_eq!(scan("#t a:'(' b:\"[\" `{`"), None);
        assert_eq!(
            scan("#t a:'it\\'s (' // ( unbalanced\n /* [ /* { */ */"),
            None
        );
        assert_eq!(scan("#t foo:[1"), Some(Unclosed { open: '[', at: 7 }));
        assert_eq!(scan("#t (a [b] "), Some(Unclosed { open: '(', at: 3 }));
        assert_eq!(scan("#t a)"), Some(UnexpectedClose { close: ')', at: 4 }));
        assert_eq!(
            scan("#t [a)"),
            Some(Mismatched {
                open: '[',
                open_at: 3,
                close: ')',
                close_at: 5
            })
        );
        assert_eq!(
            scan("#t a:\"abc"),
            Some(UnterminatedString { quote: '"', at: 5 })
        );
        assert_eq!(
            scan("#t a:'abc\\'"),
            Some(UnterminatedString { quote: '\'', at: 5 })
        );
        assert_eq!(scan("#t /* a /* b */"), Some(UnterminatedComment { at: 3 }));
        // A problem inside a string or comment is ignored; the first real one is reported.
        assert_eq!(
            scan("#t 'a)' // )\n ]"),
            Some(UnexpectedClose { close: ']', at: 14 })
        );
        // Non-ASCII text doesn't confuse the byte-wise scan.
        assert_eq!(scan("#t é:'ü)' (é"), Some(Unclosed { open: '(', at: 12 }));
    }

    #[test]
    fn test_join_alternatives() {
        let s =
            |v: &[&str]| join_alternatives(&v.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert_eq!(s(&["a"]), "a");
        assert_eq!(s(&["a", "b"]), "a or b");
        assert_eq!(s(&["a", "b", "c"]), "a, b, or c");
    }
}
