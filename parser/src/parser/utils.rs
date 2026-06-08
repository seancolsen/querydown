use chumsky::{prelude::*, text::*};

use crate::tokens::*;

/// `Psr` is an abbreviation for "Parser". This is abbreviated because it is used in many places,
/// and we don't want it to conflict with Chumsky's `Parser` trait.
///
/// This is a utility type to reduce code duplication in types. It would be easier to write as
/// follows:
///
/// ```rs
/// pub type Psr<'src, T> = Parser<'src, &'src str, T, extra::Err<Rich<'src, char>>> + Clone;
/// ```
///
/// However, we can't do that without [trait aliases][1].
///
/// [1]: https://github.com/rust-lang/rust/issues/41517
pub trait Psr<'src, T>: Parser<'src, &'src str, T, extra::Err<Rich<'src, char>>> + Clone {}
impl<'src, S, T> Psr<'src, T> for S where
    S: Parser<'src, &'src str, T, extra::Err<Rich<'src, char>>> + Clone
{
}

pub fn exactly<'src>(s: &str) -> impl Psr<'src, String> {
    just(s.to_string())
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
                    .validate(|digits, e, emitter| {
                        char::from_u32(u32::from_str_radix(&digits, 16).unwrap()).unwrap_or_else(
                            || {
                                emitter.emit(Rich::custom(e.span(), "invalid unicode character"));
                                '\u{FFFD}' // unicode replacement character
                            },
                        )
                    }),
            )),
    )
}
