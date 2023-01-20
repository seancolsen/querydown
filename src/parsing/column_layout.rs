use chumsky::{prelude::*, text::*};

use crate::syntax_tree::*;
use crate::tokens::*;

use super::expression_or_condition_set::discerned_expression;
use super::values::db_identifier;

pub fn column_layout() -> impl Parser<char, ColumnLayout, Error = Simple<char>> {
    column_spec()
        .then_ignore(whitespace())
        .repeated()
        .map(|column_specs| ColumnLayout { column_specs })
}

fn column_spec() -> impl Parser<char, ColumnSpec, Error = Simple<char>> {
    just(COLUMN_SPEC_PREFIX)
        .then(whitespace())
        .ignore_then(
            column_control()
                .or_not()
                .map(|v| v.unwrap_or(ColumnControl::default())),
        )
        .then_ignore(whitespace())
        .then(discerned_expression())
        .then(
            whitespace()
                .then(just(ALIAS_DELIMITER))
                .then(whitespace())
                .ignore_then(db_identifier())
                .or_not(),
        )
        .map(|((column_control, expression), alias)| ColumnSpec {
            column_control,
            expression,
            alias,
        })
}

fn column_control() -> impl Parser<char, ColumnControl, Error = Simple<char>> {
    #[derive(Debug, Clone)]
    enum Flag {
        Sort(SortSpec),
        Group,
        Hide,
        Partition,
    }
    let parse_flag = choice((
        just(COLUMN_CONTROL_FLAG_GROUP).to(Flag::Group),
        just(COLUMN_CONTROL_FLAG_HIDE).to(Flag::Hide),
        sort_spec().map(Flag::Sort),
        just(COLUMN_CONTROL_FLAG_PARTITION).to(Flag::Partition),
    ));
    parse_flag
        .repeated()
        .delimited_by(just(COLUMN_CONTROL_BRACE_L), just(COLUMN_CONTROL_BRACE_R))
        .map(|flags| {
            let mut sort = None;
            let mut is_group_by = false;
            let mut is_partition_by = false;
            let mut is_hidden = false;
            for flag in flags {
                match flag {
                    Flag::Sort(s) => sort = Some(s),
                    Flag::Group => is_group_by = true,
                    Flag::Hide => is_hidden = true,
                    Flag::Partition => is_partition_by = true,
                }
            }
            ColumnControl {
                sort,
                is_group_by,
                is_partition_by,
                is_hidden,
            }
        })
}

pub fn sort_spec() -> impl Parser<char, SortSpec, Error = Simple<char>> {
    #[derive(Debug, Clone)]
    enum Flag {
        Desc,
        NullsFirst,
        Ordinal(u32),
    }
    let parse_flag = choice((
        // TODO handle error if number is too large
        int(10).from_str().unwrapped().map(|v| Flag::Ordinal(v)),
        just(SORT_FLAG_DESC).to(Flag::Desc),
        just(SORT_FLAG_NULLS_FIRST).to(Flag::NullsFirst),
    ));
    let parse_flags = parse_flag
        .repeated()
        .delimited_by(just(SORT_FLAGS_BRACE_L), just(SORT_FLAGS_BRACE_R))
        .map(|flags| {
            let mut ordinal = None;
            let mut direction = SortDirection::Asc;
            let mut nulls_sort = NullsSort::default();
            for flag in flags {
                match flag {
                    Flag::Desc => direction = SortDirection::Desc,
                    Flag::NullsFirst => nulls_sort = NullsSort::First,
                    Flag::Ordinal(o) => ordinal = Some(o),
                }
            }
            SortSpec {
                ordinal,
                direction,
                nulls_sort,
            }
        });
    just(COLUMN_CONTROL_FLAG_SORT)
        .ignore_then(parse_flags.or_not())
        .map(|v| v.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_control() {
        assert_eq!(
            column_control().parse("[s(1d)]"),
            Ok(ColumnControl {
                sort: Some(SortSpec {
                    ordinal: Some(1),
                    direction: SortDirection::Desc,
                    nulls_sort: NullsSort::default(),
                }),
                is_group_by: false,
                is_partition_by: false,
                is_hidden: false,
            })
        );
    }

    #[test]
    fn test_column_spec() {
        assert_eq!(
            column_spec().parse("-8"),
            Ok(ColumnSpec {
                column_control: ColumnControl::default(),
                expression: Expression {
                    base: Value::Number("8".to_string()),
                    compositions: vec![],
                },
                alias: None,
            })
        );
        assert_eq!(
            column_spec().parse("- [s(1d)] foo: bar"),
            Ok(ColumnSpec {
                column_control: ColumnControl {
                    sort: Some(SortSpec {
                        ordinal: Some(1),
                        direction: SortDirection::Desc,
                        nulls_sort: NullsSort::default(),
                    }),
                    is_group_by: false,
                    is_partition_by: false,
                    is_hidden: false,
                },
                expression: Expression {
                    base: Value::Path(Path {
                        parts: vec![PathPart::LocalColumn("foo".to_string()),]
                    }),
                    compositions: vec![],
                },
                alias: Some("bar".to_string()),
            })
        );
    }

    #[test]
    fn test_column_layout() {
        assert_eq!(
            column_layout().parse("-foo -[g]bar: B"),
            Ok(ColumnLayout {
                column_specs: vec![
                    ColumnSpec {
                        column_control: ColumnControl::default(),
                        expression: Expression {
                            base: Value::Path(Path {
                                parts: vec![PathPart::LocalColumn("foo".to_string()),]
                            }),
                            compositions: vec![],
                        },
                        alias: None,
                    },
                    ColumnSpec {
                        column_control: ColumnControl {
                            sort: None,
                            is_group_by: true,
                            is_partition_by: false,
                            is_hidden: false,
                        },
                        expression: Expression {
                            base: Value::Path(Path {
                                parts: vec![PathPart::LocalColumn("bar".to_string()),]
                            }),
                            compositions: vec![],
                        },
                        alias: Some("B".to_string()),
                    },
                ]
            })
        );
    }
}
