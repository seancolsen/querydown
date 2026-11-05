//! Syntax highlighting: classifying the spans of Querydown source by the role they play.
//!
//! [`tokenize`] combines two sources of information:
//!
//! 1. **The parser.** The grammar records a _mark_ — a span plus a [`TokenKind`] — wherever it
//!    recognizes something whose role matters for highlighting (see `mark` in the parser's
//!    utilities). Marks are kept in parser state that is truncated whenever the parser backtracks,
//!    discarding the marks of any alternative it abandons, so what remains describes the parse that
//!    actually succeeded. This is what tells a bare word apart as a column
//!    (`$title`), a string literal (`title:performance`), or a search term (`#issues performance`),
//!    with exactly the rules the compiler uses.
//!
//! 2. **A lexer.** Source being edited is often invalid, and a parse that fails keeps only the marks
//!    of the constructs it completed before the failure. A total lexer ([`lexer`]) therefore
//!    classifies every character by its lexical shape (plus a few look-behind heuristics), and the
//!    parser's marks override its guesses wherever they exist. Valid source is highlighted entirely
//!    by the parser's understanding of it; invalid source degrades gracefully, construct by
//!    construct, to the lexer's.

mod kind;
mod lexer;

use std::cmp::Reverse;
use std::ops::Range;

pub use kind::TokenKind;

use crate::tokens::{LITERAL_FALSE, LITERAL_NULL, LITERAL_TRUE};

/// The names of the built-in constants (written with a `@` sigil, e.g. `@now`), which are
/// highlighted as [`TokenKind::Constant`] rather than [`TokenKind::Variable`]. The compiler defines
/// their meaning; a test there keeps this list in step.
pub const BUILTIN_CONSTANTS: [&str; 5] =
    [LITERAL_TRUE, LITERAL_FALSE, LITERAL_NULL, "now", "infinity"];

/// A classified span of source code.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Token {
    pub kind: TokenKind,
    /// The token's location, as a range of byte offsets into the source.
    pub span: Range<usize>,
}

/// Which grammar to read source with: a whole query, or one of the sections parsed by
/// [`parse_definitions`](crate::parse_definitions), [`parse_conditions`](crate::parse_conditions),
/// [`parse_sorting`](crate::parse_sorting), and [`parse_display`](crate::parse_display).
///
/// The grammar matters because the same text can mean different things in different places — e.g.
/// `#issues` is the base table at the start of a whole query, but a related table within a
/// condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Grammar {
    Query,
    Definitions,
    Conditions,
    Sorting,
    Display,
}

/// Classifies `source` for syntax highlighting, reading it with the given `grammar`.
///
/// The tokens are in source order and don't overlap. Every character other than whitespace belongs
/// to exactly one token. This never fails: source that doesn't parse is still fully tokenized (see
/// the [module documentation](self)).
pub fn tokenize(source: &str, grammar: Grammar) -> Vec<Token> {
    let marks = crate::record_marks(source, grammar);
    merge(source, lexer::lex(source), marks)
}

/// Overlays the parser's `marks` onto the lexer's tokens.
///
/// Each byte of a lexed token takes the kind of the innermost mark covering it, or keeps its lexed
/// kind if no mark does. (Of two marks with the same span, the one recorded last wins; that is the
/// outer of two nested parsers, which knows more about the context.) Comments and string escapes
/// keep their lexed kinds, so a mark that spans one of them can't recolor it. Each lexed token is
/// then split wherever the kind changes; tokens are never merged, so a run of brackets stays a run
/// of individual brackets.
fn merge(source: &str, lexed: Vec<Token>, mut marks: Vec<Token>) -> Vec<Token> {
    let mut kinds: Vec<Option<TokenKind>> = vec![None; source.len()];
    for token in &lexed {
        kinds[token.span.clone()].fill(Some(token.kind));
    }
    // A stable sort, so marks of equal length stay in the order they were recorded.
    marks.sort_by_key(|mark| Reverse(mark.span.len()));
    for mark in marks {
        let end = mark.span.end.min(source.len());
        let start = mark.span.start.min(end);
        for kind in kinds[start..end].iter_mut().flatten() {
            if !matches!(kind, TokenKind::Comment | TokenKind::StringEscape) {
                *kind = mark.kind;
            }
        }
    }
    let mut tokens = Vec::with_capacity(lexed.len());
    for token in lexed {
        let mut start = token.span.start;
        for i in token.span.clone() {
            if kinds[i] != kinds[start] {
                tokens.push(Token {
                    kind: kinds[start].expect("lexed bytes have a kind"),
                    span: start..i,
                });
                start = i;
            }
        }
        tokens.push(Token {
            kind: kinds[start].expect("lexed bytes have a kind"),
            span: start..token.span.end,
        });
    }
    tokens
}

#[cfg(test)]
mod tests;
