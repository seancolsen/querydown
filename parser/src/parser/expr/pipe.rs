use chumsky::{
    input::{Emitter, MapExtra},
    prelude::*,
    text::*,
};

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

/// One entry within an aggregate function's parentheses.
enum AggregateEntry {
    /// A value argument, e.g. the `0.5` in `x%percentile(0.5)`.
    Arg(Expr),
    /// A sorting entry, e.g. the `\\name` in `labels.name%list(\\name)`.
    Sort(AggregateSortExpr),
}

/// What an aggregate function's parentheses hold: its value arguments (which follow the aggregated
/// value in [`Call::args`]) and its sorting entries ([`Call::order_by`]).
type AggregateParens = (Vec<Expr>, Vec<AggregateSortExpr>);

/// Separates the entries in an aggregate function's parentheses into value arguments and sorting
/// entries, each kept in the order written.
fn split_aggregate_entries(entries: Vec<AggregateEntry>) -> AggregateParens {
    let mut args = Vec::new();
    let mut order_by = Vec::new();
    for entry in entries {
        match entry {
            AggregateEntry::Arg(arg) => args.push(arg),
            AggregateEntry::Sort(sort) => order_by.push(sort),
        }
    }
    (args, order_by)
}

/// The parentheses after an aggregate function's name, holding value arguments and sorting entries
/// in any order, e.g. the `(\\name)` in `labels.name%list(\\name)`, the `(\sd)` in
/// `labels.name%list(\sd)`, or the `(0.5)` in `x%percentile(0.5)`. A sorting entry begins with a
/// `\`, which no value can. The compiler decides which an aggregate function accepts. `value` parses
/// a value argument and `sort_expr` the expression of a `\\` sorting entry. Built from fresh copies
/// of the extra-args expression parser at each use site because the parser is consumed when
/// constructed.
fn aggregate_parens<'src>(
    sort_expr: impl Psr<'src, Expr> + 'src,
    value: impl Psr<'src, Expr> + 'src,
) -> impl Psr<'src, AggregateParens> {
    let entry = choice((
        agg_sort_expr(sort_expr).map(AggregateEntry::Sort),
        value.map(AggregateEntry::Arg),
    ))
    .boxed();
    just(COMPOSITION_ARGUMENT_BRACE_L)
        .ignore_then(
            entry
                .padded_by(pad())
                .repeated()
                .collect::<Vec<AggregateEntry>>(),
        )
        .then_ignore(just(COMPOSITION_ARGUMENT_BRACE_R))
        .map(split_aggregate_entries)
        .boxed()
}

/// Reports a value argument given to an aggregate function that's used on its own (`%count(5)`),
/// which has no aggregated value for the argument to go with.
fn leading_aggregate_arg<'src>(
    arg: Expr,
    extra: &mut MapExtra<'src, '_, &'src str, Extra<'src>>,
    emitter: &mut Emitter<Rich<'src, char>>,
) -> Expr {
    let message = "An aggregate function used on its own, without a value before it (as in \
                   `%count`), takes no arguments. To aggregate a value, write it before the \
                   function, as in `x%percentile(0.5)`";
    emitter.emit(Rich::custom(extra.span(), message));
    arg
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

/// Parses a function's parameters, its arrow, and its body, e.g. `@d => ? @d:<0 ~ "a" ~~ "b"`. The
/// body — like a user-defined function's body — is zero or more local assignments followed by a
/// single result expression, and may reference the parameters and earlier assignments.
fn function_literal<'src>(
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
}

/// Parses an inline anonymous function, e.g. `(@d => ? @d:<0 ~ "a" ~~ "b")`: a function literal in
/// parentheses.
fn anonymous_function<'src>(
    body_expr: impl Psr<'src, Expr>,
) -> impl Psr<'src, (Vec<String>, FunctionBody)> {
    function_literal(body_expr).delimited_by(
        just(EXPR_PAREN_L).then(pad()),
        pad().then(just(EXPR_PAREN_R)),
    )
}

/// One argument of a call to a named function: a value, or a function written without its outer
/// parentheses (`@e => @e:>0`), which the function being called can apply, e.g. to the elements of
/// an array (see [`ExprKind::FunctionArg`]). A function is tried first, since its parameters would
/// otherwise be read as variables.
pub(super) fn call_argument<'src>(args_expr: impl Psr<'src, Expr> + 'src) -> impl Psr<'src, Expr> {
    // (A `fn` item rather than a closure, for the reason given in `utils`.)
    fn function_arg((params, body): (Vec<String>, FunctionBody)) -> ExprKind {
        ExprKind::FunctionArg(Box::new(FunctionArg { params, body }))
    }
    let function = located(function_literal(args_expr.clone()).map(function_arg)).boxed();
    choice((function, args_expr)).boxed()
}

pub fn pipe<'src>(
    arg0_expr: impl Psr<'src, Expr>,
    extra_args_expr: impl Psr<'src, Expr> + 'src,
) -> impl Psr<'src, Expr> {
    let scalar_args = just(COMPOSITION_ARGUMENT_BRACE_L)
        .ignore_then(
            call_argument(extra_args_expr.clone())
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
        .then(aggregate_parens(extra_args_expr.clone(), extra_args_expr.clone()).or_not())
        .map(|((name, name_loc), parens)| {
            let (extra_args, order_by) = parens.unwrap_or_default();
            PipeStep::Call {
                dimension: FunctionDimension::Aggregate,
                name: name.to_string(),
                name_loc,
                extra_args,
                order_by,
            }
        });

    // A standalone (leading) aggregate, e.g. `%count`, which has no `arg0` to its left. This is how
    // `count(*)` is written. It produces a [`Call`] with an empty `args` vector, distinguishing it
    // from a piped aggregate like `created_at%max` whose `args` contains the piped-in expression.
    // Value arguments in its parentheses are reported, and dropped, since there's no aggregated
    // value at the start of `args` for them to follow.
    let leading_args = extra_args_expr.clone().validate(leading_aggregate_arg);
    let leading_aggregate = pad().ignore_then(located(
        aggregate_name()
            .then(aggregate_parens(extra_args_expr, leading_args).or_not())
            .map(|((name, name_loc), parens)| {
                let order_by = parens.map(|(_, order_by)| order_by);
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

    fn call(input: &str) -> Call {
        match parse(input) {
            Ok(Expr {
                kind: ExprKind::Call(call),
                ..
            }) => call,
            other => panic!("expected a call for {input}, got {other:?}"),
        }
    }

    fn num(n: &str) -> Expr {
        ExprKind::Number(n.to_string()).into()
    }

    #[test]
    fn test_aggregate_value_arguments() {
        use NullsSort::*;
        use SortDirection::*;
        // A value argument follows the aggregated value in `args`.
        let c = call("x%percentile(0.5)");
        assert_eq!(c.args, vec![col("x"), num("0.5")]);
        assert_eq!(c.order_by, vec![]);
        // Through a `#` path, too.
        let c = call("#issues.x%percentile( 0.25 )");
        assert_eq!(c.args.len(), 2);
        assert_eq!(c.args[1], num("0.25"));
        // Value arguments and sorting entries may be mixed, in any order; each keeps its own order.
        let c = call(r"x%list(1 \sd 2 \\a)");
        assert_eq!(c.args, vec![col("x"), num("1"), num("2")]);
        assert_eq!(
            c.order_by,
            vec![
                sort(AggregateSortKey::Value, Desc, Last),
                sort(AggregateSortKey::Expr(col("a")), Asc, Last),
            ]
        );
        // A sorting entry's flags aren't taken for a value argument, nor vice versa.
        let c = call(r"x%percentile(\\a \d 0.5)");
        assert_eq!(c.args, vec![col("x"), num("0.5")]);
        assert_eq!(
            c.order_by,
            vec![sort(AggregateSortKey::Expr(col("a")), Desc, Last)]
        );
    }

    #[test]
    fn test_leading_aggregate_rejects_value_arguments() {
        // A standalone aggregate has no aggregated value for an argument to follow.
        assert!(parse("%count(5)").is_err());
        assert!(parse("%percentile(0.5)").is_err());
        // Its sorting entries are still accepted (the compiler judges them).
        assert_eq!(call(r"%count(\\a)").order_by.len(), 1);
        assert_eq!(call("%count()").args, vec![]);
    }

    #[test]
    fn test_aggregate_value_sort_flags_require_s() {
        assert!(parse(r"name%list(\d)").is_err());
        assert!(parse(r"name%list(\x)").is_err());
    }
}
