//! Checks that the compiler fails gracefully on arbitrary invalid input: it must never panic (which,
//! in the WebAssembly build, aborts the whole module), and every error it returns must be
//! well-formed — in particular, a syntax error's location must lie within the input.
//!
//! The inputs are mutations of the valid queries in the test corpus: prefixes of each query
//! (simulating a user part-way through typing it, which is what an editor that compiles on every
//! keystroke will see) and the query with any one of its punctuation characters deleted (simulating
//! a typo that breaks the query's structure).
//!
//! To keep the test fast, mutations that only ever differ in the name they leave behind are
//! skipped: prefixes ending in the middle of a word, and deletions of letters or digits.

use std::collections::HashSet;
use std::panic::{catch_unwind, AssertUnwindSafe};

use super::corpus_loader::{build_options, load_cases};
use crate::{Compiler, Error, ErrorKind, Position, Span};

/// Returns a description of what is wrong with `span` as a location within `input`, if anything.
fn check_span(input: &str, span: &Span) -> Option<String> {
    let (start, end) = (span.start.offset, span.end.offset);
    if start > end || end > input.len() {
        return Some(format!("span {start}..{end} is out of bounds"));
    }
    if !input.is_char_boundary(start) || !input.is_char_boundary(end) {
        return Some(format!("span {start}..{end} is not on char boundaries"));
    }
    for position in [span.start, span.end] {
        if position != Position::from_offset(input, position.offset) {
            return Some(format!("inconsistent line/column in {position:?}"));
        }
    }
    None
}

/// Returns a description of what is wrong with `err`, if anything.
fn check_error(input: &str, err: &Error) -> Option<String> {
    if err.message.trim().is_empty() {
        return Some("empty message".to_string());
    }
    if err.kind == ErrorKind::Syntax {
        let Some(span) = &err.span else {
            return Some("syntax error without a span".to_string());
        };
        if let Some(problem) = check_span(input, span) {
            return Some(problem);
        }
        if err.found.is_none() && err.expected.is_empty() && err.labels.is_empty() {
            // Every syntax error should explain itself beyond the bare message, unless the message
            // came from a custom validation (which is specific enough on its own).
            if err.message.starts_with("Unexpected") {
                return Some("unexpected-token error without `found` or `expected`".to_string());
            }
        }
    }
    for label in &err.labels {
        if let Some(problem) = check_span(input, &label.span) {
            return Some(format!("label: {problem}"));
        }
    }
    None
}

#[test]
fn mutated_corpus_inputs_fail_gracefully() {
    let mut failures: Vec<String> = Vec::new();
    let mut checked = 0;
    for case in load_cases() {
        let input = case.args.first().expect("missing input code block").clone();
        let options = build_options(case.options.identifier_resolution, &case.options.dialect);
        let compiler = Compiler::new(&case.options.schema_json, options).unwrap();

        let is_word_char = |c: char| c.is_alphanumeric() || c == '_';
        let prefixes = input
            .char_indices()
            .filter(|&(i, c)| {
                let previous = input[..i].chars().next_back();
                !(is_word_char(c) && previous.is_some_and(is_word_char))
            })
            .map(|(i, _)| input[..i].to_string())
            .chain(std::iter::once(input.clone()));
        let deletions = input
            .char_indices()
            .filter(|&(_, c)| !is_word_char(c) && !c.is_whitespace())
            .map(|(i, c)| {
                let mut s = input.clone();
                s.replace_range(i..i + c.len_utf8(), "");
                s
            });
        let mutations: HashSet<String> = prefixes.chain(deletions).collect();

        for mutated in mutations {
            checked += 1;
            let outcome = catch_unwind(AssertUnwindSafe(|| compiler.compile(mutated.clone())));
            let problem = match outcome {
                Err(_) => Some("panicked".to_string()),
                Ok(Ok(_)) => None,
                Ok(Err(err)) => check_error(&mutated, &err),
            };
            if let Some(problem) = problem {
                failures.push(format!("{problem}: {mutated:?}"));
            }
        }
    }
    assert!(checked > 5000, "too few inputs checked ({checked})");
    assert!(
        failures.is_empty(),
        "{} of {checked} mutated inputs failed:\n{}",
        failures.len(),
        failures
            .iter()
            .take(30)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
