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
        let atom = choice((
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

        let piped = pipe(atom, e);

        let multiplication = choice((
            algebraic(EXPR_TIMES, Expr::Product, piped.clone()),
            algebraic(EXPR_DIVIDE, Expr::Quotient, piped),
        ));

        let addition = choice((
            algebraic(EXPR_PLUS, Expr::Sum, multiplication.clone()),
            algebraic(EXPR_MINUS, Expr::Difference, multiplication),
        ));

        comparison(addition)
    })
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

fn algebraic(
    operator: char,
    mapper: fn(Box<Expr>, Box<Expr>) -> Expr,
    e: impl Psr<Expr>,
) -> impl Psr<Expr> {
    e.clone()
        .then(just(operator).padded().ignore_then(e).repeated())
        .foldl(move |left, right| mapper(Box::new(left), Box::new(right)))
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
            p("5+7"),
            Ok(Expr::Sum(
                Box::new(Expr::Number("5".to_string())),
                Box::new(Expr::Number("7".to_string()))
            ))
        );

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
