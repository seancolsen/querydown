use chumsky::{prelude::*, text::*};

use crate::{ast::*, tokens::*};

use super::values::value;

pub fn expression<C>(
    condition_set: C,
) -> impl Parser<char, Expression, Error = Simple<char>> + Clone
where
    C: Parser<char, ConditionSet, Error = Simple<char>> + Clone + 'static,
{
    recursive(|e| {
        let composition = whitespace()
            .then(just(COMPOSITION_PREFIX))
            .then(whitespace())
            .ignore_then(
                just(AGGREGATE_FUNCTION_FLAG)
                    .or_not()
                    .map(|x| x.is_some())
                    .then(ident().map(|v| v)),
            )
            .then(
                e.delimited_by(
                    whitespace()
                        .then(just(COMPOSITION_ARGUMENT_BRACE_L))
                        .then(whitespace()),
                    whitespace()
                        .then(just(COMPOSITION_ARGUMENT_BRACE_R))
                        .then(whitespace()),
                )
                .or_not(),
            )
            .map(|((is_aggregate, function), argument)| Composition {
                function,
                argument,
                is_aggregate,
            });

        value(condition_set)
            .then(composition.repeated())
            .map(|(base, compositions)| Expression { base, compositions })
    })
}
