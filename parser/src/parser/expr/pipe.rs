use chumsky::{prelude::*, text::*};

use crate::ast::*;
use crate::parser::sorting::sort_flags;
use crate::parser::utils::*;
use crate::tokens::*;

fn agg_sort_expr<'src>(expr_parser: impl Psr<'src, Expr>) -> impl Psr<'src, SortExpr> {
    just(SORT_EXPR_PREFIX)
        .ignore_then(pad())
        .ignore_then(expr_parser)
        .then(pad().ignore_then(sort_flags()).or_not())
        .map(|(expr, flags)| {
            let (direction, nulls_sort) = flags.unwrap_or_default();
            SortExpr {
                expr,
                direction,
                nulls_sort,
            }
        })
}

pub fn pipe<'src>(
    arg0_expr: impl Psr<'src, Expr>,
    extra_args_expr: impl Psr<'src, Expr>,
) -> impl Psr<'src, Expr> {
    let scalar_args = just(COMPOSITION_ARGUMENT_BRACE_L)
        .ignore_then(
            extra_args_expr
                .clone()
                .padded_by(pad())
                .repeated()
                .collect::<Vec<Expr>>(),
        )
        .then_ignore(just(COMPOSITION_ARGUMENT_BRACE_R));

    let aggregate_order_by = just(COMPOSITION_ARGUMENT_BRACE_L)
        .ignore_then(
            agg_sort_expr(extra_args_expr)
                .padded_by(pad())
                .repeated()
                .collect::<Vec<SortExpr>>(),
        )
        .then_ignore(just(COMPOSITION_ARGUMENT_BRACE_R));

    let scalar_call = just(COMPOSITION_PIPE_SCALAR)
        .padded_by(pad())
        .ignore_then(ident())
        .then(scalar_args.or_not())
        .map(|(name, extra_args)| {
            (
                FunctionDimension::Scalar,
                name.to_string(),
                extra_args.unwrap_or_default(),
                vec![],
            )
        });

    let aggregate_call = just(COMPOSITION_PIPE_AGGREGATE)
        .padded_by(pad())
        .ignore_then(ident())
        .then(aggregate_order_by.or_not())
        .map(|(name, order_by)| {
            (
                FunctionDimension::Aggregate,
                name.to_string(),
                vec![],
                order_by.unwrap_or_default(),
            )
        });

    arg0_expr.foldl(
        choice((scalar_call, aggregate_call)).repeated(),
        |arg0, (dimension, name, extra_args, order_by)| {
            let args = std::iter::once(arg0).chain(extra_args).collect();
            Expr::Call(Call {
                name,
                dimension,
                syntax: CallSyntax::Piped,
                args,
                order_by,
            })
        },
    )
}
