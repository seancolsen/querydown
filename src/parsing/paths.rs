use chumsky::{prelude::*, text::*};

use crate::syntax_tree::*;
use crate::tokens::*;

use super::utils::*;
use super::values::db_identifier;

pub fn path(condition_set: impl QdParser<ConditionSet>) -> impl QdParser<Path> {
    let initial_path_part = choice((
        db_identifier().map(PathPart::LocalColumn),
        prefixed_link_to_many(condition_set.clone()).map(PathPart::LinkToMany),
        exactly(LINK_TO_ONE_VIA_TABLE_PREFIX)
            .then(whitespace())
            .ignore_then(db_identifier())
            .map(PathPart::LinkToOneViaTable),
    ));
    let subsequent_path_part = choice((
        db_identifier().map(PathPart::LocalColumn),
        link_to_one().map(PathPart::LinkToOneViaColumn),
        prefixed_link_to_many(condition_set).map(PathPart::LinkToMany),
    ));
    initial_path_part
        .chain(whitespace().ignore_then(subsequent_path_part).repeated())
        .map(|parts| Path { parts })
}

pub fn prefixed_link_to_many(
    condition_set: impl QdParser<ConditionSet>,
) -> impl QdParser<LinkToMany> {
    just(LINK_TO_MANY_PREFIX)
        .then(whitespace())
        .ignore_then(link_to_many(condition_set))
}

pub fn link_to_many(condition_set: impl QdParser<ConditionSet>) -> impl QdParser<LinkToMany> {
    let column = db_identifier().delimited_by(
        just(LINK_TO_MANY_COLUMN_L_BRACE).then(whitespace()),
        whitespace().then(just(LINK_TO_MANY_COLUMN_R_BRACE)),
    );
    db_identifier()
        .then_ignore(whitespace())
        .then(column.or_not())
        .then(condition_set.or_not())
        .map(|((table, column), cs)| LinkToMany {
            table,
            condition_set: cs.unwrap_or_default(),
            column,
        })
}

fn link_to_one() -> impl QdParser<String> {
    just(LINK_TO_ONE_VIA_COLUMN_PREFIX)
        .then(whitespace())
        .ignore_then(db_identifier())
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
                parts: vec![PathPart::LocalColumn("foo".to_string()),]
            })
        );
        assert_eq!(
            simple_path().parse("foo.bar"),
            Ok(Path {
                parts: vec![
                    PathPart::LocalColumn("foo".to_string()),
                    PathPart::LinkToOneViaColumn("bar".to_string()),
                ]
            })
        );
        assert_eq!(
            simple_path().parse("*foo"),
            Ok(Path {
                parts: vec![PathPart::LinkToMany(LinkToMany {
                    table: "foo".to_string(),
                    column: None,
                    condition_set: ConditionSet::default(),
                })]
            })
        );
        assert_eq!(
            simple_path().parse("*foo(bar)"),
            Ok(Path {
                parts: vec![PathPart::LinkToMany(LinkToMany {
                    table: "foo".to_string(),
                    column: Some("bar".to_string()),
                    condition_set: ConditionSet::default(),
                })]
            })
        );
        assert_eq!(
            simple_path().parse("foo.bar*baz(a)*bat.spam"),
            Ok(Path {
                parts: vec![
                    PathPart::LocalColumn("foo".to_string()),
                    PathPart::LinkToOneViaColumn("bar".to_string()),
                    PathPart::LinkToMany(LinkToMany {
                        table: "baz".to_string(),
                        column: Some("a".to_string()),
                        condition_set: ConditionSet::default(),
                    }),
                    PathPart::LinkToMany(LinkToMany {
                        table: "bat".to_string(),
                        column: None,
                        condition_set: ConditionSet::default(),
                    }),
                    PathPart::LinkToOneViaColumn("spam".to_string()),
                ]
            })
        );

        assert!(simple_path().parse(".foo").is_err(),);
        assert!(simple_path().parse("foo(bar)").is_err(),);
        assert!(simple_path().parse("foo.bar(baz)").is_err(),);
    }
}
