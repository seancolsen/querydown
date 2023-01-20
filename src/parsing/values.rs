use chumsky::{prelude::*, text::*};

use crate::syntax_tree::*;
use crate::tokens::*;

use super::paths::path;
use super::utils::*;

pub fn value<C>(condition_set: C) -> impl Parser<char, Value, Error = Simple<char>>
where
    C: Parser<char, ConditionSet, Error = Simple<char>> + Clone,
{
    choice::<_, Simple<char>>((
        exactly(LITERAL_NOW).to(Value::Now),
        exactly(LITERAL_INFINITY).to(Value::Infinity),
        exactly(LITERAL_TRUE).to(Value::True),
        exactly(LITERAL_FALSE).to(Value::False),
        exactly(LITERAL_NULL).to(Value::Null),
        just(SLOT).to(Value::Slot),
        number().map(Value::Number),
        choice((quoted(STRING_QUOTE_SINGLE), quoted(STRING_QUOTE_DOUBLE))).map(Value::String),
        path(condition_set).map(Value::Path),
    ))
}

pub fn db_identifier() -> impl Parser<char, String, Error = Simple<char>> {
    ident().or(quoted(DB_IDENTIFIER_QUOTE))
}

#[test]
fn test_db_identifier() {
    assert_eq!(db_identifier().parse("foo"), Ok("foo".to_string()));
    assert_eq!(
        db_identifier().parse("` !f \\`o'\"o`"),
        Ok(" !f `o'\"o".to_string())
    );
}

fn escape(quote: char) -> impl Parser<char, char, Error = Simple<char>> {
    just(ESCAPE_PREFIX).ignore_then(
        just(ESCAPE_PREFIX)
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

fn quoted(quote: char) -> impl Parser<char, String, Error = Simple<char>> {
    just(quote)
        .ignore_then(
            filter(move |c| *c != ESCAPE_PREFIX && *c != quote)
                .or(escape(quote))
                .repeated(),
        )
        .then_ignore(just(quote))
        .collect::<String>()
}

fn number() -> impl Parser<char, String, Error = Simple<char>> {
    just('-')
        .or_not()
        .chain::<char, _, _>(int(10))
        .chain::<char, _, _>(
            just('.')
                .chain(digits::<char, Simple<char>>(10))
                .or_not()
                .flatten(),
        )
        .collect::<String>()
        .labelled("number")
}
