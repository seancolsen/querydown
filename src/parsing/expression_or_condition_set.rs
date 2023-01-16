use chumsky::prelude::*;

use crate::ast::*;

use super::{conditions::condition_set, expressions::expression};

pub fn discerned_expression() -> impl Parser<char, Expression, Error = Simple<char>> + Clone {
    make_discerned_expression(expression_or_condition_set())
}

pub fn discerned_condition_set() -> impl Parser<char, ConditionSet, Error = Simple<char>> + Clone {
    make_discerned_condition_set(expression_or_condition_set())
}

fn make_discerned_expression<P>(
    expression_or_condition_set: P,
) -> impl Parser<char, Expression, Error = Simple<char>> + Clone
where
    P: Parser<char, ExpressionOrConditionSet, Error = Simple<char>> + Clone,
{
    expression_or_condition_set.try_map(|v, span| match v {
        ExpressionOrConditionSet::Expression(e) => Ok(e),
        ExpressionOrConditionSet::ConditionSet(_) => Err(Simple::custom(
            span,
            "Expected expression, got condition set",
        )),
    })
}

fn make_discerned_condition_set<P>(
    expression_or_condition_set: P,
) -> impl Parser<char, ConditionSet, Error = Simple<char>> + Clone
where
    P: Parser<char, ExpressionOrConditionSet, Error = Simple<char>> + Clone,
{
    expression_or_condition_set.try_map(|v, span| match v {
        ExpressionOrConditionSet::ConditionSet(e) => Ok(e),
        ExpressionOrConditionSet::Expression(_) => Err(Simple::custom(
            span,
            "Expected condition set, got expression",
        )),
    })
}

#[derive(Debug, Clone)]
pub enum ExpressionOrConditionSet {
    Expression(Expression),
    ConditionSet(ConditionSet),
}

fn expression_or_condition_set(
) -> impl Parser<char, ExpressionOrConditionSet, Error = Simple<char>> + Clone {
    recursive(|v| {
        choice((
            condition_set(make_discerned_expression(v.clone()))
                .map(ExpressionOrConditionSet::ConditionSet),
            expression(make_discerned_condition_set(v)).map(ExpressionOrConditionSet::Expression),
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discerned_expression() {
        assert_eq!(
            discerned_expression().parse("1"),
            Ok(Expression {
                base: Value::Number("1".to_string())
            })
        );
        assert_eq!(
            discerned_expression().parse("@true"),
            Ok(Expression { base: Value::True })
        );
        assert_eq!(
            discerned_expression().parse("foo"),
            Ok(Expression {
                base: Value::Path(Path {
                    parts: vec![PathPart::LocalColumn("foo".to_string())]
                })
            })
        );
        assert_eq!(
            discerned_expression().parse("foo .bar"),
            Ok(Expression {
                base: Value::Path(Path {
                    parts: vec![
                        PathPart::LocalColumn("foo".to_string()),
                        PathPart::LinkToOne("bar".to_string())
                    ]
                })
            })
        );
        assert_eq!(
            discerned_expression().parse("*foo(bar)"),
            Ok(Expression {
                base: Value::Path(Path {
                    parts: vec![PathPart::LinkToMany(LinkToMany {
                        table: "foo".to_string(),
                        column: Some("bar".to_string()),
                        condition_set: ConditionSet::default(),
                    })]
                })
            })
        );
        assert_eq!(
            discerned_expression().parse("*foo(bar){a=1}"),
            Ok(Expression {
                base: Value::Path(Path {
                    parts: vec![PathPart::LinkToMany(LinkToMany {
                        table: "foo".to_string(),
                        column: Some("bar".to_string()),
                        condition_set: ConditionSet {
                            conjunction: Conjunction::And,
                            entries: vec![ConditionSetEntry::Condition(Condition {
                                left: Expression {
                                    base: Value::Path(Path {
                                        parts: vec![PathPart::LocalColumn("a".to_string())]
                                    }),
                                },
                                operator: Operator::Eq,
                                right: Expression {
                                    base: Value::Number("1".to_string())
                                },
                            })],
                        },
                    })]
                })
            })
        );

        assert!(discerned_expression().parse("{a=1}").is_err());
    }

    #[test]
    fn test_discerned_condition_set() {
        assert_eq!(
            discerned_condition_set().parse("{a=1}"),
            Ok(ConditionSet {
                conjunction: Conjunction::And,
                entries: vec![ConditionSetEntry::Condition(Condition {
                    left: Expression {
                        base: Value::Path(Path {
                            parts: vec![PathPart::LocalColumn("a".to_string())]
                        }),
                    },
                    operator: Operator::Eq,
                    right: Expression {
                        base: Value::Number("1".to_string())
                    },
                })],
            })
        );
    }
}
