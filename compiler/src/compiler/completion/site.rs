//! The lexical side of completion: the word being typed, what introduces it, and whether the cursor
//! is somewhere completion doesn't apply.

use querydown_parser::ast::{PathPart, TableWithMany};
use querydown_parser::highlight::{Token, TokenKind};

use super::annotation::{self, AnnotationPosition};
use super::unquote;

/// Where the cursor is, lexically.
pub(super) struct Site {
    /// The start of the word being typed (which ends at the cursor). Equal to the cursor when no
    /// word has been typed yet, e.g. right after a `.`.
    pub word_start: usize,
    /// The end of the word around the cursor, which may continue past it.
    pub word_end: usize,
    pub kind: SiteKind,
}

pub(super) enum SiteKind {
    /// Querydown code, with the sigil just before the word.
    Code(Sigil),
    /// Within a column annotation, with the position the completion replaces from.
    Annotation(AnnotationPosition, usize),
    /// Somewhere nothing is completed: a string, a comment, a number, a flag, and so on.
    Blocked,
}

/// What immediately precedes the word being typed, which says what the word can be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Sigil {
    /// `#`: a table.
    Table,
    /// `@@`: a function, called directly.
    Function,
    /// `|`: a scalar function, piped.
    Pipe,
    /// `%`: an aggregate function.
    Aggregate,
    /// `%` after a `%%( … )` window definition: a window function.
    Window,
    /// `@`: a constant or variable.
    Variable,
    /// None of the above, e.g. after `$`, `.`, `(`, or whitespace: typically a column.
    None,
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

pub(super) fn analyze(source: &str, cursor: usize, tokens: &[Token]) -> Site {
    let word_start = source[..cursor]
        .char_indices()
        .rev()
        .take_while(|(_, c)| is_word_char(*c))
        .last()
        .map_or(cursor, |(i, _)| i);
    let word_end = source[cursor..]
        .char_indices()
        .find(|(_, c)| !is_word_char(*c))
        .map_or(source.len(), |(i, _)| cursor + i);
    let kind = site_kind(source, cursor, word_start, tokens);
    Site {
        word_start,
        word_end,
        kind,
    }
}

fn site_kind(source: &str, cursor: usize, word_start: usize, tokens: &[Token]) -> SiteKind {
    if within_string_or_comment(source, cursor, tokens) {
        return SiteKind::Blocked;
    }
    if let Some(position) = annotation::position(source, word_start, tokens) {
        // An annotation's `@true`, `@false` and `@null` are completed together with their `@`.
        let from = match source[..word_start].ends_with('@') {
            true => word_start - 1,
            false => word_start,
        };
        return match position {
            Some(position) => SiteKind::Annotation(position, from),
            None => SiteKind::Blocked,
        };
    }
    let word = &source[word_start..cursor];
    if word.starts_with(|c: char| c.is_ascii_digit()) {
        // A number, duration or date.
        return SiteKind::Blocked;
    }
    let before = &source[..word_start];
    let sigil = if before.ends_with("@@") {
        Sigil::Function
    } else if before.ends_with('@') {
        Sigil::Variable
    } else if before.ends_with('#') {
        Sigil::Table
    } else if before.ends_with('|') {
        Sigil::Pipe
    } else if before.ends_with("%%") {
        // `%%` must be followed by a window definition.
        return SiteKind::Blocked;
    } else if before.ends_with(")%") && closes_window(source, word_start - 2, tokens) {
        Sigil::Window
    } else if before.ends_with('%') {
        Sigil::Aggregate
    } else if before.ends_with("\\\\") {
        // A sorting expression.
        Sigil::None
    } else if before.ends_with("..") || before.ends_with(['\\', '`', '\'', '"']) {
        // A range bound is a value; flags and quoted text aren't completed.
        return SiteKind::Blocked;
    } else {
        Sigil::None
    };
    SiteKind::Code(sigil)
}

/// Whether the cursor is within (not just after) a quoted string, a quoted identifier or a comment.
fn within_string_or_comment(source: &str, cursor: usize, tokens: &[Token]) -> bool {
    tokens.iter().any(|token| {
        let text = &source[token.span.clone()];
        if token.span.start >= cursor || token.span.end < cursor {
            return false;
        }
        let open = match token.kind {
            TokenKind::Comment => {
                text.starts_with("//") || token.span.end > cursor || !text.ends_with("*/")
            }
            TokenKind::String | TokenKind::Column | TokenKind::Table | TokenKind::BaseTable => {
                let quote = text.trim_start_matches('#').chars().next();
                match quote {
                    Some(quote @ ('"' | '\'' | '`')) => {
                        let body = &text.trim_start_matches('#')[1..];
                        token.span.end > cursor || !closes_quote(body, quote)
                    }
                    _ => false,
                }
            }
            _ => false,
        };
        open
    })
}

/// Whether `body` (the text after an opening `quote`) contains the closing quote.
fn closes_quote(body: &str, quote: char) -> bool {
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                chars.next();
            }
            c if c == quote => return true,
            _ => {}
        }
    }
    false
}

/// The brackets that are open at `position`, innermost last, as `(offset, character)` pairs.
pub(super) fn open_brackets(source: &str, position: usize, tokens: &[Token]) -> Vec<(usize, char)> {
    let mut stack: Vec<(usize, char)> = vec![];
    for token in tokens {
        if token.span.start >= position {
            break;
        }
        if token.kind != TokenKind::Bracket {
            continue;
        }
        let c = source[token.span.clone()].chars().next().unwrap_or(' ');
        match c {
            '(' | '[' | '{' => stack.push((token.span.start, c)),
            ')' | ']' | '}' => {
                if stack.last().is_some_and(|(_, open)| closing(*open) == c) {
                    stack.pop();
                }
            }
            _ => {}
        }
    }
    stack
}

pub(super) fn closing(open: char) -> char {
    match open {
        '(' => ')',
        '[' => ']',
        _ => '}',
    }
}

/// Whether the `)` at `close` ends a `%%( … )` window definition.
fn closes_window(source: &str, close: usize, tokens: &[Token]) -> bool {
    // The brackets open just after the `)` are those open before it, less the one it closes.
    let before = open_brackets(source, close, tokens);
    let after = open_brackets(source, close + 1, tokens);
    match before.last() {
        Some((open, '(')) if after.len() < before.len() => source[..*open].ends_with("%%"),
        _ => false,
    }
}

/// The path written immediately before `word_start`, e.g. `author.team` before `author.team.na`,
/// read from the tokens. Empty if the word doesn't follow a `.`.
pub(super) fn path_before(source: &str, word_start: usize, tokens: &[Token]) -> Vec<PathPart> {
    let preceding: Vec<&Token> = tokens
        .iter()
        .take_while(|token| token.span.end <= word_start)
        .collect();
    let mut parts = vec![];
    let mut end = word_start;
    let mut iter = preceding.into_iter().rev();
    loop {
        let Some(separator) = iter.next() else { break };
        if separator.span.end != end || &source[separator.span.clone()] != "." {
            break;
        }
        let Some(part) = iter.next() else { break };
        if part.span.end != separator.span.start {
            break;
        }
        let part_start = part.span.start;
        let text = &source[part.span.clone()];
        let part = match text.strip_prefix('#') {
            Some(table) => PathPart::TableWithMany(TableWithMany {
                table: unquote(table),
                condition_set: Default::default(),
                linking_column: None,
            }),
            None if text.starts_with(|c: char| is_word_char(c) || c == '`') => {
                PathPart::Column(unquote(text))
            }
            None => break,
        };
        parts.push(part);
        end = part_start;
    }
    parts.reverse();
    parts
}
