use chumsky::{prelude::*, text::*};

use crate::{syntax_tree::*, tokens::*};

use super::{utils::QdParser, values::contextual_value};

pub fn expression(condition_set: impl QdParser<ConditionSet>) -> impl QdParser<Expression> {
    recursive(|e| {
        contextual_value(condition_set)
            .then(whitespace().ignore_then(composition(e)).repeated())
            .map(|(base, compositions)| Expression { base, compositions })
    })
}

fn composition(expression: impl QdParser<Expression>) -> impl QdParser<Composition> {
    let prefix = choice((
        just(COMPOSITION_PREFIX_SCALAR).to(FunctionDimension::Scalar),
        just(COMPOSITION_PREFIX_AGGREGATE).to(FunctionDimension::Aggregate),
    ));
    let brace_l = whitespace()
        .then(just(COMPOSITION_ARGUMENT_BRACE_L))
        .then(whitespace());
    let brace_r = whitespace()
        .then(just(COMPOSITION_ARGUMENT_BRACE_R))
        .then(whitespace());
    prefix
        .then_ignore(whitespace())
        .then(ident())
        .map(|(dimension, name)| Function { name, dimension })
        .then(expression.delimited_by(brace_l, brace_r).or_not())
        .map(|(function, argument)| Composition { function, argument })
}
