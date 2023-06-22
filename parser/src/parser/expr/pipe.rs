use chumsky::{prelude::*, text::*};

use crate::ast::*;
use crate::parser::utils::*;
use crate::tokens::*;

pub fn pipe(arg0_expr: impl Psr<Expr>, extra_args_expr: impl Psr<Expr>) -> impl Psr<Expr> {
    let args = just(COMPOSITION_ARGUMENT_BRACE_L)
        .ignore_then(extra_args_expr.padded().repeated())
        .then_ignore(just(COMPOSITION_ARGUMENT_BRACE_R));

    let dimension = choice((
        just(COMPOSITION_PIPE_SCALAR).to(FunctionDimension::Scalar),
        just(COMPOSITION_PIPE_AGGREGATE).to(FunctionDimension::Aggregate),
    ));

    arg0_expr
        .then(
            dimension
                .padded()
                .then(ident())
                .then(args.or_not())
                .repeated(),
        )
        .foldl(|arg0, ((dimension, name), extra_args)| {
            let args = vec![arg0]
                .into_iter()
                .chain(extra_args.unwrap_or_default().into_iter())
                .collect();
            Expr::Call(Call {
                name,
                dimension,
                syntax: CallSyntax::Piped,
                args,
            })
        })
}
