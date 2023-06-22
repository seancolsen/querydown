use chumsky::{prelude::*, text::*};

use crate::tokens::*;

/// `Psr` is an abbreviation for "Parser". This is abbreviated because it is used in many places,
/// and we don't want it to conflict with Chumsky's `Parser` trait.
///
/// This is a utility type to reduce code duplication in types. It would be easier to write as
/// follows:
///
/// ```rs
/// pub type Psr<T> = Parser<char, T, Error = Simple<char>> + Clone + 'static;
/// ```
///
/// However, we can't do that without [trait aliases][1].
///
/// [1]: https://github.com/rust-lang/rust/issues/41517
pub trait Psr<T>: Parser<char, T, Error = Simple<char>> + Clone + 'static {}
impl<S, T> Psr<T> for S where S: Parser<char, T, Error = Simple<char>> + Clone + 'static {}

pub fn exactly(s: &str) -> impl Psr<String> {
    just(s.chars().collect::<Vec<char>>()).collect::<String>()
}

pub fn usize_with_digit_count(digit_count: usize) -> impl Psr<u32> {
    filter(char::is_ascii_digit)
        .repeated()
        .exactly(digit_count)
        .collect::<String>()
        .from_str()
        .unwrapped()
}

pub fn positive_float() -> impl Psr<f64> {
    use std::str::FromStr;
    int(10)
        .chain::<char, _, _>(just('.').chain(digits(10)).or_not().flatten())
        .collect::<String>()
        .try_map(|v, span| f64::from_str(&v).map_err(|_| Simple::custom(span, "invalid float")))
}

pub fn db_identifier() -> impl Psr<String> {
    ident().or(quoted(DB_IDENTIFIER_QUOTE))
}

pub fn quoted(quote: char) -> impl Psr<String> {
    just(quote)
        .ignore_then(
            filter(move |c| *c != STRING_ESCAPE_PREFIX && *c != quote)
                .or(escape(quote))
                .repeated(),
        )
        .then_ignore(just(quote))
        .collect::<String>()
}

pub fn escape(quote: char) -> impl Psr<char> {
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
                filter(|c: &char| c.is_digit(16))
                    .repeated()
                    .exactly(4)
                    .collect::<String>()
                    .validate(|digits, span, emit| {
                        char::from_u32(u32::from_str_radix(&digits, 16).unwrap()).unwrap_or_else(
                            || {
                                emit(Simple::custom(span, "invalid unicode character"));
                                '\u{FFFD}' // unicode replacement character
                            },
                        )
                    }),
            )),
    )
}
