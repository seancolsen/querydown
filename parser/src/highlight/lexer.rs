//! A total lexer: it classifies _any_ input — including input that doesn't parse — into tokens, so
//! that highlighting never breaks while the user is mid-edit.
//!
//! It knows only the lexical shape of the language, plus a few cheap look-behind heuristics (e.g. a
//! word after `|` is a function name). Where the parser succeeds, the semantic marks it records
//! override these guesses; see the [module documentation](super).

use std::ops::Range;

use super::{Token, TokenKind, BUILTIN_CONSTANTS};
use crate::flagged::{self, Piece, Prefix};
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
    lexer(src).tokens
}

/// The runs of literal text within the flagged strings of `src` (see [`TextRun`]).
pub(super) fn flagged_text_runs(src: &str) -> Vec<TextRun> {
    lexer(src).runs
}

fn lexer(src: &str) -> Lexer<'_> {
    let mut lexer = Lexer {
        src,
        pos: 0,
        tokens: Vec::new(),
        context: Context::None,
        brackets: Vec::new(),
        interpolations: Vec::new(),
        runs: Vec::new(),
    };
    lexer.run();
    lexer
}

/// A run of a flagged string that is literal text rather than code: from the start of the string, or
/// from the `}` that ends an interpolation, through the closing quote, or up to the `{` that begins
/// an interpolation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextRun {
    pub span: Range<usize>,
    /// Whether the text continues past the end of the run: the run ends at an interpolation, or at
    /// the end of the input before a closing quote.
    pub open: bool,
    /// The string's quote character.
    pub quote: char,
}

/// An interpolation within a flagged string that the lexer is inside of.
struct Interpolation {
    /// The string's prefix, with which to read the rest of the string after the interpolation.
    prefix: Prefix,
    /// The number of brackets open outside the interpolation.
    depth: usize,
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
    /// For each open bracket, whether bare words within it are string literals. An interpolation's
    /// `{` counts as a bracket within which bare words are not string literals.
    brackets: Vec<bool>,
    /// The interpolations the lexer is inside of, innermost last.
    interpolations: Vec<Interpolation>,
    runs: Vec<TextRun>,
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
            STRING_FLAGS_PREFIX => match flagged::prefix(self.rest()) {
                Some(prefix) => {
                    let start = self.pos;
                    self.push(K::StringFlags, prefix.flags_len);
                    self.flagged_string_body(prefix, start, prefix.quote.len_utf8());
                }
                None => self.emit(K::Invalid, c.len_utf8(), Context::None),
            },
            STRING_INTERPOLATION_R if self.at_interpolation_end() => {
                let interpolation = self.interpolations.pop().expect("within an interpolation");
                self.brackets.pop();
                let start = self.pos;
                self.push(K::StringInterpolation, c.len_utf8());
                self.flagged_string_body(interpolation.prefix, start, 0);
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
                let len = 1 + self.table_name_len(1);
                self.emit(K::Table, len, Context::None);
            }
            _ if self.starts_with(PATH_TO_TABLE_WITH_ONE_PREFIX) => {
                let len = PATH_TO_TABLE_WITH_ONE_PREFIX.len();
                self.emit(K::PathSeparator, len, Context::None);
                let len = self.table_name_len(0);
                if len > 0 {
                    self.push(K::Table, len);
                }
            }
            '0'..='9' => self.number(),
            // Before `\\`, which begins it.
            _ if self.starts_with(QUERY_ANNOTATION_PREFIX) => {
                self.emit(K::Annotation, QUERY_ANNOTATION_PREFIX.len(), Context::None)
            }
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
            ':' if self.rest()[COMPARE_MATCH.len()..].starts_with(REGEX_FLAGS_PREFIX) => {
                self.flagged_regex_operator()
            }
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
            // Before `++`, which begins it.
            _ if self.starts_with(UNION_DELIMITER) => {
                self.emit(K::Union, UNION_DELIMITER.len(), Context::None)
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
                // A mismatched bracket within an interpolation doesn't close the interpolation.
                let within = self.interpolations.last().map_or(0, |i| i.depth + 1);
                if self.brackets.len() > within {
                    self.brackets.pop();
                }
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

    /// Lexes a flagged regex operator, e.g. `:\cm~`, as its `:`, its flags, and its `~`. Any letters
    /// are lexed as flags (the parser reports unknown ones), and a missing `~` is tolerated.
    fn flagged_regex_operator(&mut self) {
        use TokenKind as K;
        self.push(K::ComparisonOperator, COMPARE_MATCH.len());
        let letters = self.run_len(1, |c| c.is_ascii_alphabetic());
        let kind = if letters > 0 { K::Flag } else { K::Invalid };
        self.push(kind, REGEX_FLAGS_PREFIX.len_utf8() + letters);
        // Take a doubled `~` too, as though the flags were given to `:~~`, which the parser
        // reports.
        let tilde_len = [
            &COMPARE_LIKE[COMPARE_MATCH.len()..],
            &COMPARE_REGEX[COMPARE_MATCH.len()..],
        ]
        .into_iter()
        .find(|tilde| self.starts_with(tilde))
        .map_or(0, str::len);
        if tilde_len > 0 {
            self.push(K::ComparisonOperator, tilde_len);
        }
        self.context = Context::ComparisonOperator;
    }

    /// Whether the next character closes the innermost interpolation, i.e. no bracket is open
    /// within it.
    fn at_interpolation_end(&self) -> bool {
        self.interpolations
            .last()
            .is_some_and(|i| self.brackets.len() == i.depth + 1)
    }

    /// Lexes the body of a flagged string with the given `prefix` from `pos`, given that the first
    /// `text` bytes there (e.g. its opening quote) are already known to be text. The text run being
    /// lexed began at `run_start`. This stops at the end of the string, or after the `{` that begins
    /// an interpolation, leaving the interpolation's code to be lexed as usual until its `}`.
    fn flagged_string_body(&mut self, prefix: Prefix, run_start: usize, mut text: usize) {
        use TokenKind as K;
        self.context = Context::None;
        loop {
            match flagged::piece(&self.rest()[text..], &prefix) {
                Piece::Text(len) => text += len,
                Piece::Close | Piece::End => {
                    let closed = self.rest()[text..].starts_with(prefix.quote);
                    if closed {
                        text += prefix.quote.len_utf8();
                    }
                    self.push_text(text);
                    self.runs.push(TextRun {
                        span: run_start..self.pos,
                        open: !closed,
                        quote: prefix.quote,
                    });
                    return;
                }
                Piece::Escape(len) => {
                    self.push_text(text);
                    text = 0;
                    self.push(K::StringEscape, len);
                }
                Piece::StrayBrace => {
                    self.push_text(text);
                    text = 0;
                    self.push(K::Invalid, STRING_INTERPOLATION_R.len_utf8());
                }
                Piece::Interpolation => {
                    self.push_text(text);
                    self.runs.push(TextRun {
                        span: run_start..self.pos,
                        open: true,
                        quote: prefix.quote,
                    });
                    self.push(K::StringInterpolation, STRING_INTERPOLATION_L.len_utf8());
                    self.interpolations.push(Interpolation {
                        prefix,
                        depth: self.brackets.len(),
                    });
                    self.brackets.push(false);
                    return;
                }
            }
        }
    }

    /// Pushes the given number of bytes of a flagged string's text, if any.
    fn push_text(&mut self, len: usize) {
        if len > 0 {
            self.push(TokenKind::String, len);
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
    /// The length of the table name at `offset` past `pos`, as written after a `#`: a name, or a
    /// schema's names and then the table's, each after a `/` (`/sales/orders`).
    fn table_name_len(&self, offset: usize) -> usize {
        let mut len = 0;
        while self.rest()[offset + len..].starts_with(TABLE_SCHEMA_SEPARATOR) {
            let name = self.name_len(offset + len + 1);
            if name == 0 {
                break;
            }
            len += 1 + name;
        }
        if len > 0 {
            len
        } else {
            self.name_len(offset)
        }
    }

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

    /// Lexes a quoted string or identifier at `pos`. An unterminated quote runs to the end of the
    /// input.
    fn quoted(&mut self, quote: char, kind: TokenKind) {
        let end = self.pos + quoted_len(self.rest(), quote);
        self.tokens.push(Token {
            kind,
            span: self.pos..end,
        });
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

    /// The length of the `@YYYY-MM-DD` date at `pos`, if there is one, including any time of day
    /// that follows it as in a timestamp (`T11:33`, `T11:33:00`, or `T11:33:00.25`).
    fn date_len(&self) -> Option<usize> {
        let bytes = self.rest().as_bytes();
        let has_shape = |offset: usize, shape: &[u8]| {
            bytes.len() >= offset + shape.len()
                && shape.iter().zip(&bytes[offset..]).all(|(s, b)| match s {
                    b'd' => b.is_ascii_digit(),
                    _ => s == b,
                })
        };
        let date = b"@dddd-dd-dd";
        if !has_shape(0, date) {
            return None;
        }
        let mut len = date.len();
        if has_shape(len, b"Tdd:dd") {
            len += 6;
            if has_shape(len, b":dd") {
                len += 3;
                if has_shape(len, b".d") {
                    len += 1 + self.run_len(len + 1, |c| c.is_ascii_digit());
                }
            }
        }
        Some(len)
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
            // Bytes, not `str`, so that a character after the number can't be split.
            if rest
                .as_bytes()
                .get(..3)
                .is_some_and(|b| b.eq_ignore_ascii_case(b"min"))
            {
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
    match s[quote.len_utf8()..].find(quote) {
        Some(i) => quote.len_utf8() + i + quote.len_utf8(),
        None => s.len(),
    }
}
