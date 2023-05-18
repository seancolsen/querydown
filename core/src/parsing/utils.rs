use chumsky::{prelude::*, text::*};

/// This is a utility type to reduce code duplication in types. It would be easier to write as
/// follows:
///
/// ```rs
/// pub type QdParser<T> = Parser<char, T, Error = Simple<char>> + Clone + 'static;
/// ```
///
/// However, we can't do that without [trait aliases][1].
///
/// [1]: https://github.com/rust-lang/rust/issues/41517
pub trait QdParser<T>: Parser<char, T, Error = Simple<char>> + Clone + 'static {}
impl<S, T> QdParser<T> for S where S: Parser<char, T, Error = Simple<char>> + Clone + 'static {}

pub fn exactly(s: &str) -> impl QdParser<String> {
    just(s.chars().collect::<Vec<char>>()).collect::<String>()
}

pub fn usize_with_digit_count(digit_count: usize) -> impl QdParser<u32> {
    filter(char::is_ascii_digit)
        .repeated()
        .exactly(digit_count)
        .collect::<String>()
        .from_str()
        .unwrapped()
}

pub fn positive_float() -> impl QdParser<f64> {
    use std::str::FromStr;
    int(10)
        .chain::<char, _, _>(just('.').chain(digits(10)).or_not().flatten())
        .collect::<String>()
        .try_map(|v, span| f64::from_str(&v).map_err(|_| Simple::custom(span, "invalid float")))
}
