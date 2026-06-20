use chumsky::{prelude::*, text::*};

use crate::ast::*;
use crate::tokens::*;

use super::utils::*;
use super::{
    column_layout::result_columns, expr::comparison_operator, expr::condition_entry, expr::expr,
    sorting::sorting,
};

/// A pre-base-table definition. Several kinds of definition share the same position in the source
/// (before the base table) and are parsed together, then sorted into the [`Query`]'s fields.
enum Definition {
    Constant(ConstantDef),
    Function(FunctionDef),
    CustomComparison(CustomComparisonDef),
    ComputedColumn(ComputedColumn),
    Table(TableDef),
}

pub fn query<'src>() -> impl Psr<'src, Query> {
    pad().ignore_then(query_body().then_ignore(pad().then(end())))
}

/// Parses the body of a query — its definitions, base table, and transformations — into a [`Query`],
/// without requiring that the whole input be consumed.
///
/// This is wrapped in [`recursive`] so that a `#( ... )` subquery, which may appear within a constant
/// or user-defined table definition, can itself contain a complete query (including further
/// subqueries). The recursive handle is threaded into the definition parsers that accept subqueries.
fn query_body<'src>() -> impl Psr<'src, Query> {
    recursive(|query| {
        let base_table = just(TABLE_SIGIL).ignore_then(db_identifier());
        let transformations = transformation()
            .separated_by(pad().then(exactly(TRANSFORMATION_DELIMITER)).then(pad()))
            .collect::<Vec<Transformation>>();
        definitions_with(query)
            .then(base_table)
            .then_ignore(pad())
            .then(transformations)
            .map(|((definitions, base_table), transformations)| Query {
                definitions,
                base_table,
                transformations,
            })
    })
}

/// Parses the run of definitions (constants, functions, custom comparisons, computed columns, and
/// user-defined tables) that may precede the base table, sorting them by kind into a
/// [`Definitions`]. Yields an empty [`Definitions`] when none are present.
///
/// This is exposed so that the definitions section of a query can be parsed in isolation, separate
/// from the base table and transformations. It builds its own [`query_body`] parser so that
/// subqueries within definitions can be parsed.
pub fn definitions<'src>() -> impl Psr<'src, Definitions> {
    definitions_with(query_body())
}

/// Parses the run of definitions, using `query` to parse any `#( ... )` subqueries that appear within
/// constant or user-defined table definitions.
fn definitions_with<'src>(query: impl Psr<'src, Query> + 'src) -> impl Psr<'src, Definitions> {
    definition(query)
        .then_ignore(pad())
        .repeated()
        .collect::<Vec<Definition>>()
        .map(|definitions| {
            let mut result = Definitions::default();
            for definition in definitions {
                match definition {
                    Definition::Constant(c) => result.constants.push(c),
                    Definition::Function(f) => result.functions.push(f),
                    Definition::CustomComparison(c) => result.custom_comparisons.push(c),
                    Definition::ComputedColumn(c) => result.computed_columns.push(c),
                    Definition::Table(t) => result.tables.push(t),
                }
            }
            result
        })
}

/// Parses any of the definitions that may precede the base table.
fn definition<'src>(query: impl Psr<'src, Query> + 'src) -> impl Psr<'src, Definition> {
    choice((
        // `function` is tried before `constant` because both begin with `@`; the `@@` prefix of a
        // function would otherwise be misread as the start of a constant.
        function().map(Definition::Function),
        constant(query.clone()).map(Definition::Constant),
        // `table` is tried before `custom_comparison` and `computed_column` because all three begin
        // with `#name`; a table definition is distinguished by the `= #(` that follows its name,
        // whereas the other two have a `.` before the `=`.
        table_def(query).map(Definition::Table),
        custom_comparison().map(Definition::CustomComparison),
        computed_column().map(Definition::ComputedColumn),
    ))
}

/// Parses one user-defined constant definition, e.g. `@user_id = 1234`. The right-hand side is
/// either a `#( ... )` subquery or a single expression, whose value gets inlined wherever the
/// constant is referenced.
fn constant<'src>(query: impl Psr<'src, Query> + 'src) -> impl Psr<'src, ConstantDef> {
    let value = choice((subquery(query), expr()));
    just(CONST_SIGIL)
        .ignore_then(ident().map(|s: &str| s.to_string()))
        .then_ignore(pad().then(just(DEFINITION_ASSIGN)).then(pad()))
        .then(value)
        .map(|(name, expr)| ConstantDef { name, expr })
}

/// Parses one user-defined table definition, e.g. `#project_months = #( #issues ... )`. The
/// right-hand side is a `#( ... )` subquery whose result becomes a CTE usable as a base table.
fn table_def<'src>(query: impl Psr<'src, Query> + 'src) -> impl Psr<'src, TableDef> {
    just(TABLE_SIGIL)
        .ignore_then(db_identifier())
        .then_ignore(pad().then(just(DEFINITION_ASSIGN)).then(pad()))
        .then(table_subquery(query))
        .map(|(name, query)| TableDef { name, query })
}

/// Parses a `#( query )` subquery into the [`Query`] it wraps.
fn table_subquery<'src>(query: impl Psr<'src, Query> + 'src) -> impl Psr<'src, Query> {
    just(TABLE_SIGIL)
        .ignore_then(just(EXPR_PAREN_L))
        .ignore_then(pad())
        .ignore_then(query)
        .then_ignore(pad())
        .then_ignore(just(EXPR_PAREN_R))
}

/// Parses a `#( query )` scalar subquery into an [`Expr::Subquery`].
fn subquery<'src>(query: impl Psr<'src, Query> + 'src) -> impl Psr<'src, Expr> {
    table_subquery(query).map(|q| Expr::Subquery(Box::new(q)))
}

/// Parses a variable reference's name, e.g. the `date` in `@date`.
fn variable_name<'src>() -> impl Psr<'src, String> {
    just(CONST_SIGIL).ignore_then(ident().map(|s: &str| s.to_string()))
}

/// Parses one user-defined function definition, e.g. `@@fiscal_year = @date => (@date - 1m)|year`.
/// The body is zero or more local assignments followed by a single result expression. Parameters
/// and assignments are referenced (within the body) as variables.
fn function<'src>() -> impl Psr<'src, FunctionDef> {
    let params = variable_name()
        .separated_by(pad())
        .at_least(1)
        .collect::<Vec<String>>();
    exactly(FUNCTION_SIGIL)
        .ignore_then(ident().map(|s: &str| s.to_string()))
        .then_ignore(pad().then(just(DEFINITION_ASSIGN)).then(pad()))
        .then(params)
        .then_ignore(pad().then(exactly(FUNCTION_ARROW)).then(pad()))
        .then(function_body())
        .map(|((name, params), body)| FunctionDef { name, params, body })
}

/// Parses a function body: zero or more local assignments followed by the result expression.
fn function_body<'src>() -> impl Psr<'src, FunctionBody> {
    assignment()
        .then_ignore(pad())
        .repeated()
        .collect::<Vec<Assignment>>()
        .then(expr())
        .map(|(assignments, expr)| FunctionBody { assignments, expr })
}

/// Parses a local assignment within a function body, e.g. `@birth_year = @birth_date|year`. An
/// assignment is distinguished from the body's result expression by the `=` that follows its name.
fn assignment<'src>() -> impl Psr<'src, Assignment> {
    variable_name()
        .then_ignore(pad())
        // Match a `=` that does not begin a `=>` arrow (defensive: arrows only appear in the
        // parameter list, never within the body).
        .then_ignore(just(DEFINITION_ASSIGN).then_ignore(just('>').not()))
        .then_ignore(pad())
        .then(expr())
        .map(|(name, expr)| Assignment { name, expr })
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

/// Parses one custom comparison definition, e.g. `#issues.comment:@x = ++#comments{body:@x}`. The
/// comparison operator (here `:`) sits between the name and the parameter, and the body — in which
/// the parameter may be referenced — follows the `=`.
fn custom_comparison<'src>() -> impl Psr<'src, CustomComparisonDef> {
    just(TABLE_SIGIL)
        .ignore_then(db_identifier())
        .then_ignore(just(PATH_SEPARATOR))
        .then(db_identifier())
        .then(comparison_operator())
        .then_ignore(pad())
        .then(variable_name())
        .then_ignore(pad().then(just(DEFINITION_ASSIGN)).then(pad()))
        .then(expr())
        .map(
            |((((table, name), operator), param), body)| CustomComparisonDef {
                table,
                name,
                operator,
                param,
                body,
            },
        )
}

fn transformation<'src>() -> impl Psr<'src, Transformation> {
    conditions()
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

/// Parses the top-level filtering section of a transformation: a whitespace-separated sequence of
/// boolean expressions, combined with `AND`, yielding a [`ConditionSet`]. Yields an empty
/// [`ConditionSet`] when no expressions are present.
///
/// This is exposed so that the filtering section of a query can be parsed in isolation, separate
/// from sorting and display.
pub fn conditions<'src>() -> impl Psr<'src, ConditionSet> {
    // Each top-level filtering expression is a condition-set entry, so a lone bare word here is a
    // default text search term rather than a column reference (see `condition_entry`).
    condition_entry(expr())
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
                definitions: Definitions::default(),
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
        assert_eq!(result.definitions.computed_columns.len(), 2);
        assert_eq!(
            result.definitions.computed_columns[0].table,
            "users".to_string()
        );
        assert_eq!(
            result.definitions.computed_columns[0].name,
            "age".to_string()
        );
        assert_eq!(
            result.definitions.computed_columns[1].name,
            "can_purchase_alcohol".to_string()
        );
        assert_eq!(
            result.definitions.computed_columns[1].expr,
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
            result.definitions.constants,
            vec![ConstantDef {
                name: "user_id".to_string(),
                expr: Expr::Number("1234".to_string()),
            }]
        );
        assert!(result.definitions.computed_columns.is_empty());
    }

    #[test]
    fn test_parse_query_with_function() {
        // A function definition with a single parameter and no local assignments.
        let result = query()
            .parse("@@double = @x => @x * 2\n#issues $id|double")
            .into_result()
            .unwrap();
        assert_eq!(result.base_table, "issues".to_string());
        assert_eq!(result.definitions.functions.len(), 1);
        let func = &result.definitions.functions[0];
        assert_eq!(func.name, "double".to_string());
        assert_eq!(func.params, vec!["x".to_string()]);
        assert!(func.body.assignments.is_empty());
        assert_eq!(
            func.body.expr,
            Expr::Product(
                Box::new(Expr::Variable("x".to_string())),
                Box::new(Expr::Number("2".to_string())),
            )
        );
    }

    #[test]
    fn test_parse_query_with_function_containing_assignment() {
        // A function body may contain local assignments before the result expression.
        let result = query()
            .parse("@@f = @a @b =>\n  @s = @a + @b\n  @s * 2\n#issues $id|f(2 3)")
            .into_result()
            .unwrap();
        let func = &result.definitions.functions[0];
        assert_eq!(func.name, "f".to_string());
        assert_eq!(func.params, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(func.body.assignments.len(), 1);
        assert_eq!(func.body.assignments[0].name, "s".to_string());
        assert_eq!(
            func.body.assignments[0].expr,
            Expr::Sum(
                Box::new(Expr::Variable("a".to_string())),
                Box::new(Expr::Variable("b".to_string())),
            )
        );
        assert_eq!(
            func.body.expr,
            Expr::Product(
                Box::new(Expr::Variable("s".to_string())),
                Box::new(Expr::Number("2".to_string())),
            )
        );
    }

    #[test]
    fn test_parse_query_with_constant_from_subquery() {
        // A constant may be defined as a `#( ... )` scalar subquery. The right-hand side parses into
        // an `Expr::Subquery` wrapping the inner query.
        let result = query()
            .parse("@latest = #( #comments $created_at%max )\n#issues created_at:>@latest")
            .into_result()
            .unwrap();
        assert_eq!(result.base_table, "issues".to_string());
        assert_eq!(result.definitions.constants.len(), 1);
        let constant = &result.definitions.constants[0];
        assert_eq!(constant.name, "latest".to_string());
        let Expr::Subquery(inner) = &constant.expr else {
            panic!("expected a subquery, got {:?}", constant.expr);
        };
        assert_eq!(inner.base_table, "comments".to_string());
    }

    #[test]
    fn test_parse_query_with_user_defined_table() {
        // A user-defined table definition `#name = #( query )` parses into a `TableDef`, and the
        // name can then be used as the query's base table.
        let result = query()
            .parse("#project_months = #( #issues $project \\g $%count -> issue_count )\n#project_months $project")
            .into_result()
            .unwrap();
        assert_eq!(result.base_table, "project_months".to_string());
        assert_eq!(result.definitions.tables.len(), 1);
        let table = &result.definitions.tables[0];
        assert_eq!(table.name, "project_months".to_string());
        assert_eq!(table.query.base_table, "issues".to_string());
        // A table definition is not mistaken for a computed column or custom comparison.
        assert!(result.definitions.computed_columns.is_empty());
        assert!(result.definitions.custom_comparisons.is_empty());
    }

    #[test]
    fn test_table_definition_distinguished_from_computed_column() {
        // `#table.name = expr` is a computed column, whereas `#name = #( ... )` is a table; the two
        // are told apart by what follows the name.
        let result = query()
            .parse("#users.adult = birth_date|age|years|floor:>=21\n#users $adult")
            .into_result()
            .unwrap();
        assert_eq!(result.definitions.computed_columns.len(), 1);
        assert!(result.definitions.tables.is_empty());
    }

    #[test]
    fn test_parse_query_with_custom_comparison() {
        let result = query()
            .parse("#issues.comment:@x = ++#comments{body:@x}\n#issues comment:workaround")
            .into_result()
            .unwrap();
        assert_eq!(result.base_table, "issues".to_string());
        assert_eq!(result.definitions.custom_comparisons.len(), 1);
        let def = &result.definitions.custom_comparisons[0];
        assert_eq!(def.table, "issues".to_string());
        assert_eq!(def.name, "comment".to_string());
        assert_eq!(def.operator, Operator::Match);
        assert_eq!(def.param, "x".to_string());
        assert_eq!(
            def.body,
            Expr::HasQuantity(HasQuantity {
                quantity: Quantity::AtLeastOne,
                path_parts: vec![PathPart::TableWithMany(TableWithMany {
                    table: "comments".to_string(),
                    condition_set: ConditionSet {
                        conjunction: Conjunction::And,
                        entries: vec![Expr::Comparison(Box::new(Comparison {
                            left: ComparisonSide::Expr(Expr::Path(vec![PathPart::Column(
                                "body".to_string()
                            )])),
                            operator: Operator::Match,
                            right: ComparisonSide::Expr(Expr::Variable("x".to_string())),
                        }))]
                    },
                    linking_column: None,
                })]
            })
        );
    }
}
