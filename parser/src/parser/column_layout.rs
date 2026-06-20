use chumsky::{prelude::*, text::*};

use crate::ast::*;
use crate::tokens::*;

use super::annotation::annotation;
use super::expr::{expr, path_to_one};
use super::utils::*;

pub fn result_columns<'src>() -> impl Psr<'src, Vec<ResultColumnStatement>> {
    result_column_statement()
        .then_ignore(pad())
        .repeated()
        .collect::<Vec<ResultColumnStatement>>()
}

fn result_column_statement<'src>() -> impl Psr<'src, ResultColumnStatement> {
    just(COLUMN_SPEC_PREFIX).then(pad()).ignore_then(choice((
        column_glob().map(ResultColumnStatement::Glob),
        column_spec().map(ResultColumnStatement::Spec),
    )))
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
