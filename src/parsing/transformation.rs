use chumsky::prelude::*;
use chumsky::text::whitespace;

use crate::ast::*;

use super::column_layout::column_layout;
use super::expression_or_condition_set::*;

pub fn transformation() -> impl Parser<char, Transformation, Error = Simple<char>> {
    top_level_condition_set()
        .or_not()
        .then_ignore(whitespace())
        .then(column_layout().or_not())
        .map(|(cs, cl)| Transformation {
            condition_set: cs.unwrap_or_default(),
            column_layout: cl.unwrap_or_default(),
        })
}

fn top_level_condition_set() -> impl Parser<char, ConditionSet, Error = Simple<char>> {
    // TODO Re-enable implicit "AND" condition sets at the top level
    // choice((condition_set(), implicit_condition_set(condition_set())))
    discerned_condition_set()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transformation() {
        assert_eq!(
            transformation().then_ignore(end()).parse("{a=8} -foo"),
            Ok(Transformation {
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
                                parts: vec![PathPart::LocalColumn("foo".to_string())]
                            })
                        },
                        alias: None,
                    }]
                }
            })
        );
    }
}
