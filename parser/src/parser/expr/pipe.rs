use chumsky::{prelude::*, text::*};

use crate::ast::*;
use crate::parser::utils::*;
use crate::tokens::*;

pub fn pipe<'src>(
    arg0_expr: impl Psr<'src, Expr>,
    extra_args_expr: impl Psr<'src, Expr>,
) -> impl Psr<'src, Expr> {
    let args = just(COMPOSITION_ARGUMENT_BRACE_L)
        .ignore_then(extra_args_expr.padded().repeated().collect::<Vec<Expr>>())
        .then_ignore(just(COMPOSITION_ARGUMENT_BRACE_R));

    let dimension = choice((
        just(COMPOSITION_PIPE_SCALAR).to(FunctionDimension::Scalar),
        just(COMPOSITION_PIPE_AGGREGATE).to(FunctionDimension::Aggregate),
    ));

    arg0_expr.foldl(
        dimension
            .padded()
            .then(ident())
            .then(args.or_not())
            .repeated(),
        |arg0, ((dimension, name), extra_args)| {
            let args = vec![arg0]
                .into_iter()
                .chain(extra_args.unwrap_or_default())
                .collect();
            Expr::Call(Call {
                name: name.to_string(),
                dimension,
                syntax: CallSyntax::Piped,
                args,
            })
        },
    )
}
