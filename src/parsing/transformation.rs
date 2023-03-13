use chumsky::prelude::*;
use chumsky::text::whitespace;

use crate::syntax_tree::*;

use super::column_layout::column_layout;
use super::molecule::*;
use super::utils::QdParser;

pub fn transformation() -> impl QdParser<Transformation> {
    top_level_condition_set()
        .or_not()
        .then_ignore(whitespace())
        .then(column_layout().or_not())
        .map(|(cs, cl)| Transformation {
            condition_set: cs.unwrap_or_default(),
            column_layout: cl.unwrap_or_default(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transformation() {
        assert_eq!(
            transformation().then_ignore(end()).parse("{a=8} |foo"),
            Ok(Transformation {
                condition_set: ConditionSet {
                    conjunction: Conjunction::And,
                    entries: vec![ConditionSetEntry::Comparison(Comparison {
                        left: ComparisonPart::Expression(Expression {
                            base: Value::Path(vec![PathPart::Column("a".to_string())]),
                            compositions: vec![],
                        }),
                        operator: Operator::Eq,
                        right: ComparisonPart::Expression(Expression {
                            base: Value::Literal(Literal::Number("8".to_string())),
                            compositions: vec![],
                        })
                    })]
                },
                column_layout: ColumnLayout {
                    column_specs: vec![ColumnSpec {
                        column_control: ColumnControl::default(),
                        expression: Expression {
                            base: Value::Path(vec![PathPart::Column("foo".to_string())]),
                            compositions: vec![],
                        },
                        alias: None,
                    }]
                }
            })
        );
    }
}
