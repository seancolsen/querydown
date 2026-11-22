use chumsky::{prelude::*, text::*};

use crate::ast::*;
use crate::highlight::TokenKind;
use crate::labels;
use crate::tokens::*;

use super::utils::*;
use super::{
    annotation::query_annotation, column_layout::result_columns, expr::comparison_operator,
    expr::condition_entry, expr::expr, sorting::sorting,
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
    pad()
        .ignore_then(query_body())
        // The query-level annotation must come last: after the final pipeline stage. It belongs to
        // the whole query, so `query_body` (which also parses subqueries) doesn't accept one.
        .then(
            pad()
                .ignore_then(query_annotation())
                .then_ignore(pad().then(after_query_annotation()))
                .or_not(),
        )
        .then_ignore(pad().then(end()))
        .map(|(query, annotation)| Query {
            annotation,
            ..query
        })
}

/// Reports anything (other than whitespace and comments) written after a query annotation. Without
/// this, the error would merely say that the end of the input was expected.
fn after_query_annotation<'src>() -> impl Psr<'src, ()> {
    any()
        .validate(|c: char, extra, emitter| {
            emitter.emit(Rich::custom(
                extra.span(),
                format!(
                    "Unexpected `{c}`. A query annotation must come at the very end of the query."
                ),
            ))
        })
        .then(any().repeated())
        .ignored()
        .or_not()
        .ignored()
}

/// Parses the body of a query — its definitions, base table, and transformations — into a [`Query`],
/// without requiring that the whole input be consumed.
///
/// This is wrapped in [`recursive`] so that a `#( ... )` subquery, which may appear within a constant
/// or user-defined table definition, can itself contain a complete query (including further
/// subqueries). The recursive handle is threaded into the definition parsers that accept subqueries.
fn query_body<'src>() -> impl Psr<'src, Query> {
    recursive(|query| {
        let base_table = with_loc(mark(
            TokenKind::BaseTable,
            just(TABLE_SIGIL).ignore_then(table_name()),
        ))
        .labelled(labels::BASE_TABLE);
        let union_delimiter = pad()
            .then(exactly(UNION_DELIMITER).labelled(labels::UNION_DELIMITER))
            .then(pad());
        let pipeline_delimiter = pad()
            .then(exactly(TRANSFORMATION_DELIMITER).labelled(labels::PIPELINE_DELIMITER))
            .then(pad());
        // Each query of a union has its own base table and a single stage.
        let union_member = base_table
            .clone()
            .then_ignore(pad())
            .then(transformation())
            .map(
                |((base_table, base_table_loc), transformation)| UnionMember {
                    base_table,
                    base_table_loc,
                    transformation,
                },
            );
        let unions = union_delimiter
            .clone()
            .ignore_then(union_member.clone())
            .repeated()
            .collect::<Vec<UnionMember>>();
        let later_stages = pipeline_delimiter
            .ignore_then(transformation())
            .repeated()
            .collect::<Vec<Transformation>>();
        // A union binds more tightly than a pipeline, so one can't follow a `~~~`. It is parsed
        // anyway so that it can be reported plainly, rather than as an unexpected `+`.
        let misplaced_union = union_delimiter
            .ignore_then(union_member)
            .validate(|_, extra, emitter| {
                emitter.emit(Rich::custom(
                    extra.span(),
                    "A union (`+++`) can't follow a pipeline stage (`~~~`), because the union is \
                     performed first. To union the result of a pipeline, define it as a table \
                     (`#name = #( ... )`) and use that as a base table.",
                ))
            })
            .repeated();
        definitions_with(query)
            .then(base_table)
            .then_ignore(pad())
            .then(transformation())
            .then(unions)
            .then(later_stages)
            .then_ignore(misplaced_union)
            .map(
                |((((definitions, (base_table, base_table_loc)), first), unions), later)| Query {
                    definitions,
                    base_table,
                    base_table_loc,
                    transformations: std::iter::once(first).chain(later).collect(),
                    unions,
                    annotation: None,
                },
            )
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
    .labelled(labels::DEFINITION)
}

/// Parses one user-defined constant definition, e.g. `@user_id = 1234`. The right-hand side is
/// either a `#( ... )` subquery or a single expression, whose value gets inlined wherever the
/// constant is referenced.
fn constant<'src>(query: impl Psr<'src, Query> + 'src) -> impl Psr<'src, ConstantDef> {
    let value = choice((subquery(query), expr())).labelled(labels::VALUE);
    just(CONST_SIGIL)
        .ignore_then(
            ident()
                .map(|s: &str| s.to_string())
                .labelled(labels::VARIABLE_NAME),
        )
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
    // A query annotation isn't allowed here, but one is parsed anyway so that it can be reported
    // plainly, rather than as a malformed sort expression (since `\\\` begins with `\\`).
    let misplaced_annotation = pad()
        .ignore_then(query_annotation())
        .validate(|_, extra, emitter| {
            emitter.emit(Rich::custom(
                extra.span(),
                "A query annotation can only end the whole query, not a subquery.",
            ))
        })
        .or_not();
    just(TABLE_SIGIL)
        .ignore_then(just(EXPR_PAREN_L))
        .ignore_then(pad())
        .ignore_then(query)
        .then_ignore(misplaced_annotation)
        .then_ignore(pad())
        .then_ignore(just(EXPR_PAREN_R))
}

/// Parses a `#( query )` scalar subquery into an [`ExprKind::Subquery`].
fn subquery<'src>(query: impl Psr<'src, Query> + 'src) -> impl Psr<'src, Expr> {
    located(table_subquery(query).map(|q| ExprKind::Subquery(Box::new(q))))
}

/// Parses the `#table` that begins a computed column or custom comparison definition, along with
/// its location.
fn definition_table<'src>() -> impl Psr<'src, (TableName, Loc)> {
    with_loc(just(TABLE_SIGIL).ignore_then(table_name()))
}

/// Parses a variable reference's name, e.g. the `date` in `@date`.
fn variable_name<'src>() -> impl Psr<'src, String> {
    just(CONST_SIGIL).ignore_then(
        ident()
            .map(|s: &str| s.to_string())
            .labelled(labels::VARIABLE_NAME),
    )
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
        .ignore_then(
            ident()
                .map(|s: &str| s.to_string())
                .labelled(labels::FUNCTION_NAME),
        )
        .then_ignore(pad().then(just(DEFINITION_ASSIGN)).then(pad()))
        .then(params.labelled(labels::FUNCTION_PARAMETER))
        .then_ignore(
            pad()
                .then(exactly(FUNCTION_ARROW).labelled(labels::FUNCTION_ARROW))
                .then(pad()),
        )
        .then(function_body())
        .map(|((name, params), body)| FunctionDef { name, params, body })
}

/// Parses a function body: zero or more local assignments followed by the result expression.
fn function_body<'src>() -> impl Psr<'src, FunctionBody> {
    assignment()
        .then_ignore(pad())
        .repeated()
        .collect::<Vec<Assignment>>()
        .then(expr().labelled(labels::VALUE))
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
    definition_table()
        .then_ignore(just(PATH_SEPARATOR))
        .then(db_identifier())
        .then_ignore(pad().then(just(DEFINITION_ASSIGN)).then(pad()))
        .then(expr())
        .map(|(((table, table_loc), name), expr)| ComputedColumn {
            table,
            table_loc,
            name,
            expr,
        })
}

/// Parses one custom comparison definition, e.g. `#issues.comment:@x = ++#comments{body:@x}`. The
/// comparison operator (here `:`) sits between the name and the parameter, and the body — in which
/// the parameter may be referenced — follows the `=`.
fn custom_comparison<'src>() -> impl Psr<'src, CustomComparisonDef> {
    definition_table()
        .then_ignore(just(PATH_SEPARATOR))
        .then(db_identifier())
        .then(comparison_operator())
        .then_ignore(pad())
        .then(variable_name())
        .then_ignore(pad().then(just(DEFINITION_ASSIGN)).then(pad()))
        .then(expr())
        .map(
            |(((((table, table_loc), name), operator), param), body)| CustomComparisonDef {
                table,
                table_loc,
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
        .labelled(labels::CONDITION)
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
                base_table_loc: Loc::NONE,
                definitions: Definitions::default(),
                base_table: "foo".into(),
                transformations: vec![Transformation {
                    sorting: vec![],
                    conditions: ConditionSet {
                        conjunction: Conjunction::And,
                        entries: vec![
                            ExprKind::Comparison(Box::new(Comparison {
                                left: ComparisonSide::Expr(
                                    ExprKind::Path(vec![
                                        PathPartKind::Column("a".to_string()).into()
                                    ])
                                    .into()
                                ),
                                operator: Operator::Match,
                                right: ComparisonSide::Expr(
                                    ExprKind::Number("1".to_string()).into()
                                ),
                            }))
                            .into(),
                            ExprKind::Comparison(Box::new(Comparison {
                                left: ComparisonSide::Expr(
                                    ExprKind::Path(vec![
                                        PathPartKind::Column("b".to_string()).into()
                                    ])
                                    .into()
                                ),
                                operator: Operator::Match,
                                right: ComparisonSide::Expr(
                                    ExprKind::Number("2".to_string()).into()
                                ),
                            }))
                            .into(),
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
                        expr: ExprKind::Path(vec![PathPartKind::Column("c".to_string()).into()])
                            .into(),
                        annotation: None,
                    })],
                }],
                unions: vec![],
                annotation: None,
            })
        );
    }

    fn query_annotation_of(source: &str) -> Option<AnnotationValue> {
        query().parse(source).into_result().unwrap().annotation
    }

    fn foo_100() -> Option<AnnotationValue> {
        Some(AnnotationValue::Object(vec![(
            "foo".to_string(),
            AnnotationValue::Number("100".to_string()),
        )]))
    }

    #[test]
    fn test_parse_query_annotation() {
        assert_eq!(query_annotation_of("#issues"), None);
        assert_eq!(query_annotation_of(r"#issues \\\{foo:100}"), foo_100());
        // Written as in standard JSON, so long as there are no commas.
        assert_eq!(query_annotation_of(r#"#issues \\\{"foo": 100}"#), foo_100());
        // After every other part of the query, and surrounded by whitespace and comments.
        assert_eq!(
            query_annotation_of(
                "#issues status:open \\\\created_at $title @{width:100} // columns\n  \\\\\\{foo:100} // the query\n"
            ),
            foo_100()
        );
        // After the last stage of a pipeline.
        assert_eq!(
            query_annotation_of(r"#issues $author \g ~~~ $author \\\{foo:100}"),
            foo_100()
        );
        // The rest of the query is unaffected.
        let with = query().parse(r"#issues $title \\\{foo:100}").into_result();
        let without = query().parse(r"#issues $title").into_result();
        assert_eq!(
            with.map(|q| Query {
                annotation: None,
                ..q
            }),
            without
        );
    }

    #[test]
    fn test_parse_nested_query_annotation() {
        assert_eq!(
            query_annotation_of(r"#issues \\\{a:[1 {b:@true}] c:'x y'}"),
            Some(AnnotationValue::Object(vec![
                (
                    "a".to_string(),
                    AnnotationValue::Array(vec![
                        AnnotationValue::Number("1".to_string()),
                        AnnotationValue::Object(vec![(
                            "b".to_string(),
                            AnnotationValue::Bool(true)
                        )]),
                    ])
                ),
                ("c".to_string(), AnnotationValue::String("x y".to_string())),
            ]))
        );
    }

    #[test]
    fn test_query_annotation_placement() {
        let fails = |source: &str| assert!(query().parse(source).has_errors(), "{source}");
        // It must come last.
        fails(r"#issues \\\{foo:100} $title");
        fails(r"#issues \\\{foo:100} ~~~ $title");
        // Only one is allowed.
        fails(r"#issues \\\{a:1} \\\{b:2}");
        // The top level must be an object.
        fails(r"#issues \\\[1]");
        fails(r"#issues \\\100");
        // The `{` follows the `\\\` directly.
        fails(r"#issues \\\ {foo:100}");
        // A subquery can't have one.
        fails(r"#t = #( #issues \\\{foo:100} ) #t");
        fails(r"@n = #( #issues $%count \\\{foo:100} ) #issues");
    }

    /// The base tables of a query's union, in order, plus how many stages its pipeline has.
    fn union_shape(source: &str) -> (Vec<String>, usize) {
        let query = query().parse(source).into_result().unwrap();
        let tables = std::iter::once(query.base_table)
            .chain(query.unions.into_iter().map(|member| member.base_table))
            .map(|table| table.to_string())
            .collect();
        (tables, query.transformations.len())
    }

    #[test]
    fn test_parse_union() {
        let tables = |names: &[&str]| names.iter().map(|n| n.to_string()).collect::<Vec<_>>();
        assert_eq!(union_shape("#a"), (tables(&["a"]), 1));
        assert_eq!(union_shape("#a +++ #b"), (tables(&["a", "b"]), 1));
        // With no whitespace, after a value that `+` could otherwise continue into a sum.
        assert_eq!(union_shape("#a $x:1+++#b $2"), (tables(&["a", "b"]), 1));
        assert_eq!(
            union_shape("#a x:1 $y \n+++\n#b $z\n+++\n#c $w ~~~ $* ~~~"),
            (tables(&["a", "b", "c"]), 3)
        );
        // The union's members are parsed like the first query.
        let parsed = query()
            .parse("@n = 1 #a x:@n $y +++ #b z:2 $w -> y")
            .into_result()
            .unwrap();
        assert_eq!(parsed.definitions.constants.len(), 1);
        let member = &parsed.unions[0];
        assert_eq!(member.transformation.conditions.entries.len(), 1);
        assert_eq!(member.transformation.result_columns.len(), 1);
        // Within a subquery.
        let parsed = query()
            .parse("#t = #( #a +++ #b ) #t")
            .into_result()
            .unwrap();
        assert_eq!(parsed.definitions.tables[0].query.unions.len(), 1);
        // A sum is still a sum.
        let sum = query().parse("#a $x+1 $x + 2").into_result().unwrap();
        assert!(sum.unions.is_empty());
        assert_eq!(sum.transformations[0].result_columns.len(), 2);
        // And a quantity is still a quantity.
        let has = query().parse("#a x:1 ++#b").into_result().unwrap();
        assert!(has.unions.is_empty());
        assert_eq!(has.transformations[0].conditions.entries.len(), 2);
    }

    #[test]
    fn test_parse_tables_in_other_schemas() {
        let schema = |names: &[&str]| names.iter().map(|n| n.to_string()).collect::<Vec<_>>();
        let parsed = query()
            .parse(
                "#/sales/orders.big = total:>100\n\
                 #/sales/orders.x:@v = id:@v\n\
                 #/sales/orders +++ #/archive/main/`old orders` ~~~ $#/a/b/c / 2",
            )
            .into_result()
            .unwrap();
        let orders = TableName::in_schema(schema(&["sales"]), "orders");
        assert_eq!(parsed.base_table, orders);
        assert_eq!(parsed.definitions.computed_columns[0].table, orders);
        assert_eq!(parsed.definitions.custom_comparisons[0].table, orders);
        assert_eq!(
            parsed.unions[0].base_table,
            TableName::in_schema(schema(&["archive", "main"]), "old orders")
        );
        assert_eq!(
            parsed.unions[0].base_table.to_string(),
            "/archive/main/old orders"
        );
        // A table in the current schema has no schema, and is equal to its name.
        let parsed = query().parse("#issues").into_result().unwrap();
        assert_eq!(parsed.base_table, TableName::new("issues"));
        assert_eq!(parsed.base_table, "issues");
        assert!(!parsed.base_table.has_schema());
        // A `/` after a table in the current schema is a division.
        let parsed = query().parse("#issues $#comments/2").into_result().unwrap();
        assert_eq!(parsed.base_table, "issues");
        assert!(matches!(
            parsed.transformations[0].result_columns[0],
            ResultColumnStatement::Spec(ColumnSpec {
                expr: Expr {
                    kind: ExprKind::Quotient(..),
                    ..
                },
                ..
            })
        ));
    }

    #[test]
    fn test_tables_in_other_schemas_need_a_schema() {
        let errors = query().parse("#/orders").into_errors();
        assert_eq!(errors.len(), 1);
        assert!(
            errors[0].to_string().contains("names no schema"),
            "{}",
            errors[0]
        );
        // A user-defined table can't be put in a schema.
        assert!(query().parse("#/s/t = #( #issues ) #/s/t").has_errors());
    }

    #[test]
    fn test_union_placement() {
        let fails = |source: &str| assert!(query().parse(source).has_errors(), "{source}");
        // Each query needs a base table.
        fails("#a +++");
        fails("#a +++ $x");
        // A union can't follow a pipeline stage.
        fails("#a ~~~ $x +++ #b");
        // Nor a query annotation.
        fails(r"#a \\\{x:1} +++ #b");
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
            ExprKind::Comparison(Box::new(Comparison {
                left: ComparisonSide::Expr(
                    ExprKind::Path(vec![PathPartKind::Column("age".to_string()).into()]).into()
                ),
                operator: Operator::Gte,
                right: ComparisonSide::Expr(ExprKind::Number("21".to_string()).into()),
            }))
            .into()
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
                expr: ExprKind::Number("1234".to_string()).into(),
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
            ExprKind::Product(
                Box::new(ExprKind::Variable("x".to_string()).into()),
                Box::new(ExprKind::Number("2".to_string()).into()),
            )
            .into()
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
            ExprKind::Sum(
                Box::new(ExprKind::Variable("a".to_string()).into()),
                Box::new(ExprKind::Variable("b".to_string()).into()),
            )
            .into()
        );
        assert_eq!(
            func.body.expr,
            ExprKind::Product(
                Box::new(ExprKind::Variable("s".to_string()).into()),
                Box::new(ExprKind::Number("2".to_string()).into()),
            )
            .into()
        );
    }

    #[test]
    fn test_parse_query_with_constant_from_subquery() {
        // A constant may be defined as a `#( ... )` scalar subquery. The right-hand side parses into
        // an `ExprKind::Subquery` wrapping the inner query.
        let result = query()
            .parse("@latest = #( #comments $created_at%max )\n#issues created_at:>@latest")
            .into_result()
            .unwrap();
        assert_eq!(result.base_table, "issues".to_string());
        assert_eq!(result.definitions.constants.len(), 1);
        let constant = &result.definitions.constants[0];
        assert_eq!(constant.name, "latest".to_string());
        let ExprKind::Subquery(inner) = &constant.expr.kind else {
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
            ExprKind::HasQuantity(HasQuantity {
                quantity: Quantity::AtLeastOne,
                path_parts: vec![PathPartKind::TableWithMany(TableWithMany {
                    table: "comments".into(),
                    condition_set: ConditionSet {
                        conjunction: Conjunction::And,
                        entries: vec![ExprKind::Comparison(Box::new(Comparison {
                            left: ComparisonSide::Expr(
                                ExprKind::Path(vec![
                                    PathPartKind::Column("body".to_string()).into()
                                ])
                                .into()
                            ),
                            operator: Operator::Match,
                            right: ComparisonSide::Expr(ExprKind::Variable("x".to_string()).into()),
                        }))
                        .into()]
                    },
                    linking_column: None,
                })
                .into()]
            })
            .into()
        );
    }
}
