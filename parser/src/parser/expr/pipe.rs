use chumsky::{prelude::*, text::*};

use crate::ast::*;
use crate::highlight::TokenKind;
use crate::labels;
use crate::parser::sorting::{sort_flags, value_sort_flags};
use crate::parser::utils::*;
use crate::tokens::*;

/// One sorting entry within an aggregate function's parentheses: either a `\\`-prefixed sorting
/// expression with optional `\d`/`\n` flags (`\\created_at%min \d`), or the column flags `\s`,
/// `\sd`, etc., which sort by the value being aggregated.
fn agg_sort_expr<'src>(expr_parser: impl Psr<'src, Expr>) -> impl Psr<'src, AggregateSortExpr> {
    let expr_entry = just(SORT_EXPR_PREFIX)
        .ignore_then(pad())
        .ignore_then(expr_parser)
        .then(pad().ignore_then(sort_flags()).or_not())
        .map(|(expr, flags)| (AggregateSortKey::Expr(expr), flags.unwrap_or_default()));
    let value_entry = value_sort_flags().map(|flags| (AggregateSortKey::Value, flags));
    choice((expr_entry, value_entry)).map_with(|(key, (direction, nulls_sort)), extra| {
        AggregateSortExpr {
            key,
            direction,
            nulls_sort,
            loc: loc(extra),
        }
    })
}

/// An aggregate's optional ORDER BY clause, e.g. the `(\\name)` in `labels.name%list(\\name)` or the
/// `(\sd)` in `labels.name%list(\sd)`. Built from a fresh copy of the extra-args expression parser
/// at each use site because the parser is consumed when constructed.
fn aggregate_order_by<'src>(
    args_expr: impl Psr<'src, Expr>,
) -> impl Psr<'src, Vec<AggregateSortExpr>> {
    just(COMPOSITION_ARGUMENT_BRACE_L)
        .ignore_then(
            agg_sort_expr(args_expr)
                .padded_by(pad())
                .repeated()
                .collect::<Vec<AggregateSortExpr>>(),
        )
        .then_ignore(just(COMPOSITION_ARGUMENT_BRACE_R))
}

/// One step in a pipe chain: a function applied to the value accumulated so far. Either a named
/// (built-in or user-defined) function or an inline anonymous function.
enum PipeStep {
    Call {
        dimension: FunctionDimension,
        name: String,
        name_loc: Loc,
        extra_args: Vec<Expr>,
        order_by: Vec<AggregateSortExpr>,
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
        .then_ignore(
            pad()
                .then(exactly(FUNCTION_ARROW).labelled(labels::FUNCTION_ARROW))
                .then(pad()),
        )
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

    let scalar_call = pad()
        .ignore_then(with_loc(
            just(COMPOSITION_PIPE_SCALAR).then(pad()).ignore_then(mark(
                TokenKind::Function,
                ident().labelled(labels::FUNCTION_NAME),
            )),
        ))
        .then(scalar_args.or_not())
        .map(|((name, name_loc), extra_args)| PipeStep::Call {
            dimension: FunctionDimension::Scalar,
            name: name.to_string(),
            name_loc,
            extra_args: extra_args.unwrap_or_default(),
            order_by: vec![],
        });

    // The aggregate function's name, located along with the `%` before it.
    let aggregate_name = || {
        with_loc(
            just(COMPOSITION_PIPE_AGGREGATE)
                .then(pad())
                .ignore_then(mark(
                    TokenKind::AggregateFunction,
                    ident().labelled(labels::AGGREGATE_FUNCTION_NAME),
                )),
        )
    };
    let aggregate_call = pad()
        .ignore_then(aggregate_name())
        .then(aggregate_order_by(extra_args_expr.clone()).or_not())
        .map(|((name, name_loc), order_by)| PipeStep::Call {
            dimension: FunctionDimension::Aggregate,
            name: name.to_string(),
            name_loc,
            extra_args: vec![],
            order_by: order_by.unwrap_or_default(),
        });

    // A standalone (leading) aggregate, e.g. `%count`, which has no `arg0` to its left. This is how
    // `count(*)` is written. It produces a [`Call`] with an empty `args` vector, distinguishing it
    // from a piped aggregate like `created_at%max` whose `args` contains the piped-in expression.
    let leading_aggregate = pad().ignore_then(located(
        aggregate_name()
            .then(aggregate_order_by(extra_args_expr).or_not())
            .map(|((name, name_loc), order_by)| {
                ExprKind::Call(Call {
                    name: name.to_string(),
                    name_loc,
                    dimension: FunctionDimension::Aggregate,
                    syntax: CallSyntax::Piped,
                    args: vec![],
                    order_by: order_by.unwrap_or_default(),
                })
            }),
    ));

    // `anonymous_call` is tried before `scalar_call` because both begin with `|`; they diverge only
    // after the pipe (`(` for an anonymous function versus an identifier for a named function).
    //
    // Each application is located from the start of the piped-in value to the end of the step.
    choice((leading_aggregate, arg0_expr)).foldl(
        with_loc(choice((anonymous_call, scalar_call, aggregate_call))).repeated(),
        |arg0, (step, step_loc)| {
            let loc = arg0.loc.to(step_loc);
            let kind = match step {
                PipeStep::Call {
                    dimension,
                    name,
                    name_loc,
                    extra_args,
                    order_by,
                } => {
                    let args = std::iter::once(arg0).chain(extra_args).collect();
                    ExprKind::Call(Call {
                        name,
                        name_loc,
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
                    let call = AnonymousFunctionCall { params, body, args };
                    ExprKind::AnonymousFunctionCall(Box::new(call))
                }
            };
            Expr::new(kind, loc)
        },
    )
}

#[cfg(test)]
mod tests {
    use super::super::expr;
    use super::*;

    fn parse(input: &str) -> Result<Expr, ()> {
        expr()
            .then_ignore(end())
            .parse(input)
            .into_result()
            .map_err(|_| ())
    }

    fn order_by(input: &str) -> Vec<AggregateSortExpr> {
        match parse(input) {
            Ok(Expr {
                kind: ExprKind::Call(call),
                ..
            }) => call.order_by,
            other => panic!("expected a call for {input}, got {other:?}"),
        }
    }

    fn col(name: &str) -> Expr {
        ExprKind::Path(vec![PathPartKind::Column(name.to_string()).into()]).into()
    }

    fn sort(
        key: AggregateSortKey,
        direction: SortDirection,
        nulls: NullsSort,
    ) -> AggregateSortExpr {
        AggregateSortExpr {
            loc: Loc::NONE,
            key,
            direction,
            nulls_sort: nulls,
        }
    }

    #[test]
    fn test_aggregate_value_sort_flags() {
        use AggregateSortKey::Value;
        use NullsSort::*;
        use SortDirection::*;
        assert_eq!(order_by(r"name%list(\s)"), vec![sort(Value, Asc, Last)]);
        assert_eq!(order_by(r"name%list(\sd)"), vec![sort(Value, Desc, Last)]);
        assert_eq!(order_by(r"name%list(\sn)"), vec![sort(Value, Asc, First)]);
        assert_eq!(order_by(r"name%list(\snd)"), vec![sort(Value, Desc, First)]);
        assert_eq!(
            order_by(r"name%list( \sdn )"),
            vec![sort(Value, Desc, First)]
        );
    }

    #[test]
    fn test_aggregate_sort_entries_mix() {
        use NullsSort::*;
        use SortDirection::*;
        // A `\\` entry's flags are distinguished from a following `\s` entry.
        assert_eq!(
            order_by(r"name%list(\\a \d \sd \\b)"),
            vec![
                sort(AggregateSortKey::Expr(col("a")), Desc, Last),
                sort(AggregateSortKey::Value, Desc, Last),
                sort(AggregateSortKey::Expr(col("b")), Asc, Last),
            ]
        );
        // An aggregate may serve as a sort key.
        let entries = order_by(r"name%list(\\%count \d)");
        assert_eq!(entries.len(), 1);
        assert!(matches!(
            &entries[0].key,
            AggregateSortKey::Expr(Expr { kind: ExprKind::Call(Call { name, .. }), .. }) if name == "count"
        ));
    }

    #[test]
    fn test_aggregate_value_sort_flags_require_s() {
        assert!(parse(r"name%list(\d)").is_err());
        assert!(parse(r"name%list(\x)").is_err());
    }
}
