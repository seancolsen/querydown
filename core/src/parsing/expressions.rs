use chumsky::{prelude::*, text::*};

use super::utils::QdParser;
use crate::parsing::values::value;
use crate::{syntax_tree::*, tokens::*};

pub fn expression(condition_set: impl QdParser<ConditionSet>) -> impl QdParser<Expression> {
    recursive(|e| {
        value(condition_set)
            .then(whitespace().ignore_then(composition(e)).repeated())
            .map(|(base, compositions)| Expression { base, compositions })
    })
}

fn composition(expression: impl QdParser<Expression>) -> impl QdParser<Composition> {
    let prefix = choice((
        just(COMPOSITION_PIPE_SCALAR).to(FunctionDimension::Scalar),
        just(COMPOSITION_PIPE_AGGREGATE).to(FunctionDimension::Aggregate),
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
