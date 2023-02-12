use chumsky::{prelude::*, text::*};

use crate::syntax_tree::*;
use crate::tokens::*;

use super::utils::*;
use super::values::db_identifier;

pub fn path(condition_set: impl QdParser<ConditionSet>) -> impl QdParser<Path> {
    path_part(condition_set.clone())
        .chain(
            whitespace()
                .then(just(PATH_SEPARATOR))
                .ignore_then(path_part(condition_set))
                .repeated(),
        )
        .map(|parts| Path { parts })
}

fn path_part(condition_set: impl QdParser<ConditionSet>) -> impl QdParser<PathPart> {
    choice((
        db_identifier().map(PathPart::Column),
        table_with_many(condition_set).map(PathPart::TableWithMany),
        table_with_one().map(PathPart::TableWithOne),
    ))
}

fn table_with_one() -> impl QdParser<String> {
    exactly(PATH_TO_TABLE_WITH_ONE_PREFIX).ignore_then(db_identifier())
}

fn table_with_many(condition_set: impl QdParser<ConditionSet>) -> impl QdParser<TableWithMany> {
    let column = db_identifier().delimited_by(
        just(TABLE_WITH_MANY_COLUMN_BRACE_L).then(whitespace()),
        whitespace().then(just(TABLE_WITH_MANY_COLUMN_BRACE_R)),
    );
    just(PATH_TO_TABLE_WITH_MANY_PREFIX)
        .then(whitespace())
        .ignore_then(
            db_identifier()
                .then(column.or_not())
                .then(condition_set.or_not())
                .map(|((table, column), cs)| TableWithMany {
                    table,
                    condition_set: cs.unwrap_or_default(),
                    column,
                }),
        )
}

#[cfg(test)]
mod tests {
    use crate::parsing::utils::exactly;

    use super::*;

    /// A mock condition set parser that will never succeed to parse any input. This okay because
    /// we don't test cases like this here. Testing for paths which contain condition sets is done
    /// at a higher level (see `test_discerned_expression`) because it requires parsing for
    /// expressions and condition_sets.
    fn incapable_condition_set_parser() -> impl QdParser<ConditionSet> {
        exactly("NOPE").map(|_| ConditionSet::default())
    }

    fn simple_path() -> impl QdParser<Path> {
        path(incapable_condition_set_parser()).then_ignore(end())
    }

    #[test]
    fn test_path() {
        assert_eq!(
            simple_path().parse("foo"),
            Ok(Path {
                parts: vec![PathPart::Column("foo".to_string()),]
            })
        );
        assert_eq!(
            simple_path().parse("foo.bar"),
            Ok(Path {
                parts: vec![
                    PathPart::Column("foo".to_string()),
                    PathPart::Column("bar".to_string()),
                ]
            })
        );
        assert_eq!(
            simple_path().parse("#foo"),
            Ok(Path {
                parts: vec![PathPart::TableWithMany(TableWithMany {
                    table: "foo".to_string(),
                    column: None,
                    condition_set: ConditionSet::default(),
                })]
            })
        );
        assert_eq!(
            simple_path().parse("#foo(bar)"),
            Ok(Path {
                parts: vec![PathPart::TableWithMany(TableWithMany {
                    table: "foo".to_string(),
                    column: Some("bar".to_string()),
                    condition_set: ConditionSet::default(),
                })]
            })
        );
        assert_eq!(
            simple_path().parse(">>clients.start_date"),
            Ok(Path {
                parts: vec![
                    PathPart::TableWithOne("clients".to_string()),
                    PathPart::Column("start_date".to_string())
                ]
            })
        );
        assert_eq!(
            simple_path().parse("foo.bar.#baz(a).#bat.>>spam.eggs"),
            Ok(Path {
                parts: vec![
                    PathPart::Column("foo".to_string()),
                    PathPart::Column("bar".to_string()),
                    PathPart::TableWithMany(TableWithMany {
                        table: "baz".to_string(),
                        column: Some("a".to_string()),
                        condition_set: ConditionSet::default(),
                    }),
                    PathPart::TableWithMany(TableWithMany {
                        table: "bat".to_string(),
                        column: None,
                        condition_set: ConditionSet::default(),
                    }),
                    PathPart::TableWithOne("spam".to_string()),
                    PathPart::Column("eggs".to_string()),
                ]
            })
        );

        assert!(simple_path().parse(".foo").is_err(),);
        assert!(simple_path().parse(".foo#bar").is_err(),);
        assert!(simple_path().parse(".foo>>bar").is_err(),);
        assert!(simple_path().parse(".foo..bar").is_err(),);
        assert!(simple_path().parse("foo. bar").is_err(),);
        assert!(simple_path().parse("foo. #bar").is_err(),);
        assert!(simple_path().parse("foo(bar)").is_err(),);
        assert!(simple_path().parse("foo.bar(baz)").is_err(),);
    }
}
