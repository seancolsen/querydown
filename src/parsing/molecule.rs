use chumsky::prelude::*;

use crate::syntax_tree::*;

use super::{
    conditions::{condition_set, implicit_condition_set},
    expressions::expression,
    utils::LqlParser,
};

pub fn top_level_condition_set() -> impl LqlParser<ConditionSet> {
    choice((
        discerned_condition_set(),
        implicit_condition_set(discerned_condition_set(), discerned_expression()),
    ))
}

pub fn discerned_expression() -> impl LqlParser<Expression> {
    make_discerned_expression(molecule())
}

pub fn discerned_condition_set() -> impl LqlParser<ConditionSet> {
    make_discerned_condition_set(molecule())
}

#[derive(Debug, Clone)]
pub enum Molecule {
    Expression(Expression),
    ConditionSet(ConditionSet),
}

fn molecule() -> impl LqlParser<Molecule> {
    recursive(|molecule| {
        choice((
            condition_set(make_discerned_expression(molecule.clone())).map(Molecule::ConditionSet),
            expression(make_discerned_condition_set(molecule)).map(Molecule::Expression),
        ))
    })
}

fn make_discerned_expression(molecule: impl LqlParser<Molecule>) -> impl LqlParser<Expression> {
    molecule.try_map(|v, span| match v {
        Molecule::Expression(e) => Ok(e),
        Molecule::ConditionSet(_) => Err(Simple::custom(
            span,
            "Expected expression, got condition set",
        )),
    })
}

fn make_discerned_condition_set(
    molecule: impl LqlParser<Molecule>,
) -> impl LqlParser<ConditionSet> {
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
                base: Value::Number("1".to_string()),
                compositions: vec![],
            })
        );
        assert_eq!(
            discerned_expression().parse("@true"),
            Ok(Expression {
                base: Value::True,
                compositions: vec![]
            })
        );
        assert_eq!(
            discerned_expression().parse("foo"),
            Ok(Expression {
                base: Value::Path(Path {
                    parts: vec![PathPart::LocalColumn("foo".to_string())]
                }),
                compositions: vec![],
            })
        );
        assert_eq!(
            discerned_expression().parse("foo|bar(2)%baz"),
            Ok(Expression {
                base: Value::Path(Path {
                    parts: vec![PathPart::LocalColumn("foo".to_string())]
                }),
                compositions: vec![
                    Composition {
                        function: Function {
                            name: "bar".to_string(),
                            dimension: FunctionDimension::Scalar
                        },
                        argument: Some(Expression {
                            base: Value::Number("2".to_string()),
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
                base: Value::Path(Path {
                    parts: vec![
                        PathPart::LocalColumn("foo".to_string()),
                        PathPart::LinkToOneViaColumn("bar".to_string())
                    ]
                }),
                compositions: vec![],
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
                }),
                compositions: vec![],
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
                            entries: vec![ConditionSetEntry::Comparison(Comparison {
                                left: ComparisonPart::Expression(Expression {
                                    base: Value::Path(Path {
                                        parts: vec![PathPart::LocalColumn("a".to_string())]
                                    }),
                                    compositions: vec![],
                                }),
                                operator: Operator::Eq,
                                right: ComparisonPart::Expression(Expression {
                                    base: Value::Number("1".to_string()),
                                    compositions: vec![],
                                }),
                            })],
                        },
                    })]
                }),
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
                        base: Value::Path(Path {
                            parts: vec![PathPart::LocalColumn("a".to_string())]
                        }),
                        compositions: vec![],
                    }),
                    operator: Operator::Eq,
                    right: ComparisonPart::Expression(Expression {
                        base: Value::Number("1".to_string()),
                        compositions: vec![],
                    }),
                })],
            })
        );
    }
}
