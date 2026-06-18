use chumsky::prelude::*;

use crate::ast::*;
use crate::tokens::*;

use super::utils::*;
use super::{column_layout::result_columns, expr::expr, sorting::sorting};

pub fn query<'src>() -> impl Psr<'src, Query> {
    let base_table = just(TABLE_SIGIL).ignore_then(db_identifier());
    let transformations = transformation()
        .separated_by(pad().then(exactly(TRANSFORMATION_DELIMITER)).then(pad()))
        .collect::<Vec<Transformation>>();
    pad().ignore_then(
        base_table
            .then_ignore(pad())
            .then(transformations)
            .then_ignore(pad().then(end()))
            .map(|(base_table, transformations)| Query {
                base_table,
                transformations,
            }),
    )
}

fn transformation<'src>() -> impl Psr<'src, Transformation> {
    top_level_condition_set()
        .then_ignore(pad())
        .then(sorting())
        .then_ignore(pad())
        .then(result_columns().or_not())
        .map(|((conditions, sorting), cl)| Transformation {
            conditions,
            sorting,
            result_columns: cl.unwrap_or_default(),
        })
}

fn top_level_condition_set<'src>() -> impl Psr<'src, ConditionSet> {
    expr()
        .padded_by(pad())
        .repeated()
        .collect::<Vec<Expr>>()
        .map(|entries| ConditionSet {
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
            query().parse("#foo a:1 b:2 $c").into_result(),
            Ok(Query {
                base_table: "foo".to_string(),
                transformations: vec![Transformation {
                    sorting: vec![],
                    conditions: ConditionSet {
                        conjunction: Conjunction::And,
                        entries: vec![
                            Expr::Comparison(Box::new(Comparison {
                                left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column(
                                    "a".to_string()
                                )])),
                                operator: Operator::Match,
                                right: ComparisonSide::Expr(Expr::Number("1".to_string())),
                            })),
                            Expr::Comparison(Box::new(Comparison {
                                left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column(
                                    "b".to_string()
                                )])),
                                operator: Operator::Match,
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
                        expr: Expr::Path(vec![PathPart::Column("c".to_string())]),
                        annotation: None,
                    })],
                }],
            })
        );
    }

    #[test]
    fn test_parse_query_with_comments() {
        // Comments are permitted anywhere whitespace is, and are not propagated into the AST, so a
        // query peppered with comments parses identically to the same query without them.
        let commented = query()
            .parse("#foo // pick rows\n a:1 /* nested /* block */ comment */ b:2 $c")
            .into_result();
        let plain = query().parse("#foo a:1 b:2 $c").into_result();
        assert!(commented.is_ok());
        assert_eq!(commented, plain);
    }
}
