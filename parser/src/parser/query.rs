use chumsky::{prelude::*, text::*};

use crate::ast::*;
use crate::tokens::*;

use super::utils::*;
use super::{column_layout::result_columns, expr::expr, sorting::sorting};

/// A pre-base-table definition. Several kinds of definition share the same position in the source
/// (before the base table) and are parsed together, then sorted into the [`Query`]'s fields.
enum Definition {
    Constant(ConstantDef),
    ComputedColumn(ComputedColumn),
}

pub fn query<'src>() -> impl Psr<'src, Query> {
    let definitions = definition()
        .then_ignore(pad())
        .repeated()
        .collect::<Vec<Definition>>();
    let base_table = just(TABLE_SIGIL).ignore_then(db_identifier());
    let transformations = transformation()
        .separated_by(pad().then(exactly(TRANSFORMATION_DELIMITER)).then(pad()))
        .collect::<Vec<Transformation>>();
    pad().ignore_then(
        definitions
            .then(base_table)
            .then_ignore(pad())
            .then(transformations)
            .then_ignore(pad().then(end()))
            .map(|((definitions, base_table), transformations)| {
                let mut constants = vec![];
                let mut computed_columns = vec![];
                for definition in definitions {
                    match definition {
                        Definition::Constant(c) => constants.push(c),
                        Definition::ComputedColumn(c) => computed_columns.push(c),
                    }
                }
                Query {
                    constants,
                    computed_columns,
                    base_table,
                    transformations,
                }
            }),
    )
}

/// Parses any of the definitions that may precede the base table.
fn definition<'src>() -> impl Psr<'src, Definition> {
    choice((
        constant().map(Definition::Constant),
        computed_column().map(Definition::ComputedColumn),
    ))
}

/// Parses one user-defined constant definition, e.g. `@user_id = 1234`. The right-hand side is a
/// single expression whose value gets inlined wherever the constant is referenced.
fn constant<'src>() -> impl Psr<'src, ConstantDef> {
    just(CONST_SIGIL)
        .ignore_then(ident().map(|s: &str| s.to_string()))
        .then_ignore(pad().then(just(DEFINITION_ASSIGN)).then(pad()))
        .then(expr())
        .map(|(name, expr)| ConstantDef { name, expr })
}

/// Parses one computed column definition, e.g. `#users.age = birth_date|age|years|floor`. These are
/// only permitted before the base table — never within the query itself. The right-hand side is a
/// single expression which may itself reference earlier computed columns.
fn computed_column<'src>() -> impl Psr<'src, ComputedColumn> {
    just(TABLE_SIGIL)
        .ignore_then(db_identifier())
        .then_ignore(just(PATH_SEPARATOR))
        .then(db_identifier())
        .then_ignore(pad().then(just(DEFINITION_ASSIGN)).then(pad()))
        .then(expr())
        .map(|((table, name), expr)| ComputedColumn { table, name, expr })
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
                constants: vec![],
                computed_columns: vec![],
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

    #[test]
    fn test_parse_query_with_computed_columns() {
        // Computed column definitions precede the base table. Each defines a named expression scoped
        // to a table, and a later definition may reference an earlier one by name.
        let result = query()
            .parse("#users.age = birth_date|age|years|floor\n#users.can_purchase_alcohol = age:>=21\n#users $can_purchase_alcohol")
            .into_result()
            .unwrap();
        assert_eq!(result.base_table, "users".to_string());
        assert_eq!(result.computed_columns.len(), 2);
        assert_eq!(result.computed_columns[0].table, "users".to_string());
        assert_eq!(result.computed_columns[0].name, "age".to_string());
        assert_eq!(
            result.computed_columns[1].name,
            "can_purchase_alcohol".to_string()
        );
        assert_eq!(
            result.computed_columns[1].expr,
            Expr::Comparison(Box::new(Comparison {
                left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column("age".to_string())])),
                operator: Operator::Gte,
                right: ComparisonSide::Expr(Expr::Number("21".to_string())),
            }))
        );
    }

    #[test]
    fn test_parse_query_with_constants() {
        // Constant definitions precede the base table and bind a name to an expression.
        let result = query()
            .parse("@user_id = 1234\n#issues author:@user_id")
            .into_result()
            .unwrap();
        assert_eq!(result.base_table, "issues".to_string());
        assert_eq!(
            result.constants,
            vec![ConstantDef {
                name: "user_id".to_string(),
                expr: Expr::Number("1234".to_string()),
            }]
        );
        assert!(result.computed_columns.is_empty());
    }
}
