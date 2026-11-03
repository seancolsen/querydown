//! The error type shared by every stage of Querydown processing.
//!
//! All fallible public entry points — parsing (here) and compilation (in the `querydown` crate,
//! which re-exports this module's types) — report failures as an [`Error`]. An error always carries
//! a [`kind`](Error::kind) and a human-readable [`message`](Error::message). Errors caused by a
//! specific piece of the input — currently every [`ErrorKind::Syntax`] error — also carry a
//! [`span`](Error::span) locating that piece, and may carry further detail: secondary
//! [`labels`](Error::labels), the [`expected`](Error::expected) and [`found`](Error::found)
//! input, and a [`hint`](Error::hint) on how to fix the problem.
//!
//! For display, [`Error`] implements [`std::fmt::Display`] (a one-line summary including the line
//! and column) and offers [`Error::render`], which draws a multi-line diagnostic with the offending
//! source line(s) underlined.

use std::fmt;
use std::ops::Range;

use serde::Serialize;

/// The broad category of an [`Error`].
///
/// This enum is `#[non_exhaustive]`: new kinds may be added in future versions, so match it with a
/// wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum ErrorKind {
    /// The Querydown source is not syntactically valid. Syntax errors always have a
    /// [`span`](Error::span).
    Syntax,
    /// The database schema supplied to the compiler is invalid (e.g. malformed JSON, or a link that
    /// refers to a table that doesn't exist). The error is in the schema, not the Querydown source,
    /// so these errors have no span.
    Schema,
    /// The Querydown source is syntactically valid, but can't be compiled — for example because it
    /// refers to a table, column, or function that doesn't exist, or uses a feature in an
    /// unsupported way.
    ///
    /// Compile errors do not yet carry a span, because the parsed syntax tree does not currently
    /// record source locations.
    Compile,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ErrorKind::Syntax => "syntax error",
            ErrorKind::Schema => "schema error",
            ErrorKind::Compile => "compile error",
        })
    }
}

/// A location within the source text.
///
/// The [`offset`](Self::offset) is a **byte** offset, suitable for slicing the source string in
/// Rust (`&source[offset..]`). The [`line`](Self::line) and [`column`](Self::column) are both
/// **1-based**, and the column counts Unicode scalar values (`char`s), not bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct Position {
    /// Byte offset from the beginning of the source.
    pub offset: usize,
    /// 1-based line number. Lines are separated by `\n`.
    pub line: usize,
    /// 1-based column number, counted in `char`s from the beginning of the line.
    pub column: usize,
}

impl Position {
    /// Computes the position of the given byte `offset` within `source`.
    ///
    /// An `offset` past the end of `source` is clamped to its end, and one falling inside a
    /// multi-byte character is moved back to that character's start.
    pub fn from_offset(source: &str, offset: usize) -> Self {
        let mut offset = offset.min(source.len());
        while !source.is_char_boundary(offset) {
            offset -= 1;
        }
        let before = &source[..offset];
        let line_start = before.rfind('\n').map_or(0, |i| i + 1);
        Position {
            offset,
            line: before.matches('\n').count() + 1,
            column: before[line_start..].chars().count() + 1,
        }
    }
}

/// A contiguous region of the source text, from [`start`](Self::start) (inclusive) to
/// [`end`](Self::end) (exclusive).
///
/// A span may be empty (`start == end`), which happens when the error is at a point _between_
/// characters — most commonly when the input ends unexpectedly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct Span {
    pub start: Position,
    pub end: Position,
}

impl Span {
    /// Builds a span covering the given byte `range` of `source`.
    pub fn from_range(source: &str, range: Range<usize>) -> Self {
        let start = Position::from_offset(source, range.start);
        let end = Position::from_offset(source, range.end.max(range.start));
        Span { start, end }
    }

    /// The span as a byte range, suitable for slicing the source string: `&source[span.range()]`.
    pub fn range(&self) -> Range<usize> {
        self.start.offset..self.end.offset
    }

    /// True if the span covers no characters.
    pub fn is_empty(&self) -> bool {
        self.start.offset == self.end.offset
    }
}

/// A secondary location related to an [`Error`], with a note explaining its relevance — for
/// example, pointing out where an unclosed bracket was opened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Label {
    pub span: Span,
    pub message: String,
}

/// An error produced while parsing or compiling Querydown.
///
/// See the [module documentation](self) for an overview. The fields are public for reading; the
/// struct is `#[non_exhaustive]` so that more detail can be added in future versions without a
/// breaking change. Construct one with [`Error::new`] and the `with_*` builder methods.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct Error {
    /// The broad category of error.
    pub kind: ErrorKind,
    /// A human-readable description of the problem, as one or more complete sentences. It does not
    /// include the location; use [`Display`](fmt::Display) or [`Error::render`] for that.
    pub message: String,
    /// The primary location of the problem within the source, if it is attributable to one. Always
    /// present for [`ErrorKind::Syntax`] errors.
    ///
    /// When the source was parsed in sections (e.g. via `parse_conditions`), the span is relative to
    /// the section's own text.
    pub span: Option<Span>,
    /// Secondary locations relevant to the problem, each with an explanatory note.
    pub labels: Vec<Label>,
    /// For syntax errors: human-readable descriptions of what the parser would have accepted at
    /// [`span`](Self::span), e.g. ``["a value", "`]`"]``. Empty when not applicable.
    pub expected: Vec<String>,
    /// For syntax errors: a description of what was actually found at [`span`](Self::span), e.g.
    /// ``"`)`"`` or `"end of input"`.
    pub found: Option<String>,
    /// A suggestion on how to fix the problem, when one is available.
    pub hint: Option<String>,
}

impl Error {
    /// Creates an error of the given kind with no location or other detail.
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Error {
            kind,
            message: message.into(),
            span: None,
            labels: Vec::new(),
            expected: Vec::new(),
            found: None,
            hint: None,
        }
    }

    /// Shorthand for `Error::new(ErrorKind::Compile, message)`.
    pub fn compile(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Compile, message)
    }

    /// Shorthand for `Error::new(ErrorKind::Schema, message)`.
    pub fn schema(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Schema, message)
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    pub fn with_label(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(Label {
            span,
            message: message.into(),
        });
        self
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Renders a multi-line, human-readable diagnostic for display in a terminal or other
    /// fixed-width context, quoting and underlining the relevant parts of `source`.
    ///
    /// `source` must be the text that was parsed to produce this error (for a section parser, the
    /// section's text). The output has no trailing newline. For example:
    ///
    /// ```text
    /// syntax error: Unexpected `)`. Expected a value.
    ///  --> line 1, column 13
    ///   |
    /// 1 | #issues foo:)
    ///   |             ^
    /// ```
    pub fn render(&self, source: &str) -> String {
        crate::render::render(self, source)
    }
}

impl fmt::Display for Error {
    /// Writes the message followed by the line and column of the error (if known), e.g.
    /// ``Unexpected `)`. Expected a value. (line 1, column 13)``.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)?;
        if let Some(span) = &self.span {
            write!(
                f,
                " (line {}, column {})",
                span.start.line, span.start.column
            )?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_from_offset() {
        let source = "ab\ncdé\nf";
        let p = |offset| {
            let pos = Position::from_offset(source, offset);
            (pos.line, pos.column)
        };
        assert_eq!(p(0), (1, 1));
        assert_eq!(p(2), (1, 3)); // the newline itself
        assert_eq!(p(3), (2, 1));
        assert_eq!(p(5), (2, 3)); // `é`, a two-byte char
        assert_eq!(p(7), (2, 4)); // the newline after `é`
        assert_eq!(p(8), (3, 1));
        assert_eq!(p(9), (3, 2)); // end of input
        assert_eq!(Position::from_offset(source, 100).offset, 9); // clamped
        assert_eq!(Position::from_offset(source, 6).offset, 5); // mid-char, moved back
    }

    #[test]
    fn test_display_includes_location() {
        let source = "x\nyz";
        let err = Error::new(ErrorKind::Syntax, "Oops.").with_span(Span::from_range(source, 3..4));
        assert_eq!(err.to_string(), "Oops. (line 2, column 2)");
        assert_eq!(Error::compile("Nope.").to_string(), "Nope.");
    }
}
