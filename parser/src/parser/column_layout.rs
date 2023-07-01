use chumsky::{prelude::*, text::*};

use crate::ast::*;
use crate::tokens::*;

use super::expr::{expr, path_to_one};
use super::utils::*;

pub fn result_columns() -> impl Psr<Vec<ResultColumnStatement>> {
    result_column_statement()
        .then_ignore(whitespace())
        .repeated()
}

fn result_column_statement() -> impl Psr<ResultColumnStatement> {
    just(COLUMN_SPEC_PREFIX)
        .then(whitespace())
        .ignore_then(choice((
            column_glob().map(ResultColumnStatement::Glob),
            column_spec().map(ResultColumnStatement::Spec),
        )))
}

fn column_glob() -> impl Psr<ColumnGlob> {
    let head = path_to_one()
        .then_ignore(just(PATH_SEPARATOR))
        .or_not()
        .map(|p| p.unwrap_or_default());

    let specs = column_spec()
        .padded()
        .repeated()
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

fn column_spec() -> impl Psr<ColumnSpec> {
    expr()
        .then(
            whitespace()
                .then(just(COLUMN_ALIAS_PREFIX))
                .then(whitespace())
                .ignore_then(db_identifier())
                .or_not(),
        )
        .then(
            whitespace()
                .ignore_then(
                    column_control()
                        .or_not()
                        .map(|v| v.unwrap_or(ColumnControl::default())),
                )
                .or_not(),
        )
        .map(|((expr, alias), ctrl)| ColumnSpec {
            expr,
            alias,
            column_control: ctrl.unwrap_or_default(),
        })
}

fn column_control() -> impl Psr<ColumnControl> {
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
        int(10).from_str().unwrapped().map(|v| Flag::Ordinal(v)),
        just(COLUMN_CONTROL_FLAG_GROUP).to(Flag::Group),
        just(COLUMN_CONTROL_FLAG_NULLS_FIRST).to(Flag::NullsFirst),
        just(COLUMN_CONTROL_FLAG_HIDE).to(Flag::Hide),
        just(COLUMN_CONTROL_FLAG_PARTITION).to(Flag::Partition),
    ));
    just(COLUMN_CONTROL_FLAGS_PREFIX).ignore_then(flag.repeated().at_least(1).map(|flags| {
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
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_column_control() {
        assert_eq!(
            column_control().parse(r"\s1d"),
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
            column_spec().parse("8"),
            Ok(ColumnSpec {
                column_control: ColumnControl::default(),
                expr: Expr::Number("8".to_string()),
                alias: None,
            })
        );
        assert_eq!(
            column_spec().parse(r"foo->bar\s1d"),
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
            })
        );
    }

    #[test]
    fn test_parse_result_columns() {
        assert_eq!(
            result_columns().parse(r"$* $a.b.*(c \h d\s) $foo $bar->B \g"),
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
                        },
                    ]
                }),
                ResultColumnStatement::Spec(ColumnSpec {
                    column_control: ColumnControl::default(),
                    expr: Expr::Path(vec![PathPart::Column("foo".to_string())]),
                    alias: None,
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
                }),
            ])
        );
    }
}
