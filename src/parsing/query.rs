use chumsky::{prelude::*, text::*};

use crate::ast::*;
use crate::tokens::*;

use super::transformation::transformation;
use super::utils::*;
use super::values::*;

pub fn query() -> impl Parser<char, Query, Error = Simple<char>> {
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
            query().parse("foo {a=8} -bar"),
            Ok(Query {
                base_table: "foo".to_string(),
                transformations: vec![Transformation {
                    condition_set: ConditionSet {
                        conjunction: Conjunction::And,
                        entries: vec![ConditionSetEntry::Condition(Condition {
                            left: Expression {
                                base: Value::Path(Path {
                                    parts: vec![PathPart::LocalColumn("a".to_string())]
                                })
                            },
                            operator: Operator::Eq,
                            right: Expression {
                                base: Value::Number("8".to_string())
                            }
                        })]
                    },
                    column_layout: ColumnLayout {
                        column_specs: vec![ColumnSpec {
                            column_control: ColumnControl::default(),
                            expression: Expression {
                                base: Value::Path(Path {
                                    parts: vec![PathPart::LocalColumn("bar".to_string())]
                                })
                            },
                            alias: None,
                        }]
                    }
                }],
            })
        );
    }
}
