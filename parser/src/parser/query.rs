use chumsky::{prelude::*, text::*};

use crate::ast::*;
use crate::tokens::*;

use super::utils::*;
use super::{column_layout::column_layout, expr::expr};

pub fn query() -> impl Psr<Query> {
    let base_table = just(TABLE_SIGIL).ignore_then(db_identifier());
    let transformations = transformation().separated_by(
        whitespace()
            .then(exactly(TRANSFORMATION_DELIMITER))
            .then(whitespace()),
    );
    base_table
        .then_ignore(whitespace())
        .then(transformations)
        .then_ignore(whitespace().then(end()))
        .map(|(base_table, transformations)| Query {
            base_table,
            transformations,
        })
}

fn transformation() -> impl Psr<Transformation> {
    top_level_condition_set()
        .then_ignore(whitespace())
        .then(column_layout().or_not())
        .map(|(conditions, cl)| Transformation {
            conditions,
            column_layout: cl.unwrap_or_default(),
        })
}

fn top_level_condition_set() -> impl Psr<ConditionSet> {
    expr().padded().repeated().map(|entries| ConditionSet {
        conjunction: Conjunction::And,
        entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_query() {
        assert_eq!(
            query().parse("#foo a:1 b:2 $c"),
            Ok(Query {
                base_table: "foo".to_string(),
                transformations: vec![Transformation {
                    conditions: ConditionSet {
                        conjunction: Conjunction::And,
                        entries: vec![
                            Expr::Comparison(Box::new(Comparison {
                                left: Expr::Path(vec![PathPart::Column("a".to_string())]),
                                operator: Operator::Eq,
                                right: Expr::Number("1".to_string()),
                                is_expand_left: false,
                                is_expand_right: false,
                            })),
                            Expr::Comparison(Box::new(Comparison {
                                left: Expr::Path(vec![PathPart::Column("b".to_string())]),
                                operator: Operator::Eq,
                                right: Expr::Number("2".to_string()),
                                is_expand_left: false,
                                is_expand_right: false,
                            })),
                        ],
                    },
                    column_layout: ColumnLayout {
                        column_specs: vec![ColumnSpec {
                            alias: None,
                            column_control: ColumnControl {
                                sort: None,
                                group: None,
                                is_partition_by: false,
                                is_hidden: false
                            },
                            expr: Expr::Path(vec![PathPart::Column("c".to_string())])
                        }]
                    },
                }],
            })
        );
    }
}
