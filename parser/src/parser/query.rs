use chumsky::{prelude::*, text::*};

use crate::ast::*;
use crate::tokens::*;

use super::utils::*;
use super::{column_layout::result_columns, expr::expr};

pub fn query() -> impl Psr<Query> {
    let base_table = just(TABLE_SIGIL).ignore_then(db_identifier());
    let transformations = transformation().separated_by(
        whitespace()
            .then(exactly(TRANSFORMATION_DELIMITER))
            .then(whitespace()),
    );
    whitespace().ignore_then(
        base_table
            .then_ignore(whitespace())
            .then(transformations)
            .then_ignore(whitespace().then(end()))
            .map(|(base_table, transformations)| Query {
                base_table,
                transformations,
            }),
    )
}

fn transformation() -> impl Psr<Transformation> {
    top_level_condition_set()
        .then_ignore(whitespace())
        .then(result_columns().or_not())
        .map(|(conditions, cl)| Transformation {
            conditions,
            result_columns: cl.unwrap_or_default(),
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
                                left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column(
                                    "a".to_string()
                                )])),
                                operator: Operator::Eq,
                                right: ComparisonSide::Expr(Expr::Number("1".to_string())),
                            })),
                            Expr::Comparison(Box::new(Comparison {
                                left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column(
                                    "b".to_string()
                                )])),
                                operator: Operator::Eq,
                                right: ComparisonSide::Expr(Expr::Number("2".to_string())),
                            })),
                        ],
                    },
                    result_columns: vec![ResultColumnStatement::Spec(ColumnSpec {
                        alias: None,
                        column_control: ColumnControl {
                            sort: None,
                            group: None,
                            is_partition_by: false,
                            is_hidden: false
                        },
                        expr: Expr::Path(vec![PathPart::Column("c".to_string())])
                    })],
                }],
            })
        );
    }
}
