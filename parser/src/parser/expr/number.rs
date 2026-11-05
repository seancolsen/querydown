use chumsky::{prelude::*, text::*};

use crate::highlight::TokenKind;
use crate::parser::utils::*;

pub fn number<'src>() -> impl Psr<'src, String> {
    mark(
        TokenKind::Number,
        just('-')
            .or_not()
            .then(int(10))
            .then(just('.').then(digits(10)).or_not())
            .to_slice()
            .map(|s: &str| s.to_string()),
    )
    .labelled(crate::labels::NUMBER)
}
