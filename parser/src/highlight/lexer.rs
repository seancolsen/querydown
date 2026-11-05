//! A total lexer: it classifies _any_ input — including input that doesn't parse — into tokens, so
//! that highlighting never breaks while the user is mid-edit.
//!
//! It knows only the lexical shape of the language, plus a few cheap look-behind heuristics (e.g. a
//! word after `|` is a function name). Where the parser succeeds, the semantic marks it records
//! override these guesses; see the [module documentation](super).

use std::ops::Range;

use super::{Token, TokenKind, BUILTIN_CONSTANTS};
use crate::tokens::*;

/// Characters that may follow a backslash to form column control or sort flags, e.g. `\sd2`.
const FLAG_CHARS: [char; 6] = [
    COLUMN_CONTROL_FLAG_SORT,
    COLUMN_CONTROL_FLAG_DESC,
    COLUMN_CONTROL_FLAG_NULLS_FIRST,
    COLUMN_CONTROL_FLAG_GROUP,
    COLUMN_CONTROL_FLAG_HIDE,
    COLUMN_CONTROL_FLAG_PARTITION,
];

/// Comparison operators, longest first so that the longest match wins.
const COMPARISON_OPERATORS: [&str; 9] = [
    COMPARE_EQ_CASE_SENSITIVE,
    COMPARE_GTE,
    COMPARE_LTE,
    COMPARE_LIKE,
    COMPARE_REGEX,
    COMPARE_EQ,
    COMPARE_GT,
    COMPARE_LT,
    COMPARE_MATCH,
];

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Splits `src` into tokens covering every non-whitespace character, in order and without overlap.
pub(super) fn lex(src: &str) -> Vec<Token> {
    let mut lexer = Lexer {
        src,
        pos: 0,
        tokens: Vec::new(),
        context: Context::None,
        brackets: Vec::new(),
    };
    lexer.run();
    lexer.tokens
}

/// What the previous significant token implies about how to read a following word.
#[derive(Clone, Copy, PartialEq)]
enum Context {
    None,
    /// After `|`: a word is a scalar function name.
    ScalarPipe,
    /// After `%`: a word is an aggregate function name.
    AggregatePipe,
    /// After `->`: a word is an alias.
    AliasArrow,
    /// After a comparison operator: a word is a string literal, as is every word within a bracket
    /// that opens here (e.g. `title:~[color colour]`).
    ComparisonOperator,
    /// After `$` or `.`: a `*` is a glob.
    GlobPosition,
}

struct Lexer<'a> {
    src: &'a str,
    pos: usize,
    tokens: Vec<Token>,
    context: Context,
    /// For each open bracket, whether bare words within it are string literals.
    brackets: Vec<bool>,
}

impl Lexer<'_> {
    fn rest(&self) -> &str {
        &self.src[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn peek_nth(&self, n: usize) -> Option<char> {
        self.rest().chars().nth(n)
    }

    fn starts_with(&self, s: &str) -> bool {
        self.rest().starts_with(s)
    }

    /// The byte length of the leading run of `rest()` (from `offset` bytes in) whose chars satisfy
    /// `f`.
    fn run_len(&self, offset: usize, f: impl Fn(char) -> bool) -> usize {
        self.rest()[offset..]
            .char_indices()
            .find(|(_, c)| !f(*c))
            .map_or(self.rest().len() - offset, |(i, _)| i)
    }

    fn push(&mut self, kind: TokenKind, len: usize) {
        let span = self.pos..self.pos + len;
        self.pos += len;
        self.tokens.push(Token { kind, span });
    }

    /// Pushes a token and sets the context implied by it for the next significant token.
    fn emit(&mut self, kind: TokenKind, len: usize, context: Context) {
        self.push(kind, len);
        self.context = context;
    }

    fn run(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.pos += c.len_utf8();
            } else if self.starts_with(COMMENT_LINE) {
                // Comments are transparent to the context, e.g. `| /* note */ upper`.
                let len = self.run_len(0, |c| c != '\n');
                self.push(TokenKind::Comment, len);
            } else if self.starts_with(COMMENT_BLOCK_L) {
                let len = self.block_comment_len();
                self.push(TokenKind::Comment, len);
            } else {
                self.token(c);
            }
        }
    }

    /// The length of the (possibly nested, possibly unterminated) block comment at `pos`.
    fn block_comment_len(&self) -> usize {
        let rest = self.rest();
        let mut depth = 0;
        let mut i = 0;
        while i < rest.len() {
            if rest[i..].starts_with(COMMENT_BLOCK_L) {
                depth += 1;
                i += COMMENT_BLOCK_L.len();
            } else if rest[i..].starts_with(COMMENT_BLOCK_R) {
                depth -= 1;
                i += COMMENT_BLOCK_R.len();
                if depth == 0 {
                    return i;
                }
            } else {
                i += rest[i..].chars().next().map_or(1, char::len_utf8);
            }
        }
        rest.len()
    }

    /// Lexes one significant token beginning with `c`.
    fn token(&mut self, c: char) {
        use TokenKind as K;
        let context = self.context;
        match c {
            STRING_QUOTE_DOUBLE | STRING_QUOTE_SINGLE => {
                self.quoted(c, K::String);
                self.context = Context::None;
            }
            DB_IDENTIFIER_QUOTE => {
                // A backtick-quoted identifier is a column reference even where a bare word would
                // be a string literal.
                let kind = match self.word_kind(context, false) {
                    TokenKind::String => TokenKind::Column,
                    kind => kind,
                };
                self.quoted(c, kind);
                self.context = Context::None;
            }
            CONST_SIGIL => self.at_sign(),
            TABLE_SIGIL => {
                let len = 1 + self.name_len(1);
                self.emit(K::Table, len, Context::None);
            }
            _ if self.starts_with(PATH_TO_TABLE_WITH_ONE_PREFIX) => {
                let len = PATH_TO_TABLE_WITH_ONE_PREFIX.len();
                self.emit(K::PathSeparator, len, Context::None);
                if self.peek().is_some_and(|c| is_ident_start(c) || c == '`') {
                    let len = self.name_len(0);
                    self.push(K::Table, len);
                }
            }
            '0'..='9' => self.number(),
            _ if self.starts_with(SORT_EXPR_PREFIX) => {
                self.emit(K::Sort, SORT_EXPR_PREFIX.len(), Context::None)
            }
            COLUMN_CONTROL_FLAGS_PREFIX => {
                let flags = self.run_len(1, |c| FLAG_CHARS.contains(&c) || c.is_ascii_digit());
                let kind = if flags > 0 { K::Flag } else { K::Invalid };
                self.emit(kind, 1 + flags, Context::None);
            }
            COLUMN_SPEC_PREFIX => self.emit(K::ResultColumn, 1, Context::GlobPosition),
            _ if self.starts_with(COLUMN_ALIAS_PREFIX) => {
                self.emit(K::Operator, COLUMN_ALIAS_PREFIX.len(), Context::AliasArrow)
            }
            _ if self.starts_with(WINDOW_DEFINITION_PREFIX) => {
                self.emit(K::Pipe, WINDOW_DEFINITION_PREFIX.len(), Context::None)
            }
            COMPOSITION_PIPE_AGGREGATE => self.emit(K::Pipe, 1, Context::AggregatePipe),
            COMPOSITION_PIPE_SCALAR => self.emit(K::Pipe, 1, Context::ScalarPipe),
            ':' => {
                let op = COMPARISON_OPERATORS
                    .iter()
                    .find(|op| self.starts_with(op))
                    .expect("`:` alone is an operator");
                self.emit(K::ComparisonOperator, op.len(), Context::ComparisonOperator);
            }
            _ if self.starts_with(TRANSFORMATION_DELIMITER) => {
                self.emit(K::Pipeline, TRANSFORMATION_DELIMITER.len(), Context::None)
            }
            _ if self.starts_with(CASE_FALLBACK) => {
                self.emit(K::Case, CASE_FALLBACK.len(), Context::None)
            }
            CASE_VARIANT_SEPARATOR | CASE_BEGIN => self.emit(K::Case, 1, Context::None),
            _ if self.starts_with(HAS_QUANTITY_AT_LEAST_ONE)
                || self.starts_with(HAS_QUANTITY_ZERO) =>
            {
                // `++#comments` is a quantity prefix, but `a --1` is `a - -1`.
                let after = 2 + self.run_len(2, char::is_whitespace);
                let is_quantity = self.rest()[after..]
                    .starts_with(|c: char| c == TABLE_SIGIL || c == '>' || is_ident_start(c));
                if is_quantity {
                    self.emit(K::HasQuantity, 2, Context::None);
                } else {
                    self.emit(K::Operator, 1, Context::None);
                }
            }
            _ if self.starts_with(COMPARISON_RANGE_BOUND_SEPARATOR) => {
                self.emit(K::Operator, COMPARISON_RANGE_BOUND_SEPARATOR.len(), context)
            }
            _ if self.starts_with(FUNCTION_ARROW) => {
                self.emit(K::Definition, FUNCTION_ARROW.len(), Context::None)
            }
            DEFINITION_ASSIGN => self.emit(K::Definition, 1, Context::None),
            COLUMN_GLOB if context == Context::GlobPosition => self.emit(K::Glob, 1, Context::None),
            EXPR_PLUS
            | EXPR_MINUS
            | EXPR_TIMES
            | EXPR_DIVIDE
            | NEGATE
            | CONDITION_SET_OR_SHORTHAND
            | '<'
            | '>' => self.emit(K::Operator, 1, Context::None),
            PATH_SEPARATOR => self.emit(K::PathSeparator, 1, Context::GlobPosition),
            '(' | '[' | '{' => {
                let strings = context == Context::ComparisonOperator || self.in_strings();
                self.brackets.push(strings);
                self.emit(K::Bracket, 1, Context::None);
            }
            ')' | ']' | '}' => {
                self.brackets.pop();
                self.emit(K::Bracket, 1, Context::None);
            }
            _ if is_ident_start(c) => {
                let len = self.name_len(0);
                let after = self.rest()[len..].trim_start();
                let followed_by_path = after.starts_with(PATH_SEPARATOR)
                    && !after.starts_with(COMPARISON_RANGE_BOUND_SEPARATOR);
                let kind = self.word_kind(context, followed_by_path);
                self.emit(kind, len, Context::None);
            }
            _ => self.emit(K::Invalid, c.len_utf8(), Context::None),
        }
    }

    /// Whether bare words within the innermost open bracket are string literals.
    fn in_strings(&self) -> bool {
        self.brackets.last().copied().unwrap_or(false)
    }

    /// How a word reads given the preceding context.
    fn word_kind(&self, context: Context, followed_by_path: bool) -> TokenKind {
        match context {
            Context::ScalarPipe => TokenKind::Function,
            Context::AggregatePipe => TokenKind::AggregateFunction,
            Context::AliasArrow => TokenKind::Alias,
            _ if followed_by_path => TokenKind::Column,
            Context::ComparisonOperator => TokenKind::String,
            _ if self.in_strings() => TokenKind::String,
            _ => TokenKind::Column,
        }
    }

    /// The length of the bare word or backtick-quoted identifier starting `offset` bytes into
    /// `rest()`, or 0 if there is none.
    fn name_len(&self, offset: usize) -> usize {
        let rest = &self.rest()[offset..];
        if rest.starts_with(DB_IDENTIFIER_QUOTE) {
            quoted_len(rest, DB_IDENTIFIER_QUOTE)
        } else if rest.starts_with(is_ident_start) {
            self.run_len(offset, is_ident_continue)
        } else {
            0
        }
    }

    /// Lexes a quoted string or identifier at `pos`, splitting out escape sequences as
    /// [`TokenKind::StringEscape`] tokens (or [`TokenKind::Invalid`] ones, for an unrecognized
    /// escape). An unterminated quote runs to the end of the input.
    fn quoted(&mut self, quote: char, kind: TokenKind) {
        let end = self.pos + quoted_len(self.rest(), quote);
        let mut start = self.pos;
        for (escape, valid) in escapes(&self.src[self.pos..end], quote) {
            let escape = escape.start + self.pos..escape.end + self.pos;
            if escape.start > start {
                self.tokens.push(Token {
                    kind,
                    span: start..escape.start,
                });
            }
            start = escape.end;
            self.tokens.push(Token {
                kind: match valid {
                    true => TokenKind::StringEscape,
                    false => TokenKind::Invalid,
                },
                span: escape,
            });
        }
        if end > start {
            self.tokens.push(Token {
                kind,
                span: start..end,
            });
        }
        self.pos = end;
    }

    /// Lexes a token beginning with `@`: a function call or definition (`@@name`), a date, an
    /// annotation (`@{`), or a constant or variable.
    fn at_sign(&mut self) {
        use TokenKind as K;
        if self.starts_with(FUNCTION_SIGIL) {
            let len = FUNCTION_SIGIL.len() + self.name_len(FUNCTION_SIGIL.len());
            self.emit(K::Function, len, Context::None);
        } else if let Some(len) = self.date_len() {
            self.emit(K::Date, len, Context::None);
        } else if self.peek_nth(1) == Some(CONDITION_SET_AND_BRACE_L) {
            self.emit(K::Annotation, 1, Context::None);
        } else {
            let name_len = self.name_len(1);
            let name = &self.rest()[1..1 + name_len];
            let kind = if BUILTIN_CONSTANTS.contains(&name) {
                K::Constant
            } else {
                K::Variable
            };
            self.emit(kind, 1 + name_len, Context::None);
        }
    }

    /// The length of the `@YYYY-MM-DD` date at `pos`, if there is one.
    fn date_len(&self) -> Option<usize> {
        let bytes = self.rest().as_bytes();
        let shape = b"@dddd-dd-dd";
        let matches = bytes.len() >= shape.len()
            && shape.iter().zip(bytes).all(|(s, b)| match s {
                b'd' => b.is_ascii_digit(),
                _ => s == b,
            });
        matches.then_some(shape.len())
    }

    /// Lexes a number or a duration (a number with unit suffixes, e.g. `1y6m` or `90min`).
    fn number(&mut self) {
        let number_len = |lexer: &Self, offset: usize| {
            let int = lexer.run_len(offset, |c| c.is_ascii_digit());
            let rest = &lexer.rest()[offset + int..];
            let fraction =
                if rest.starts_with('.') && rest[1..].starts_with(|c: char| c.is_ascii_digit()) {
                    1 + lexer.run_len(offset + int + 1, |c| c.is_ascii_digit())
                } else {
                    0
                };
            int + fraction
        };
        let unit_len = |lexer: &Self, offset: usize| {
            let rest = &lexer.rest()[offset..];
            if rest.len() >= 3 && rest[..3].eq_ignore_ascii_case("min") {
                3
            } else if rest.starts_with(|c: char| "ymwdhsYMWDHS".contains(c)) {
                1
            } else {
                0
            }
        };
        let mut len = number_len(self, 0);
        let unit = unit_len(self, len);
        if unit == 0 {
            self.emit(TokenKind::Number, len, Context::None);
            return;
        }
        len += unit;
        // Further parts, e.g. the `6m` of `1y6m`.
        while self.rest()[len..].starts_with(|c: char| c.is_ascii_digit()) {
            let part = number_len(self, len);
            let unit = unit_len(self, len + part);
            if unit == 0 {
                break;
            }
            len += part + unit;
        }
        self.emit(TokenKind::Duration, len, Context::None);
    }
}

/// The length of the quoted string or identifier at the start of `s` (which begins with `quote`),
/// including both quotes. An unterminated quote runs to the end of `s`.
fn quoted_len(s: &str, quote: char) -> usize {
    let mut chars = s.char_indices().skip(1);
    while let Some((i, c)) = chars.next() {
        if c == STRING_ESCAPE_PREFIX {
            chars.next();
        } else if c == quote {
            return i + c.len_utf8();
        }
    }
    s.len()
}

/// The spans of the escape sequences within the quoted string or identifier `s`, each with whether
/// it is a valid escape (mirroring the parser's `escape`).
fn escapes(s: &str, quote: char) -> Vec<(Range<usize>, bool)> {
    let mut result = Vec::new();
    let mut chars = s.char_indices().skip(1).peekable();
    while let Some((i, c)) = chars.next() {
        if c == quote {
            break;
        }
        if c != STRING_ESCAPE_PREFIX {
            continue;
        }
        let mut end = i + 1;
        let mut valid = false;
        if let Some((j, escaped)) = chars.next() {
            end = j + escaped.len_utf8();
            valid = escaped == quote || "\\/bfnrt".contains(escaped);
            if escaped == 'u' {
                let mut digits = 0;
                while digits < 4 {
                    match chars.peek() {
                        Some(&(k, h)) if h.is_ascii_hexdigit() => {
                            end = k + 1;
                            digits += 1;
                            chars.next();
                        }
                        _ => break,
                    }
                }
                valid = digits == 4;
            }
        }
        result.push((i..end, valid));
    }
    result
}
