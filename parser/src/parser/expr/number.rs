use chumsky::{prelude::*, text::*};

use crate::parser::utils::*;

pub fn number() -> impl Psr<String> {
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
