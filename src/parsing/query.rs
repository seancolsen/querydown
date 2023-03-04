use chumsky::{prelude::*, text::*};

use crate::syntax_tree::*;
use crate::tokens::*;

use super::transformation::transformation;
use super::utils::*;
use super::values::*;

pub fn query() -> impl QdParser<Query> {
    let base_table = db_identifier().then_ignore(whitespace());
    let transformations = transformation().separated_by(
        whitespace()
            .then(exactly(TRANSFORMATION_DELIMITER))
            .then(whitespace()),
    );
    base_table
        .then(transformations)
        .then_ignore(whitespace().then(end()))
        .map(|(base_table, transformations)| Query {
            base_table,
            transformations,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query() {
        assert_eq!(
            query().parse("foo a=8 |bar"),
            Ok(Query {
                base_table: "foo".to_string(),
                transformations: vec![Transformation {
                    condition_set: ConditionSet {
                        conjunction: Conjunction::And,
                        entries: vec![ConditionSetEntry::Comparison(Comparison {
                            left: ComparisonPart::Expression(Expression {
                                base: ContextualValue::Value(Value::Path(Path::ToOne(vec![
                                    PathPartToOne::Column("a".to_string())
                                ]))),
                                compositions: vec![],
                            }),
                            operator: Operator::Eq,
                            right: ComparisonPart::Expression(Expression {
                                base: ContextualValue::Value(Value::Literal(Literal::Number(
                                    "8".to_string()
                                ))),
                                compositions: vec![],
                            })
                        })]
                    },
                    column_layout: ColumnLayout {
                        column_specs: vec![ColumnSpec {
                            column_control: ColumnControl::default(),
                            expression: Expression {
                                base: ContextualValue::Value(Value::Path(Path::ToOne(vec![
                                    PathPartToOne::Column("bar".to_string())
                                ]))),
                                compositions: vec![],
                            },
                            alias: None,
                        }]
                    }
                }],
            })
        );
    }
}
