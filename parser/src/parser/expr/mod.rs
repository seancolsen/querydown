mod case;
mod comparison;
mod condition_set;
mod date;
mod duration;
mod has_quantity;
mod number;
mod path;
mod pipe;

pub use comparison::operator as comparison_operator;
pub use path::path_to_one;

use chumsky::{prelude::*, text::*};

use crate::ast::*;
use crate::parser::utils::*;
use crate::tokens::*;

use self::{
    case::case, comparison::comparison, condition_set::condition_set, date::date,
    duration::duration, has_quantity::has_quantity, number::number, path::path, pipe::pipe,
};

pub fn expr<'src>() -> impl Psr<'src, Expr> {
    // A bit about how this works:
    //
    // - `prec` is short for "precedence".
    //
    // - `prec_atom` is the highest precedence rule. These kinds of expressions can be parsed
    //   unambiguously because they are entirely self-contained, for example, a string. Some of
    //   these expressions also include other expressions, e.g. `parenthetical`, and in that case,
    //   we pass the recursive rule to the sub-parser so that it can parse its own sub-expressions
    //   using the most generalized expression parser which includes rules for all levels of
    //   precedence.
    //
    // - As we move to lower precedence rules, we build them up by composing the higher precedence
    //   rules.
    //
    // - In total, it's a circular system of composition. The lower precedence rules compose the
    //   higher precedence rules, and the highest precedence rule recursively composes the lowest
    //   precedence rule.
    //
    // I took this approach from [Chumsky's example code][1].
    //
    // [1]: https://github.com/zesterer/chumsky/blob/0.9/examples/foo.rs#L33

    // Each precedence level is `.boxed()` to type-erase it. Chumsky combinator types grow
    // multiplicatively as they nest, and this parser nests deeply; without these boxing
    // "firewalls" the fully monomorphized type — and rustc's memory use compiling it — balloons
    // far enough to OOM the build. Boxing is semantically transparent and also speeds up compiles.
    recursive(|prec_comma| {
        let prec_atom = choice((
            // `duration` is tried before `number` because both begin with a digit. A duration
            // requires a trailing unit (e.g. `6m`), so a unit-less number like `6` falls through.
            duration().map(Expr::Duration),
            number().map(Expr::Number),
            date().map(Expr::Date),
            string().map(Expr::String),
            variable().map(Expr::Variable),
            path(prec_comma.clone()).map(Expr::Path),
            has_quantity(prec_comma.clone()).map(Expr::HasQuantity),
            case(prec_comma.clone()).map(Expr::Case),
            condition_set(prec_comma.clone()).map(Expr::ConditionSet),
            parenthetical(prec_comma.clone()),
        ))
        .boxed();

        let prec_pipe = pipe(prec_atom.clone(), prec_comma.clone()).boxed();

        let prec_multiplication = multiplication(prec_pipe).boxed();

        let prec_addition = addition(prec_multiplication).boxed();

        // The right-hand side of a comparison uses a parallel precedence chain that is identical to
        // the one above in every respect except its atom: a bare (unquoted) word on the right-hand
        // side is interpreted as a string literal rather than a column reference. See
        // `comparison_rhs_value` for details.
        let prec_atom_rhs = choice((
            // See the ordering note on `prec_atom` above: `duration` before `number`.
            duration().map(Expr::Duration),
            number().map(Expr::Number),
            date().map(Expr::Date),
            string().map(Expr::String),
            variable().map(Expr::Variable),
            comparison_rhs_value(prec_comma.clone()),
            has_quantity(prec_comma.clone()).map(Expr::HasQuantity),
            condition_set(prec_comma.clone()).map(Expr::ConditionSet),
            parenthetical(prec_comma.clone()),
        ))
        .boxed();

        let prec_pipe_rhs = pipe(prec_atom_rhs, prec_comma.clone()).boxed();

        let prec_multiplication_rhs = multiplication(prec_pipe_rhs).boxed();

        let prec_addition_rhs = addition(prec_multiplication_rhs).boxed();

        let prec_comparison = comparison(
            prec_addition.clone(),
            prec_addition_rhs,
            prec_comma,
            prec_atom,
        )
        .map(|c| Expr::Comparison(Box::new(c)))
        .or(prec_addition)
        .boxed();

        let prec_negation = negation(prec_comparison).boxed();

        or_condition_set(prec_negation).boxed()
    })
}

/// Negates an expression with a `!` prefix, producing [`Expr::Not`]. This binds more loosely than
/// comparison (so `!foo:2` negates the whole comparison) but more tightly than the comma shorthand.
/// The prefix may be repeated, e.g. `!!foo`. When no `!` is present, the expression passes through
/// unchanged, so this rule is transparent.
fn negation<'src>(e: impl Psr<'src, Expr>) -> impl Psr<'src, Expr> {
    just(NEGATE)
        .then_ignore(pad())
        .repeated()
        .foldr(e, |_, expr| Expr::Not(Box::new(expr)))
}

/// Joins comma-separated expressions into an "OR" condition set, e.g. `foo:1,bar:2`. This is the
/// lowest-precedence operator in the language. A single expression with no trailing comma is passed
/// through unchanged, so this rule is transparent when no comma is present.
fn or_condition_set<'src>(e: impl Psr<'src, Expr>) -> impl Psr<'src, Expr> {
    e.separated_by(just(CONDITION_SET_OR_SHORTHAND).padded_by(pad()))
        .at_least(1)
        .collect::<Vec<Expr>>()
        .map(|mut entries| {
            if entries.len() == 1 {
                entries.pop().unwrap()
            } else {
                Expr::ConditionSet(ConditionSet::via_or(entries))
            }
        })
}

fn operator<'src>(
    c: char,
    expr_enum_constructor: fn(Box<Expr>, Box<Expr>) -> Expr,
) -> impl Psr<'src, fn(Box<Expr>, Box<Expr>) -> Expr> {
    just(c).padded_by(pad()).to(expr_enum_constructor)
}

fn variable<'src>() -> impl Psr<'src, String> {
    just(CONST_SIGIL).ignore_then(ident().map(|s: &str| s.to_string()))
}

fn string<'src>() -> impl Psr<'src, String> {
    quoted(STRING_QUOTE_SINGLE).or(quoted(STRING_QUOTE_DOUBLE))
}

/// Parses the value on the right-hand side of a comparison, where a bare (unquoted) word is
/// interpreted as a string literal rather than a column reference. This matches the behavior users
/// expect from tools like GitHub, where e.g. `description:title` searches for the literal text
/// "title". To refer to a column on the right-hand side, the identifier can either be quoted with
/// backticks (e.g. `` `title` ``) or written as a multi-part path (e.g. `foo.bar`).
///
/// A bare word is only treated as a string when it stands alone. If it is the head of a longer path
/// (e.g. `foo.bar`), the whole path is parsed as a column reference instead. Everywhere other than
/// the right-hand side of a comparison, bare words continue to be parsed as column references.
fn comparison_rhs_value<'src>(expr: impl Psr<'src, Expr>) -> impl Psr<'src, Expr> {
    let bare_string = ident()
        .then_ignore(pad().then(just(PATH_SEPARATOR)).not())
        .map(|s: &str| Expr::String(s.to_string()));
    choice((bare_string, path(expr).map(Expr::Path)))
}

fn parenthetical<'src>(e: impl Psr<'src, Expr>) -> impl Psr<'src, Expr> {
    e.padded_by(pad())
        .delimited_by(just(EXPR_PAREN_L), just(EXPR_PAREN_R))
}

fn multiplication<'src>(e: impl Psr<'src, Expr>) -> impl Psr<'src, Expr> {
    let op = choice((
        operator(EXPR_TIMES, Expr::Product),
        operator(EXPR_DIVIDE, Expr::Quotient),
    ));
    e.clone().foldl(op.then(e).repeated(), |lhs, (f, rhs)| {
        f(Box::new(lhs), Box::new(rhs))
    })
}

fn addition<'src>(e: impl Psr<'src, Expr>) -> impl Psr<'src, Expr> {
    let op = choice((
        operator(EXPR_PLUS, Expr::Sum),
        operator(EXPR_MINUS, Expr::Difference),
    ));
    e.clone().foldl(op.then(e).repeated(), |lhs, (f, rhs)| {
        f(Box::new(lhs), Box::new(rhs))
    })
}

#[cfg(test)]
mod tests {
    use crate::ast::*;
    use chumsky::prelude::*;

    use super::expr;

    #[test]
    fn test_parse_expr() {
        let p = |s: &str| {
            expr()
                .then_ignore(end())
                .parse(s)
                .into_result()
                .map_err(|_| ())
        };

        assert_eq!(p("8"), Ok(Expr::Number("8".to_string())));
        assert_eq!(
            p("@2000-01-01"),
            Ok(Expr::Date(Date {
                year: 2000,
                day: 1,
                month: 1
            }))
        );
        assert_eq!(
            p("1y"),
            Ok(Expr::Duration(Duration {
                years: 1.0,
                ..Default::default()
            }))
        );
        // A digit-initial token with no unit is a plain number, not a duration.
        assert_eq!(p("6"), Ok(Expr::Number("6".to_string())));
        assert_eq!(p("'foo'"), Ok(Expr::String("foo".to_string())));
        assert_eq!(p("\"foo\""), Ok(Expr::String("foo".to_string())));
        assert_eq!(p("@foo"), Ok(Expr::Variable("foo".to_string())));
        assert_eq!(p("@null"), Ok(Expr::Variable("null".to_string())));
        assert_eq!(
            p("foo"),
            Ok(Expr::Path(vec![PathPart::Column("foo".to_string())]))
        );
        assert_eq!(
            p("++#foo"),
            Ok(Expr::HasQuantity(HasQuantity {
                quantity: Quantity::AtLeastOne,
                path_parts: vec![PathPart::TableWithMany(TableWithMany {
                    table: "foo".to_string(),
                    condition_set: ConditionSet::default(),
                    linking_column: None
                })]
            }))
        );
        assert_eq!(
            p("++#foo{a:2}"),
            Ok(Expr::HasQuantity(HasQuantity {
                quantity: Quantity::AtLeastOne,
                path_parts: vec![PathPart::TableWithMany(TableWithMany {
                    table: "foo".to_string(),
                    condition_set: ConditionSet {
                        conjunction: Conjunction::And,
                        entries: vec![Expr::Comparison(Box::new(Comparison {
                            left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column(
                                "a".to_string()
                            )])),
                            operator: Operator::Match,
                            right: ComparisonSide::Expr(Expr::Number("2".to_string())),
                        }))]
                    },
                    linking_column: None
                })]
            }))
        );
        assert_eq!(
            p("[a b]"),
            Ok(Expr::ConditionSet(ConditionSet {
                conjunction: Conjunction::Or,
                entries: vec![
                    Expr::Path(vec![PathPart::Column("a".to_string())]),
                    Expr::Path(vec![PathPart::Column("b".to_string())]),
                ]
            }))
        );
        assert_eq!(
            p("{a b}"),
            Ok(Expr::ConditionSet(ConditionSet {
                conjunction: Conjunction::And,
                entries: vec![
                    Expr::Path(vec![PathPart::Column("a".to_string())]),
                    Expr::Path(vec![PathPart::Column("b".to_string())]),
                ]
            }))
        );

        // The comma is shorthand for an "OR" condition set.
        assert_eq!(
            p("a,b"),
            Ok(Expr::ConditionSet(ConditionSet {
                conjunction: Conjunction::Or,
                entries: vec![
                    Expr::Path(vec![PathPart::Column("a".to_string())]),
                    Expr::Path(vec![PathPart::Column("b".to_string())]),
                ]
            }))
        );

        // Whitespace is allowed around the comma, and it joins three or more expressions.
        assert_eq!(
            p("a , b ,c"),
            Ok(Expr::ConditionSet(ConditionSet {
                conjunction: Conjunction::Or,
                entries: vec![
                    Expr::Path(vec![PathPart::Column("a".to_string())]),
                    Expr::Path(vec![PathPart::Column("b".to_string())]),
                    Expr::Path(vec![PathPart::Column("c".to_string())]),
                ]
            }))
        );

        // The comma has lower precedence than comparison, so each comparison becomes its own entry.
        assert_eq!(
            p("foo:1,bar:2"),
            Ok(Expr::ConditionSet(ConditionSet {
                conjunction: Conjunction::Or,
                entries: vec![
                    Expr::Comparison(Box::new(Comparison {
                        left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column(
                            "foo".to_string()
                        )])),
                        operator: Operator::Match,
                        right: ComparisonSide::Expr(Expr::Number("1".to_string())),
                    })),
                    Expr::Comparison(Box::new(Comparison {
                        left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column(
                            "bar".to_string()
                        )])),
                        operator: Operator::Match,
                        right: ComparisonSide::Expr(Expr::Number("2".to_string())),
                    })),
                ]
            }))
        );

        // The comma also has lower precedence than addition.
        assert_eq!(
            p("1+2,3"),
            Ok(Expr::ConditionSet(ConditionSet {
                conjunction: Conjunction::Or,
                entries: vec![
                    Expr::Sum(
                        Box::new(Expr::Number("1".to_string())),
                        Box::new(Expr::Number("2".to_string()))
                    ),
                    Expr::Number("3".to_string()),
                ]
            }))
        );

        // A trailing comma is not allowed.
        assert!(p("a,").is_err());

        assert_eq!(
            p("5*7"),
            Ok(Expr::Product(
                Box::new(Expr::Number("5".to_string())),
                Box::new(Expr::Number("7".to_string()))
            ))
        );

        assert_eq!(
            p("@a/@b"),
            Ok(Expr::Quotient(
                Box::new(Expr::Variable("a".to_string())),
                Box::new(Expr::Variable("b".to_string()))
            ))
        );

        assert_eq!(
            p("5+7"),
            Ok(Expr::Sum(
                Box::new(Expr::Number("5".to_string())),
                Box::new(Expr::Number("7".to_string()))
            ))
        );

        assert_eq!(
            p("@a-@b"),
            Ok(Expr::Difference(
                Box::new(Expr::Variable("a".to_string())),
                Box::new(Expr::Variable("b".to_string()))
            ))
        );

        assert_eq!(
            p("5 - 7"),
            Ok(Expr::Difference(
                Box::new(Expr::Number("5".to_string())),
                Box::new(Expr::Number("7".to_string()))
            ))
        );

        assert_eq!(
            p("5 -7"),
            Ok(Expr::Difference(
                Box::new(Expr::Number("5".to_string())),
                Box::new(Expr::Number("7".to_string()))
            ))
        );

        // This is two expressions, not one.
        assert!(p("5 (-7)").is_err());

        assert_eq!(
            p("5*7+3"),
            Ok(Expr::Sum(
                Box::new(Expr::Product(
                    Box::new(Expr::Number("5".to_string())),
                    Box::new(Expr::Number("7".to_string()))
                )),
                Box::new(Expr::Number("3".to_string()))
            ))
        );

        assert_eq!(
            p("3+5*7"),
            Ok(Expr::Sum(
                Box::new(Expr::Number("3".to_string())),
                Box::new(Expr::Product(
                    Box::new(Expr::Number("5".to_string())),
                    Box::new(Expr::Number("7".to_string()))
                ))
            ))
        );

        assert_eq!(
            p("( 3 + 5 )"),
            Ok(Expr::Sum(
                Box::new(Expr::Number("3".to_string())),
                Box::new(Expr::Number("5".to_string()))
            ))
        );

        assert_eq!(
            p("(3+5)*7"),
            Ok(Expr::Product(
                Box::new(Expr::Sum(
                    Box::new(Expr::Number("3".to_string())),
                    Box::new(Expr::Number("5".to_string()))
                )),
                Box::new(Expr::Number("7".to_string()))
            ))
        );

        // Comparisons don't fold
        assert!(p("1:2:3").is_err());

        // Nested comparisons can be done via parentheses
        assert_eq!(
            p("(1:2):3"),
            Ok(Expr::Comparison(Box::new(Comparison {
                left: ComparisonSide::Expr(Expr::Comparison(Box::new(Comparison {
                    left: ComparisonSide::Expr(Expr::Number("1".to_string())),
                    operator: Operator::Match,
                    right: ComparisonSide::Expr(Expr::Number("2".to_string())),
                }))),
                operator: Operator::Match,
                right: ComparisonSide::Expr(Expr::Number("3".to_string())),
            })))
        );

        assert_eq!(
            p("x:@a..@b"),
            Ok(Expr::Comparison(Box::new(Comparison {
                left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column("x".to_string())])),
                operator: Operator::Match,
                right: ComparisonSide::Range(Range {
                    lower: RangeBound {
                        expr: Expr::Variable("a".to_string()),
                        exclusivity: Exclusivity::Inclusive,
                    },
                    upper: RangeBound {
                        expr: Expr::Variable("b".to_string()),
                        exclusivity: Exclusivity::Inclusive,
                    },
                }),
            })))
        );

        assert_eq!(
            p("x:@a<..<@b"),
            Ok(Expr::Comparison(Box::new(Comparison {
                left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column("x".to_string())])),
                operator: Operator::Match,
                right: ComparisonSide::Range(Range {
                    lower: RangeBound {
                        expr: Expr::Variable("a".to_string()),
                        exclusivity: Exclusivity::Exclusive,
                    },
                    upper: RangeBound {
                        expr: Expr::Variable("b".to_string()),
                        exclusivity: Exclusivity::Exclusive,
                    },
                }),
            })))
        );

        assert_eq!(
            p("1|a|b(2)|c(3 4)"),
            Ok(Expr::Call(Call {
                name: "c".to_string(),
                dimension: FunctionDimension::Scalar,
                syntax: CallSyntax::Piped,
                args: vec![
                    Expr::Call(Call {
                        name: "b".to_string(),
                        dimension: FunctionDimension::Scalar,
                        syntax: CallSyntax::Piped,
                        args: vec![
                            Expr::Call(Call {
                                name: "a".to_string(),
                                dimension: FunctionDimension::Scalar,
                                syntax: CallSyntax::Piped,
                                args: vec![Expr::Number("1".to_string())],
                                order_by: vec![],
                            }),
                            Expr::Number("2".to_string())
                        ],
                        order_by: vec![],
                    }),
                    Expr::Number("3".to_string()),
                    Expr::Number("4".to_string()),
                ],
                order_by: vec![],
            }))
        );

        assert_eq!(
            p("[a b] ..: 2 + foo * @bar | baz"),
            Ok(Expr::Comparison(Box::new(Comparison {
                left: ComparisonSide::Expansion(ConditionSet {
                    entries: vec![
                        Expr::Path(vec![PathPart::Column("a".to_string())]),
                        Expr::Path(vec![PathPart::Column("b".to_string())]),
                    ],
                    conjunction: Conjunction::Or,
                }),
                operator: Operator::Match,
                right: ComparisonSide::Expr(Expr::Sum(
                    Box::new(Expr::Number("2".to_string())),
                    Box::new(Expr::Product(
                        // A bare word on the right-hand side of a comparison is a string literal.
                        Box::new(Expr::String("foo".to_string())),
                        Box::new(Expr::Call(Call {
                            name: "baz".to_string(),
                            dimension: FunctionDimension::Scalar,
                            syntax: CallSyntax::Piped,
                            args: vec![Expr::Variable("bar".to_string())],
                            order_by: vec![],
                        })),
                    )),
                )),
            })))
        );

        // A standalone aggregate `%count` has no `arg0`, so its `args` vector is empty. This is how
        // `count(*)` is written.
        assert_eq!(
            p("%count"),
            Ok(Expr::Call(Call {
                name: "count".to_string(),
                dimension: FunctionDimension::Aggregate,
                syntax: CallSyntax::Piped,
                args: vec![],
                order_by: vec![],
            }))
        );

        // A piped aggregate `created_at%max` carries the piped-in expression as its single argument.
        assert_eq!(
            p("created_at%max"),
            Ok(Expr::Call(Call {
                name: "max".to_string(),
                dimension: FunctionDimension::Aggregate,
                syntax: CallSyntax::Piped,
                args: vec![Expr::Path(vec![PathPart::Column("created_at".to_string())])],
                order_by: vec![],
            }))
        );

        // A `!` prefix negates a bare expression.
        assert_eq!(
            p("!foo"),
            Ok(Expr::Not(Box::new(Expr::Path(vec![PathPart::Column(
                "foo".to_string()
            )]))))
        );

        // Negation binds more loosely than comparison, so `!foo:2` negates the whole comparison.
        assert_eq!(
            p("!foo:2"),
            Ok(Expr::Not(Box::new(Expr::Comparison(Box::new(
                Comparison {
                    left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column(
                        "foo".to_string()
                    )])),
                    operator: Operator::Match,
                    right: ComparisonSide::Expr(Expr::Number("2".to_string())),
                }
            )))))
        );

        // The `!` prefix can be repeated.
        assert_eq!(
            p("!!foo"),
            Ok(Expr::Not(Box::new(Expr::Not(Box::new(Expr::Path(vec![
                PathPart::Column("foo".to_string())
            ]))))))
        );

        // Negation binds more tightly than the comma shorthand, so only the first entry is negated.
        assert_eq!(
            p("!foo:1,bar:2"),
            Ok(Expr::ConditionSet(ConditionSet {
                conjunction: Conjunction::Or,
                entries: vec![
                    Expr::Not(Box::new(Expr::Comparison(Box::new(Comparison {
                        left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column(
                            "foo".to_string()
                        )])),
                        operator: Operator::Match,
                        right: ComparisonSide::Expr(Expr::Number("1".to_string())),
                    })))),
                    Expr::Comparison(Box::new(Comparison {
                        left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column(
                            "bar".to_string()
                        )])),
                        operator: Operator::Match,
                        right: ComparisonSide::Expr(Expr::Number("2".to_string())),
                    })),
                ]
            }))
        );
    }

    #[test]
    fn test_bare_text_on_comparison_rhs_is_a_string() {
        let p = |s: &str| {
            expr()
                .then_ignore(end())
                .parse(s)
                .into_result()
                .map_err(|_| ())
        };

        // A bare word on the left-hand side of a comparison is a column reference, while a bare
        // word on the right-hand side is a string literal.
        assert_eq!(
            p("description:title"),
            Ok(Expr::Comparison(Box::new(Comparison {
                left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column(
                    "description".to_string()
                )])),
                operator: Operator::Match,
                right: ComparisonSide::Expr(Expr::String("title".to_string())),
            })))
        );

        // A bare word on the right-hand side behaves identically to a quoted string.
        assert_eq!(p("description:title"), p("description:\"title\""));

        // Backtick-quoting the right-hand side designates it as a column reference instead.
        assert_eq!(
            p("description:`title`"),
            Ok(Expr::Comparison(Box::new(Comparison {
                left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column(
                    "description".to_string()
                )])),
                operator: Operator::Match,
                right: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column(
                    "title".to_string()
                )])),
            })))
        );

        // A multi-part path on the right-hand side remains a column reference, since it is not a
        // bare, standalone word.
        assert_eq!(
            p("description:foo.bar"),
            Ok(Expr::Comparison(Box::new(Comparison {
                left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column(
                    "description".to_string()
                )])),
                operator: Operator::Match,
                right: ComparisonSide::Expr(Expr::Path(vec![
                    PathPart::Column("foo".to_string()),
                    PathPart::Column("bar".to_string()),
                ])),
            })))
        );

        // The rule applies to other comparison operators too, not just match.
        assert_eq!(
            p("status:=open"),
            Ok(Expr::Comparison(Box::new(Comparison {
                left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column(
                    "status".to_string()
                )])),
                operator: Operator::Eq,
                right: ComparisonSide::Expr(Expr::String("open".to_string())),
            })))
        );
    }
}
