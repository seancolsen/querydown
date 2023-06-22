use chumsky::prelude::*;

use crate::ast::*;
use crate::parser::utils::*;
use crate::tokens::*;

pub fn date() -> impl Psr<Date> {
    just(CONST_SIGIL).ignore_then(
        usize_with_digit_count(4)
            .then_ignore(just('-'))
            .then(usize_with_digit_count(2))
            .then_ignore(just('-'))
            .then(usize_with_digit_count(2))
            .map(|((year, month), day)| Date { year, month, day })
            .labelled("date"),
    )
}
