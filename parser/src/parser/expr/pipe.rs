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

/// An aggregate's optional ORDER BY clause, e.g. the `(\name)` in `labels.name%list(\name)`. Built
/// from a fresh copy of the extra-args expression parser at each use site because the parser is
/// consumed when constructed.
fn aggregate_order_by<'src>(args_expr: impl Psr<'src, Expr>) -> impl Psr<'src, Vec<SortExpr>> {
    just(COMPOSITION_ARGUMENT_BRACE_L)
        .ignore_then(
            agg_sort_expr(args_expr)
                .padded_by(pad())
                .repeated()
                .collect::<Vec<SortExpr>>(),
        )
        .then_ignore(just(COMPOSITION_ARGUMENT_BRACE_R))
}

/// One step in a pipe chain: a function applied to the value accumulated so far. Either a named
/// (built-in or user-defined) function or an inline anonymous function.
enum PipeStep {
    Call {
        dimension: FunctionDimension,
        name: String,
        extra_args: Vec<Expr>,
        order_by: Vec<SortExpr>,
    },
    AnonymousFn {
        params: Vec<String>,
        body: FunctionBody,
        extra_args: Vec<Expr>,
    },
}

/// Parses an inline anonymous function, e.g. `(@d => ? @d:<0 ~ "a" ~~ "b")`. The body — like a
/// user-defined function's body — is zero or more local assignments followed by a single result
/// expression, and may reference the parameters and earlier assignments.
fn anonymous_function<'src>(
    body_expr: impl Psr<'src, Expr>,
) -> impl Psr<'src, (Vec<String>, FunctionBody)> {
    let params = super::variable()
        .separated_by(pad())
        .at_least(1)
        .collect::<Vec<String>>();
    // A local assignment, distinguished from the result expression by the `=` that follows its name.
    // The `=` must not begin a `=>` arrow (defensive; arrows only appear in the parameter list).
    let assignment = super::variable()
        .then_ignore(pad())
        .then_ignore(just(DEFINITION_ASSIGN).then_ignore(just('>').not()))
        .then_ignore(pad())
        .then(body_expr.clone())
        .map(|(name, expr)| Assignment { name, expr });
    let function_body = assignment
        .then_ignore(pad())
        .repeated()
        .collect::<Vec<Assignment>>()
        .then(body_expr)
        .map(|(assignments, expr)| FunctionBody { assignments, expr });
    params
        .then_ignore(pad().then(exactly(FUNCTION_ARROW)).then(pad()))
        .then(function_body)
        .delimited_by(
            just(EXPR_PAREN_L).then(pad()),
            pad().then(just(EXPR_PAREN_R)),
        )
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

    // An inline anonymous function applied via a pipe, e.g. `value|(@d => @d:<0)`. The piped-in
    // value becomes the first argument; any parenthesized values that follow supply the rest.
    let anonymous_call = just(COMPOSITION_PIPE_SCALAR)
        .padded_by(pad())
        .ignore_then(anonymous_function(extra_args_expr.clone()))
        .then(scalar_args.clone().or_not())
        .map(|((params, body), extra_args)| PipeStep::AnonymousFn {
            params,
            body,
            extra_args: extra_args.unwrap_or_default(),
        });

    let scalar_call = just(COMPOSITION_PIPE_SCALAR)
        .padded_by(pad())
        .ignore_then(ident())
        .then(scalar_args.or_not())
        .map(|(name, extra_args)| PipeStep::Call {
            dimension: FunctionDimension::Scalar,
            name: name.to_string(),
            extra_args: extra_args.unwrap_or_default(),
            order_by: vec![],
        });

    let aggregate_call = just(COMPOSITION_PIPE_AGGREGATE)
        .padded_by(pad())
        .ignore_then(ident())
        .then(aggregate_order_by(extra_args_expr.clone()).or_not())
        .map(|(name, order_by)| PipeStep::Call {
            dimension: FunctionDimension::Aggregate,
            name: name.to_string(),
            extra_args: vec![],
            order_by: order_by.unwrap_or_default(),
        });

    // A standalone (leading) aggregate, e.g. `%count`, which has no `arg0` to its left. This is how
    // `count(*)` is written. It produces a [`Call`] with an empty `args` vector, distinguishing it
    // from a piped aggregate like `created_at%max` whose `args` contains the piped-in expression.
    let leading_aggregate = just(COMPOSITION_PIPE_AGGREGATE)
        .padded_by(pad())
        .ignore_then(ident())
        .then(aggregate_order_by(extra_args_expr).or_not())
        .map(|(name, order_by)| {
            Expr::Call(Call {
                name: name.to_string(),
                dimension: FunctionDimension::Aggregate,
                syntax: CallSyntax::Piped,
                args: vec![],
                order_by: order_by.unwrap_or_default(),
            })
        });

    // `anonymous_call` is tried before `scalar_call` because both begin with `|`; they diverge only
    // after the pipe (`(` for an anonymous function versus an identifier for a named function).
    choice((leading_aggregate, arg0_expr)).foldl(
        choice((anonymous_call, scalar_call, aggregate_call)).repeated(),
        |arg0, step| match step {
            PipeStep::Call {
                dimension,
                name,
                extra_args,
                order_by,
            } => {
                let args = std::iter::once(arg0).chain(extra_args).collect();
                Expr::Call(Call {
                    name,
                    dimension,
                    syntax: CallSyntax::Piped,
                    args,
                    order_by,
                })
            }
            PipeStep::AnonymousFn {
                params,
                body,
                extra_args,
            } => {
                let args = std::iter::once(arg0).chain(extra_args).collect();
                Expr::AnonymousFunctionCall(Box::new(AnonymousFunctionCall { params, body, args }))
            }
        },
    )
}
