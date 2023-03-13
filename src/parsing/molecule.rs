use chumsky::prelude::*;

use crate::syntax_tree::*;

use super::{
    conditions::{condition_set, implicit_condition_set},
    expressions::expression,
    utils::QdParser,
};

pub fn top_level_condition_set() -> impl QdParser<ConditionSet> {
    choice((
        discerned_condition_set(),
        implicit_condition_set(discerned_condition_set(), discerned_expression()),
    ))
}

pub fn discerned_expression() -> impl QdParser<Expression> {
    make_discerned_expression(molecule())
}

pub fn discerned_condition_set() -> impl QdParser<ConditionSet> {
    make_discerned_condition_set(molecule())
}

#[derive(Debug, Clone)]
pub enum Molecule {
    Expression(Expression),
    ConditionSet(ConditionSet),
}

fn molecule() -> impl QdParser<Molecule> {
    recursive(|molecule| {
        choice((
            condition_set(make_discerned_expression(molecule.clone())).map(Molecule::ConditionSet),
            expression(make_discerned_condition_set(molecule)).map(Molecule::Expression),
        ))
    })
}

fn make_discerned_expression(molecule: impl QdParser<Molecule>) -> impl QdParser<Expression> {
    molecule.try_map(|v, span| match v {
        Molecule::Expression(e) => Ok(e),
        Molecule::ConditionSet(_) => Err(Simple::custom(
            span,
            "Expected expression, got condition set",
        )),
    })
}

fn make_discerned_condition_set(molecule: impl QdParser<Molecule>) -> impl QdParser<ConditionSet> {
    molecule.try_map(|v, span| match v {
        Molecule::ConditionSet(e) => Ok(e),
        Molecule::Expression(_) => Err(Simple::custom(
            span,
            "Expected condition set, got expression",
        )),
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
                base: Value::Literal(Literal::Number("1".to_string())),
                compositions: vec![],
            })
        );
        assert_eq!(
            discerned_expression().parse("@true"),
            Ok(Expression {
                base: Value::Literal(Literal::True),
                compositions: vec![]
            })
        );
        assert_eq!(
            discerned_expression().parse("foo"),
            Ok(Expression {
                base: (Value::Path(vec![PathPart::Column("foo".to_string())])),
                compositions: vec![],
            })
        );
        assert_eq!(
            discerned_expression().parse("foo:bar(2)%baz"),
            Ok(Expression {
                base: Value::Path(vec![PathPart::Column("foo".to_string())]),
                compositions: vec![
                    Composition {
                        function: Function {
                            name: "bar".to_string(),
                            dimension: FunctionDimension::Scalar
                        },
                        argument: Some(Expression {
                            base: Value::Literal(Literal::Number("2".to_string())),
                            compositions: vec![]
                        }),
                    },
                    Composition {
                        function: Function {
                            name: "baz".to_string(),
                            dimension: FunctionDimension::Aggregate
                        },
                        argument: None,
                    }
                ],
            })
        );
        assert_eq!(
            discerned_expression().parse("foo .bar"),
            Ok(Expression {
                base: Value::Path(vec![
                    PathPart::Column("foo".to_string()),
                    PathPart::Column("bar".to_string()),
                ]),
                compositions: vec![],
            })
        );
        assert_eq!(
            discerned_expression().parse("#foo(bar)"),
            Ok(Expression {
                base: Value::Path(vec![PathPart::TableWithMany(TableWithMany {
                    table: "foo".to_string(),
                    linking_column: Some("bar".to_string()),
                    condition_set: ConditionSet::default(),
                }),]),
                compositions: vec![],
            })
        );
        assert_eq!(
            discerned_expression().parse("#foo(bar){a=1}"),
            Ok(Expression {
                base: Value::Path(vec![PathPart::TableWithMany(TableWithMany {
                    table: "foo".to_string(),
                    linking_column: Some("bar".to_string()),
                    condition_set: ConditionSet {
                        conjunction: Conjunction::And,
                        entries: vec![ConditionSetEntry::Comparison(Comparison {
                            left: ComparisonPart::Expression(Expression {
                                base: Value::Path(vec![PathPart::Column("a".to_string())]),
                                compositions: vec![],
                            }),
                            operator: Operator::Eq,
                            right: ComparisonPart::Expression(Expression {
                                base: Value::Literal(Literal::Number("1".to_string())),
                                compositions: vec![],
                            }),
                        })],
                    },
                }),]),
                compositions: vec![],
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
                entries: vec![ConditionSetEntry::Comparison(Comparison {
                    left: ComparisonPart::Expression(Expression {
                        base: Value::Path(vec![PathPart::Column("a".to_string())]),
                        compositions: vec![],
                    }),
                    operator: Operator::Eq,
                    right: ComparisonPart::Expression(Expression {
                        base: Value::Literal(Literal::Number("1".to_string())),
                        compositions: vec![],
                    }),
                })],
            })
        );
    }
}
