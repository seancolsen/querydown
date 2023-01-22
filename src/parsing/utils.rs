use chumsky::prelude::*;

/// This is a utility type to reduce code duplication in types. It would be easier to write as
/// follows:
///
/// ```rs
/// pub type LqlParser<T> = Parser<char, T, Error = Simple<char>> + Clone + 'static;
/// ```
///
/// However, we can't do that without [trait aliases][1].
///
/// [1]: https://github.com/rust-lang/rust/issues/41517
pub trait LqlParser<T>: Parser<char, T, Error = Simple<char>> + Clone + 'static {}
impl<S, T> LqlParser<T> for S where S: Parser<char, T, Error = Simple<char>> + Clone + 'static {}

pub fn exactly(s: &str) -> impl LqlParser<String> {
    just(s.chars().collect::<Vec<char>>()).collect::<String>()
}
