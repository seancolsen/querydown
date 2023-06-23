use chumsky::{prelude::*, text::*};

use crate::ast::*;
use crate::parser::utils::*;
use crate::tokens::*;

use super::{
    comparison::comparison, condition_set::condition_set, date::date, duration::duration,
    has_quantity::has_quantity, number::number, path::path, pipe::pipe,
};

pub fn expr() -> impl Psr<Expr> {
    recursive(|e| {
        // We begin with the highest precedence rules first. Lower precedence rules compose the
        // higher precedence rules.
        let prec_atom = choice((
            number().map(Expr::Number),
            date().map(Expr::Date),
            duration().map(Expr::Duration),
            string().map(Expr::String),
            variable().map(Expr::Variable),
            path(e.clone()).map(Expr::Path),
            has_quantity(e.clone()).map(Expr::HasQuantity),
            condition_set(e.clone()).map(Expr::ConditionSet),
            parenthetical(e.clone()),
        ));

        let prec_pipe = pipe(prec_atom, e);

        let prec_multiplication = multiplication(prec_pipe);

        let prec_addition = addition(prec_multiplication);

        comparison(prec_addition)
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
                            left: Expr::Path(vec![PathPart::Column("a".to_string())]),
                            operator: Operator::Eq,
                            right: Expr::Number("2".to_string()),
                            is_expand_left: false,
                            is_expand_right: false,
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

        assert_eq!(
            p("1:2:3"),
            Ok(Expr::Comparison(Box::new(Comparison {
                left: Expr::Comparison(Box::new(Comparison {
                    left: Expr::Number("1".to_string()),
                    operator: Operator::Eq,
                    right: Expr::Number("2".to_string()),
                    is_expand_left: false,
                    is_expand_right: false,
                })),
                operator: Operator::Eq,
                right: Expr::Number("3".to_string()),
                is_expand_left: false,
                is_expand_right: false,
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
                left: Expr::ConditionSet(ConditionSet {
                    entries: vec![
                        Expr::Path(vec![PathPart::Column("a".to_string())]),
                        Expr::Path(vec![PathPart::Column("b".to_string())]),
                    ],
                    conjunction: Conjunction::Or,
                }),
                operator: Operator::Neq,
                right: Expr::Sum(
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
                ),
                is_expand_left: true,
                is_expand_right: false,
            })))
        );
    }
}
