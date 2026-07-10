use chumsky::{prelude::*, text::*};

use crate::ast::*;
use crate::tokens::*;

use super::annotation::annotation;
use super::expr::{expr, path, path_to_one};
use super::nesting::prefix_leading_path;
use super::utils::*;

pub fn result_columns<'src>() -> impl Psr<'src, Vec<ResultColumnStatement>> {
    result_column_statement()
        .then_ignore(pad())
        .repeated()
        .collect::<Vec<Vec<ResultColumnStatement>>>()
        .map(|groups| groups.into_iter().flatten().collect())
}

/// Parses one `$`-prefixed result column statement. A plain spec or glob yields a single
/// statement, but a nesting group like `$issue.( $id $title )` desugars into one statement per
/// nested entry (`$issue.id $issue.title`), so this yields a `Vec`.
fn result_column_statement<'src>() -> impl Psr<'src, Vec<ResultColumnStatement>> {
    recursive(|statement| {
        just(COLUMN_SPEC_PREFIX).then(pad()).ignore_then(choice((
            // `nested_result_columns` is tried before the other two because a nesting group and a
            // glob (and a plain spec) can all begin with the same path; they diverge only at the
            // `.(` that follows the nesting group's head.
            nested_result_columns(statement),
            column_glob().map(|glob| vec![ResultColumnStatement::Glob(glob)]),
            column_spec().map(|spec| vec![ResultColumnStatement::Spec(spec)]),
        )))
    })
}

/// Parses a result column nesting group, e.g. `$issue.( $id $title )`, desugaring it into the
/// equivalent flat statements (`$issue.id $issue.title`) by applying the head path to each nested
/// entry. Groups may be nested within groups; the heads compose.
fn nested_result_columns<'src>(
    statement: impl Psr<'src, Vec<ResultColumnStatement>> + 'src,
) -> impl Psr<'src, Vec<ResultColumnStatement>> {
    let entries = statement
        .then_ignore(pad())
        .repeated()
        .collect::<Vec<Vec<ResultColumnStatement>>>();
    path(expr())
        .then_ignore(pad().then(just(PATH_SEPARATOR)))
        .then(
            just(RESULT_COLUMNS_NESTING_BRACE_L)
                .then(pad())
                .ignore_then(entries)
                .then_ignore(just(RESULT_COLUMNS_NESTING_BRACE_R)),
        )
        .try_map(|(head, groups), span| {
            groups
                .into_iter()
                .flatten()
                .map(|statement| apply_path_prefix(&head, statement))
                .collect::<Result<Vec<ResultColumnStatement>, String>>()
                .map_err(|message| Rich::custom(span, message))
        })
}

/// Applies a nesting group's head path to one (already desugared) statement within the group: the
/// head is prepended to a glob's own head, or spliced onto the leading column reference of a
/// spec's expression.
fn apply_path_prefix(
    head: &[PathPart],
    statement: ResultColumnStatement,
) -> Result<ResultColumnStatement, String> {
    match statement {
        ResultColumnStatement::Spec(spec) => Ok(ResultColumnStatement::Spec(ColumnSpec {
            expr: prefix_leading_path(head, spec.expr)?,
            ..spec
        })),
        ResultColumnStatement::Glob(glob) => Ok(ResultColumnStatement::Glob(ColumnGlob {
            head: head.iter().cloned().chain(glob.head).collect(),
            specs: glob.specs,
        })),
    }
}

fn column_glob<'src>() -> impl Psr<'src, ColumnGlob> {
    let head = path_to_one()
        .then_ignore(just(PATH_SEPARATOR))
        .or_not()
        .map(|p| p.unwrap_or_default());

    let specs = column_spec()
        .padded_by(pad())
        .repeated()
        .collect::<Vec<ColumnSpec>>()
        .delimited_by(
            just(COLUMN_GLOB_ADJUSTMENT_BRACE_L),
            just(COLUMN_GLOB_ADJUSTMENT_BRACE_R),
        )
        .or_not()
        .map(|a| a.unwrap_or_default());

    head.then_ignore(just(COLUMN_GLOB))
        .then(specs)
        .map(|(head, specs)| ColumnGlob { head, specs })
}

fn column_spec<'src>() -> impl Psr<'src, ColumnSpec> {
    expr()
        .then(
            pad()
                .then(just(COLUMN_ALIAS_PREFIX))
                .then(pad())
                .ignore_then(db_identifier())
                .or_not(),
        )
        .then(
            pad()
                .ignore_then(
                    column_control()
                        .or_not()
                        .map(|v| v.unwrap_or(ColumnControl::default())),
                )
                .or_not(),
        )
        // Annotation must come last in the spec — after the sorting/grouping flags and after the
        // alias.
        .then(pad().ignore_then(annotation()).or_not())
        .map(|(((expr, alias), ctrl), annotation)| ColumnSpec {
            expr,
            alias,
            column_control: ctrl.unwrap_or_default(),
            annotation,
        })
}

pub(crate) fn column_control<'src>() -> impl Psr<'src, ColumnControl> {
    #[derive(Clone)]
    enum Flag {
        Sort,
        Desc,
        Ordinal(u32),
        Group,
        NullsFirst,
        Hide,
        Partition,
    }
    enum Context {
        Sorting,
        Grouping,
        General,
    }
    let flag = choice((
        just(COLUMN_CONTROL_FLAG_SORT).to(Flag::Sort),
        just(COLUMN_CONTROL_FLAG_DESC).to(Flag::Desc),
        // TODO_ERR handle error if number is too large
        int(10).from_str().unwrapped().map(Flag::Ordinal),
        just(COLUMN_CONTROL_FLAG_GROUP).to(Flag::Group),
        just(COLUMN_CONTROL_FLAG_NULLS_FIRST).to(Flag::NullsFirst),
        just(COLUMN_CONTROL_FLAG_HIDE).to(Flag::Hide),
        just(COLUMN_CONTROL_FLAG_PARTITION).to(Flag::Partition),
    ));
    just(COLUMN_CONTROL_FLAGS_PREFIX).ignore_then(
        flag.repeated()
            .at_least(1)
            .collect::<Vec<Flag>>()
            .map(|flags| {
                let mut context = Context::General;
                let mut sort = false;
                let mut sort_ordinal: Option<u32> = None;
                let mut sort_direction = SortDirection::default();
                let mut sort_nulls = NullsSort::default();
                let mut group = false;
                let mut group_ordinal: Option<u32> = None;
                let mut partition = false;
                let mut hide = false;
                let mut handle_ordinal = |o: u32, c: &Context| match c {
                    Context::Sorting => sort_ordinal = Some(o),
                    Context::Grouping => group_ordinal = Some(o),
                    Context::General => {}
                };
                for flag in flags {
                    match flag {
                        Flag::Sort => {
                            sort = true;
                            context = Context::Sorting;
                        }
                        Flag::Desc => sort_direction = SortDirection::Desc,
                        Flag::Ordinal(o) => handle_ordinal(o, &context),
                        Flag::Group => {
                            group = true;
                            context = Context::Grouping;
                        }
                        Flag::NullsFirst => sort_nulls = NullsSort::First,
                        Flag::Hide => hide = true,
                        Flag::Partition => partition = true,
                    }
                }
                ColumnControl {
                    sort: if sort {
                        Some(SortSpec {
                            ordinal: sort_ordinal,
                            direction: sort_direction,
                            nulls_sort: sort_nulls,
                        })
                    } else {
                        None
                    },
                    group: if group {
                        Some(GroupSpec {
                            ordinal: group_ordinal,
                        })
                    } else {
                        None
                    },
                    is_partition_by: partition,
                    is_hidden: hide,
                }
            }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_column_control() {
        assert_eq!(
            column_control().parse(r"\s1d").into_result(),
            Ok(ColumnControl {
                sort: Some(SortSpec {
                    ordinal: Some(1),
                    direction: SortDirection::Desc,
                    nulls_sort: NullsSort::default(),
                }),
                group: None,
                is_partition_by: false,
                is_hidden: false,
            })
        );
    }

    #[test]
    fn test_parse_column_spec() {
        assert_eq!(
            column_spec().parse("8").into_result(),
            Ok(ColumnSpec {
                column_control: ColumnControl::default(),
                expr: Expr::Number("8".to_string()),
                alias: None,
                annotation: None,
            })
        );
        assert_eq!(
            column_spec().parse(r"foo->bar\s1d").into_result(),
            Ok(ColumnSpec {
                column_control: ColumnControl {
                    sort: Some(SortSpec {
                        ordinal: Some(1),
                        direction: SortDirection::Desc,
                        nulls_sort: NullsSort::default(),
                    }),
                    group: None,
                    is_partition_by: false,
                    is_hidden: false,
                },
                expr: Expr::Path(vec![PathPart::Column("foo".to_string())]),
                alias: Some("bar".to_string()),
                annotation: None,
            })
        );
    }

    #[test]
    fn test_parse_column_spec_with_annotation() {
        // Annotation is parsed after the alias and after the column control flags.
        assert_eq!(
            column_spec()
                .then_ignore(end())
                .parse(r"foo->bar\sd @{width:100}")
                .into_result(),
            Ok(ColumnSpec {
                column_control: ColumnControl {
                    sort: Some(SortSpec {
                        ordinal: None,
                        direction: SortDirection::Desc,
                        nulls_sort: NullsSort::default(),
                    }),
                    group: None,
                    is_partition_by: false,
                    is_hidden: false,
                },
                expr: Expr::Path(vec![PathPart::Column("foo".to_string())]),
                alias: Some("bar".to_string()),
                annotation: Some(AnnotationValue::Object(vec![(
                    "width".to_string(),
                    AnnotationValue::Number("100".to_string())
                )])),
            })
        );
    }

    #[test]
    fn test_annotation_must_come_last() {
        // Annotation before the column control flags is not allowed.
        assert!(column_spec()
            .then_ignore(end())
            .parse(r"foo @{width:100}\sd")
            .has_errors());
    }

    /// Parses a full result columns section, requiring that all input is consumed. Errors are
    /// discarded because these tests only compare desugared output across equivalent inputs.
    fn parse_result_columns(source: &str) -> Result<Vec<ResultColumnStatement>, ()> {
        result_columns()
            .then_ignore(end())
            .parse(source)
            .into_result()
            .map_err(|_| ())
    }

    #[test]
    fn test_result_column_nesting_desugars_to_flat_form() {
        // A nesting group is exactly equivalent to writing the head before each nested entry.
        // Aliases, control flags, annotations, and comments all ride along unchanged.
        let nested = parse_result_columns(
            r"$id @{hide:yes} // the comment id
            $created_at
            $issue.(
              $id @{hide:yes} // the issue id
              $title @{width:400}
              $#labels.name%list(\\name)
              $project.name
            )",
        );
        let flat = parse_result_columns(
            r"$id @{hide:yes} // the comment id
            $created_at
            $issue.id @{hide:yes} // the issue id
            $issue.title @{width:400}
            $issue.#labels.name%list(\\name)
            $issue.project.name",
        );
        assert!(nested.is_ok());
        assert_eq!(nested, flat);
    }

    #[test]
    fn test_result_column_nesting_equivalences() {
        let eq = |nested: &str, flat: &str| {
            let nested_result = parse_result_columns(nested);
            assert!(nested_result.is_ok(), "failed to parse: {nested}");
            assert_eq!(nested_result, parse_result_columns(flat), "for: {nested}");
        };
        // A multi-part head, and a group containing a single entry.
        eq("$a.b.( $c )", "$a.b.c");
        // Groups nest; the heads compose.
        eq("$a.( $b.( $c ) $d )", "$a.b.c $a.d");
        // A glob inside a group has the head prepended to its own head.
        eq("$author.( $* )", "$author.*");
        eq(r"$author.( $*(id \h) )", r"$author.*(id \h)");
        // The head is spliced onto the piped-in value of a function application.
        eq("$issue.( $title|upper )", "$issue.title|upper");
        // ... and onto the left side of arithmetic and comparisons, through a `!` prefix, and onto
        // a `++`/`--` quantity path.
        eq("$issue.( $id * 2 )", "$issue.id * 2");
        eq("$issue.( $status:done )", "$issue.status:done");
        eq("$issue.( $!is_open )", "$!issue.is_open");
        eq("$issue.( $++#labels )", "$++issue.#labels");
        // A head may traverse to-many paths.
        eq(
            "$#comments.( $created_at%max )",
            "$#comments.created_at%max",
        );
        // Sorting and hiding flags ride along.
        eq(
            r"$issue.( $id \h $created_at \sd )",
            r"$issue.id \h $issue.created_at \sd",
        );
        // An empty group contributes no columns.
        assert_eq!(
            parse_result_columns("$issue.( ) $id"),
            parse_result_columns("$id")
        );
    }

    #[test]
    fn test_result_column_nesting_requires_leading_column_reference() {
        // A nested entry whose expression has no leading column reference cannot receive the head
        // path, so it is a parse error.
        assert!(parse_result_columns("$issue.( $8 )").is_err());
        assert!(parse_result_columns("$issue.( $'foo' )").is_err());
        assert!(parse_result_columns("$issue.( $%count )").is_err());
        assert!(parse_result_columns("$issue.( $@@now() )").is_err());
        // Even when other entries are fine.
        assert!(parse_result_columns("$issue.( $id $8 )").is_err());
    }

    #[test]
    fn test_result_column_nesting_does_not_break_globs() {
        // A glob with a head path still parses as a glob, not a nesting group.
        assert_eq!(
            parse_result_columns(r"$a.b.*(c \h)"),
            Ok(vec![ResultColumnStatement::Glob(ColumnGlob {
                head: vec![
                    PathPart::Column("a".to_string()),
                    PathPart::Column("b".to_string()),
                ],
                specs: vec![ColumnSpec {
                    column_control: ColumnControl {
                        sort: None,
                        group: None,
                        is_partition_by: false,
                        is_hidden: true,
                    },
                    expr: Expr::Path(vec![PathPart::Column("c".to_string())]),
                    alias: None,
                    annotation: None,
                }],
            })])
        );
    }

    #[test]
    fn test_parse_result_columns() {
        assert_eq!(
            result_columns()
                .parse(r"$* $a.b.*(c \h d\s) $foo $bar->B \g")
                .into_result(),
            Ok(vec![
                ResultColumnStatement::Glob(ColumnGlob::default()),
                ResultColumnStatement::Glob(ColumnGlob {
                    head: vec![
                        PathPart::Column("a".to_string()),
                        PathPart::Column("b".to_string()),
                    ],
                    specs: vec![
                        ColumnSpec {
                            column_control: ColumnControl {
                                sort: None,
                                group: None,
                                is_partition_by: false,
                                is_hidden: true,
                            },
                            expr: Expr::Path(vec![PathPart::Column("c".to_string())]),
                            alias: None,
                            annotation: None,
                        },
                        ColumnSpec {
                            column_control: ColumnControl {
                                sort: Some(SortSpec {
                                    ordinal: None,
                                    direction: SortDirection::Asc,
                                    nulls_sort: NullsSort::default(),
                                }),
                                group: None,
                                is_partition_by: false,
                                is_hidden: false,
                            },
                            expr: Expr::Path(vec![PathPart::Column("d".to_string())]),
                            alias: None,
                            annotation: None,
                        },
                    ]
                }),
                ResultColumnStatement::Spec(ColumnSpec {
                    column_control: ColumnControl::default(),
                    expr: Expr::Path(vec![PathPart::Column("foo".to_string())]),
                    alias: None,
                    annotation: None,
                }),
                ResultColumnStatement::Spec(ColumnSpec {
                    column_control: ColumnControl {
                        sort: None,
                        group: Some(GroupSpec { ordinal: None }),
                        is_partition_by: false,
                        is_hidden: false,
                    },
                    expr: Expr::Path(vec![PathPart::Column("bar".to_string())]),
                    alias: Some("B".to_string()),
                    annotation: None,
                }),
            ])
        );
    }
}
