use chumsky::{prelude::*, text::*};

use crate::syntax_tree::*;
use crate::tokens::*;

use super::molecule::discerned_expression;
use super::utils::QdParser;
use super::values::db_identifier;

pub fn column_layout() -> impl QdParser<ColumnLayout> {
    column_spec()
        .then_ignore(whitespace())
        .repeated()
        .map(|column_specs| ColumnLayout { column_specs })
}

fn column_spec() -> impl QdParser<ColumnSpec> {
    just(COLUMN_SPEC_PREFIX)
        .then(whitespace())
        .ignore_then(discerned_expression())
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
        .map(|((expression, alias), ctrl)| ColumnSpec {
            expression,
            alias,
            column_control: ctrl.unwrap_or_default(),
        })
}

fn column_control() -> impl QdParser<ColumnControl> {
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
    fn test_column_control() {
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
    fn test_column_spec() {
        assert_eq!(
            column_spec().parse("$8"),
            Ok(ColumnSpec {
                column_control: ColumnControl::default(),
                expression: Expression {
                    base: Value::Literal(Literal::Number("8".to_string())),
                    compositions: vec![],
                },
                alias: None,
            })
        );
        assert_eq!(
            column_spec().parse(r"$foo->bar\s1d"),
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
                expression: Expression {
                    base: Value::Path(vec![PathPart::Column("foo".to_string())]),
                    compositions: vec![],
                },
                alias: Some("bar".to_string()),
            })
        );
    }

    #[test]
    fn test_column_layout() {
        assert_eq!(
            column_layout().parse(r"$foo $bar->B \g"),
            Ok(ColumnLayout {
                column_specs: vec![
                    ColumnSpec {
                        column_control: ColumnControl::default(),
                        expression: Expression {
                            base: Value::Path(vec![PathPart::Column("foo".to_string())]),
                            compositions: vec![],
                        },
                        alias: None,
                    },
                    ColumnSpec {
                        column_control: ColumnControl {
                            sort: None,
                            group: Some(GroupSpec { ordinal: None }),
                            is_partition_by: false,
                            is_hidden: false,
                        },
                        expression: Expression {
                            base: Value::Path(vec![PathPart::Column("bar".to_string())]),
                            compositions: vec![],
                        },
                        alias: Some("B".to_string()),
                    },
                ]
            })
        );
    }
}
