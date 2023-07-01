use chumsky::{prelude::*, text::*};

use crate::ast::*;
use crate::parser::utils::*;
use crate::tokens::*;

use super::{
    comparison::comparison, condition_set::condition_set, date::date, duration::duration,
    has_quantity::has_quantity, number::number, path::path, pipe::pipe,
};

pub fn expr() -> impl Psr<Expr> {
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

    recursive(|prec_comparison| {
        let prec_atom = choice((
            number().map(Expr::Number),
            date().map(Expr::Date),
            duration().map(Expr::Duration),
            string().map(Expr::String),
            variable().map(Expr::Variable),
            path(prec_comparison.clone()).map(Expr::Path),
            has_quantity(prec_comparison.clone()).map(Expr::HasQuantity),
            condition_set(prec_comparison.clone()).map(Expr::ConditionSet),
            parenthetical(prec_comparison.clone()),
        ));

        let prec_pipe = pipe(prec_atom.clone(), prec_comparison.clone());

        let prec_multiplication = multiplication(prec_pipe);

        let prec_addition = addition(prec_multiplication);

        comparison(prec_addition.clone(), prec_comparison, prec_atom)
            .map(|c| Expr::Comparison(Box::new(c)))
            .or(prec_addition)
    })
}

fn operator(
    c: char,
    expr_enum_constructor: fn(Box<Expr>, Box<Expr>) -> Expr,
) -> impl Psr<fn(Box<Expr>, Box<Expr>) -> Expr> {
    just(c).padded().to(expr_enum_constructor)
}

fn variable() -> impl Psr<String> {
    just(CONST_SIGIL).ignore_then(ident())
}

fn string() -> impl Psr<String> {
    quoted(STRING_QUOTE_SINGLE).or(quoted(STRING_QUOTE_DOUBLE))
}

fn parenthetical(e: impl Psr<Expr>) -> impl Psr<Expr> {
    e.padded()
        .delimited_by(just(EXPR_PAREN_L), just(EXPR_PAREN_R))
}

fn multiplication(e: impl Psr<Expr>) -> impl Psr<Expr> {
    let op = choice((
        operator(EXPR_TIMES, Expr::Product),
        operator(EXPR_DIVIDE, Expr::Quotient),
    ));
    e.clone()
        .then(op.then(e).repeated())
        .foldl(|lhs, (f, rhs)| f(Box::new(lhs), Box::new(rhs)))
}

fn addition(e: impl Psr<Expr>) -> impl Psr<Expr> {
    let op = choice((
        operator(EXPR_PLUS, Expr::Sum),
        operator(EXPR_MINUS, Expr::Difference),
    ));
    e.clone()
        .then(op.then(e).repeated())
        .foldl(|lhs, (f, rhs)| f(Box::new(lhs), Box::new(rhs)))
}

#[cfg(test)]
mod tests {
    use crate::ast::*;
    use chumsky::prelude::*;

    use super::expr;

    #[test]
    fn test_parse_expr() {
        let parser = expr().then_ignore(end());
        let p = |s: &str| parser.parse(s);

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
            p("@1Y"),
            Ok(Expr::Duration(Duration {
                years: 1.0,
                ..Default::default()
            }))
        );
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
                            operator: Operator::Eq,
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
                    operator: Operator::Eq,
                    right: ComparisonSide::Expr(Expr::Number("2".to_string())),
                }))),
                operator: Operator::Eq,
                right: ComparisonSide::Expr(Expr::Number("3".to_string())),
            })))
        );

        assert_eq!(
            p("x:@a..@b"),
            Ok(Expr::Comparison(Box::new(Comparison {
                left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column("x".to_string())])),
                operator: Operator::Eq,
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
                operator: Operator::Eq,
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
                            }),
                            Expr::Number("2".to_string())
                        ],
                    }),
                    Expr::Number("3".to_string()),
                    Expr::Number("4".to_string()),
                ],
            }))
        );

        assert_eq!(
            p("[a b] ..! 2 + foo * @bar | baz"),
            Ok(Expr::Comparison(Box::new(Comparison {
                left: ComparisonSide::Expansion(ConditionSet {
                    entries: vec![
                        Expr::Path(vec![PathPart::Column("a".to_string())]),
                        Expr::Path(vec![PathPart::Column("b".to_string())]),
                    ],
                    conjunction: Conjunction::Or,
                }),
                operator: Operator::Neq,
                right: ComparisonSide::Expr(Expr::Sum(
                    Box::new(Expr::Number("2".to_string())),
                    Box::new(Expr::Product(
                        Box::new(Expr::Path(vec![PathPart::Column("foo".to_string())])),
                        Box::new(Expr::Call(Call {
                            name: "baz".to_string(),
                            dimension: FunctionDimension::Scalar,
                            syntax: CallSyntax::Piped,
                            args: vec![Expr::Variable("bar".to_string())],
                        })),
                    )),
                )),
            })))
        );
    }
}
