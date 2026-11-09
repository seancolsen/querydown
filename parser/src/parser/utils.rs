use chumsky::{
    input::{Checkpoint, Cursor},
    inspector::Inspector,
    prelude::*,
    text::*,
};

use crate::highlight::{Token, TokenKind};
use crate::tokens::*;

/// The parser state: the highlighting marks recorded so far (see [`mark`]).
///
/// Chumsky notifies this [`Inspector`] whenever the parser saves its position and whenever it
/// rewinds to a saved position, i.e. backtracks. Rewinding truncates the marks to those recorded
/// before the save, discarding the marks of the abandoned alternative. (This is Chumsky's
/// `TruncateState`, which we can't use directly because it requires `Token: Default`.)
#[derive(Debug, Default)]
pub struct Marks(pub Vec<Token>);

impl<'src, I: Input<'src>> Inspector<'src, I> for Marks {
    type Checkpoint = usize;

    fn on_token(&mut self, _: &I::Token) {}

    fn on_save<'parse>(&self, _: &Cursor<'src, 'parse, I>) -> usize {
        self.0.len()
    }

    fn on_rewind<'parse>(&mut self, checkpoint: &Checkpoint<'src, 'parse, I, usize>) {
        self.0.truncate(*checkpoint.inspector());
    }
}

/// The "extra" type parameter shared by all of our parsers: rich errors, plus the [`Marks`] state.
pub type Extra<'src> = extra::Full<Rich<'src, char>, Marks, ()>;

/// `Psr` is an abbreviation for "Parser". This is abbreviated because it is used in many places,
/// and we don't want it to conflict with Chumsky's `Parser` trait.
///
/// This is a utility type to reduce code duplication in types. It would be easier to write as
/// follows:
///
/// ```rs
/// pub type Psr<'src, T> = Parser<'src, &'src str, T, Extra<'src>> + Clone;
/// ```
///
/// However, we can't do that without [trait aliases][1].
///
/// [1]: https://github.com/rust-lang/rust/issues/41517
pub trait Psr<'src, T>: Parser<'src, &'src str, T, Extra<'src>> + Clone {}
impl<'src, S, T> Psr<'src, T> for S where S: Parser<'src, &'src str, T, Extra<'src>> + Clone {}

/// Records the span matched by `parser` as a `kind` token for syntax highlighting (see the
/// [`highlight`](crate::highlight) module). This is otherwise transparent: it yields `parser`'s
/// output and fails wherever `parser` fails.
///
/// Mark the narrowest parser that covers the token — e.g. the name in `|upper`, not the pipe too —
/// since a mark recolors every non-comment character within its span.
///
/// This uses `try_map_with` rather than `map_with` because Chumsky skips `map_with` closures for
/// parsers whose output is discarded (e.g. inside `ignore_then` or `not`), whereas `try_map_with`
/// always runs.
pub fn mark<'src, T>(kind: TokenKind, parser: impl Psr<'src, T>) -> impl Psr<'src, T> {
    parser.try_map_with(move |output, extra| {
        let span: SimpleSpan = extra.span();
        extra.state().0.push(Token {
            kind,
            span: span.into_range(),
        });
        Ok(output)
    })
}

pub fn exactly<'src>(s: &str) -> impl Psr<'src, String> {
    just(s.to_string())
}

/// Matches a single code comment, discarding it.
///
/// Two forms are supported:
/// - a line comment, `// ...`, running to the end of the line, and
/// - a block comment, `/* ... */`, which may be nested.
///
/// Comments are not propagated into the AST.
pub fn comment<'src>() -> impl Psr<'src, ()> {
    let line = just(COMMENT_LINE)
        .then(any().filter(|c: &char| *c != '\n').repeated())
        .ignored();

    let block = recursive(|block| {
        // The body of a block comment is any mix of nested block comments and individual
        // characters that don't form a delimiter. Nesting is handled by trying `block` first.
        let content = choice((
            block,
            any()
                .and_is(just(COMMENT_BLOCK_L).not())
                .and_is(just(COMMENT_BLOCK_R).not())
                .ignored(),
        ));
        content
            .repeated()
            .delimited_by(just(COMMENT_BLOCK_L), just(COMMENT_BLOCK_R))
    });

    choice((line, block))
}

/// Matches any run of insignificant input — whitespace and/or comments — discarding it. Use this
/// instead of Chumsky's `whitespace()` so that comments are permitted anywhere whitespace is.
///
/// The result is `.boxed()` to type-erase it. `pad()` is used at dozens of call sites, and its
/// underlying combinator type (which embeds the recursive comment parser) is large; without boxing,
/// inlining that type into every enclosing parser balloons rustc's monomorphization memory enough
/// to OOM the build.
pub fn pad<'src>() -> impl Psr<'src, ()> {
    choice((
        any().filter(|c: &char| c.is_whitespace()).ignored(),
        comment(),
    ))
    .repeated()
    .ignored()
    .labelled(crate::labels::WHITESPACE)
    .boxed()
}

pub fn usize_with_digit_count<'src>(digit_count: usize) -> impl Psr<'src, u32> {
    any()
        .filter(char::is_ascii_digit)
        .repeated()
        .exactly(digit_count)
        .collect::<String>()
        .from_str()
        .unwrapped()
}

pub fn positive_float<'src>() -> impl Psr<'src, f64> {
    use std::str::FromStr;
    int(10)
        .then(just('.').then(digits(10)).or_not())
        .to_slice()
        .try_map(|v: &str, span| f64::from_str(v).map_err(|_| Rich::custom(span, "invalid float")))
}

pub fn db_identifier<'src>() -> impl Psr<'src, String> {
    ident()
        .map(|s: &str| s.to_string())
        .or(quoted(DB_IDENTIFIER_QUOTE))
        .labelled(crate::labels::NAME)
}

pub fn quoted<'src>(quote: char) -> impl Psr<'src, String> {
    just(quote)
        .ignore_then(
            any()
                .filter(move |c: &char| *c != STRING_ESCAPE_PREFIX && *c != quote)
                .or(escape(quote))
                .repeated()
                .collect::<String>(),
        )
        .then_ignore(just(quote))
}

pub fn escape<'src>(quote: char) -> impl Psr<'src, char> {
    just(STRING_ESCAPE_PREFIX).ignore_then(
        just(STRING_ESCAPE_PREFIX)
            .or(just('/'))
            .or(just(quote))
            .or(just('b').to('\x08'))
            .or(just('f').to('\x0C'))
            .or(just('n').to('\n'))
            .or(just('r').to('\r'))
            .or(just('t').to('\t'))
            .or(just('u').ignore_then(
                any()
                    .filter(|c: &char| c.is_ascii_hexdigit())
                    .repeated()
                    .exactly(4)
                    .collect::<String>()
                    .labelled(crate::labels::UNICODE_ESCAPE_DIGITS)
                    .validate(|digits, e, emitter| {
                        char::from_u32(u32::from_str_radix(&digits, 16).unwrap()).unwrap_or_else(
                            || {
                                emitter.emit(Rich::custom(
                                    e.span(),
                                    format!("`\\u{digits}` is not a valid unicode character."),
                                ));
                                '\u{FFFD}' // unicode replacement character
                            },
                        )
                    }),
            ))
            .labelled(crate::labels::ESCAPE_CHARACTER),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_pad(s: &str) -> Result<(), ()> {
        pad()
            .then_ignore(end())
            .parse(s)
            .into_result()
            .map_err(|_| ())
    }

    #[test]
    fn test_pad_consumes_whitespace_and_comments() {
        assert_eq!(parse_pad(""), Ok(()));
        assert_eq!(parse_pad("   \n\t "), Ok(()));
        assert_eq!(parse_pad("// line comment"), Ok(()));
        assert_eq!(parse_pad("// line\n  // another\n"), Ok(()));
        assert_eq!(parse_pad("/* block */"), Ok(()));
        assert_eq!(parse_pad("  /* a */ // b\n /* c */ "), Ok(()));
    }

    #[test]
    fn test_block_comments_nest() {
        assert_eq!(parse_pad("/* outer /* inner */ outer */"), Ok(()));
        assert_eq!(parse_pad("/* a /* b /* c */ b */ a */"), Ok(()));
        // An unterminated block comment, including one whose nesting is unbalanced, fails.
        assert_eq!(parse_pad("/* unterminated"), Err(()));
        assert_eq!(parse_pad("/* /* only one close */"), Err(()));
    }
}
