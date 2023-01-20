use chumsky::{prelude::*, text::*};

use crate::{syntax_tree::*, tokens::*};

use super::values::value;

pub fn expression<C>(
    condition_set: C,
) -> impl Parser<char, Expression, Error = Simple<char>> + Clone
where
    C: Parser<char, ConditionSet, Error = Simple<char>> + Clone + 'static,
{
    recursive(|e| {
        value(condition_set)
            .then(whitespace().ignore_then(composition(e)).repeated())
            .map(|(base, compositions)| Expression { base, compositions })
    })
}

fn composition<E>(expression: E) -> impl Parser<char, Composition, Error = Simple<char>> + Clone
where
    E: Parser<char, Expression, Error = Simple<char>> + Clone + 'static,
{
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
