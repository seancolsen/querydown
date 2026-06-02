use chumsky::{prelude::*, text::*};

use crate::ast::*;
use crate::parser::utils::*;
use crate::tokens::*;

use super::condition_set::condition_set;

pub fn path<'src>(expr: impl Psr<'src, Expr>) -> impl Psr<'src, Vec<PathPart>> {
    path_part(expr.clone())
        .then(
            whitespace()
                .then(just(PATH_SEPARATOR))
                .ignore_then(path_part(expr))
                .repeated()
                .collect::<Vec<PathPart>>(),
        )
        .map(|(head, rest)| {
            let mut parts = vec![head];
            parts.extend(rest);
            parts
        })
}

pub fn path_to_one<'src>() -> impl Psr<'src, Vec<PathPart>> {
    path_part_to_one()
        .then(
            whitespace()
                .then(just(PATH_SEPARATOR))
                .ignore_then(path_part_to_one())
                .repeated()
                .collect::<Vec<PathPart>>(),
        )
        .map(|(head, rest)| {
            let mut parts = vec![head];
            parts.extend(rest);
            parts
        })
}

fn path_part<'src>(expr: impl Psr<'src, Expr>) -> impl Psr<'src, PathPart> {
    choice((
        db_identifier().map(PathPart::Column),
        table_with_many(expr).map(PathPart::TableWithMany),
        table_with_one().map(PathPart::TableWithOne),
    ))
}

fn path_part_to_one<'src>() -> impl Psr<'src, PathPart> {
    choice((
        db_identifier().map(PathPart::Column),
        table_with_one().map(PathPart::TableWithOne),
    ))
}

fn table_with_one<'src>() -> impl Psr<'src, String> {
    exactly(PATH_TO_TABLE_WITH_ONE_PREFIX).ignore_then(db_identifier())
}

fn table_with_many<'src>(expr: impl Psr<'src, Expr>) -> impl Psr<'src, TableWithMany> {
    let column = db_identifier().delimited_by(
        just(TABLE_WITH_MANY_COLUMN_BRACE_L).then(whitespace()),
        whitespace().then(just(TABLE_WITH_MANY_COLUMN_BRACE_R)),
    );
    just(TABLE_SIGIL).ignore_then(
        db_identifier()
            .then(column.or_not())
            .then(condition_set(expr).or_not())
            .map(|((table, column), cs)| TableWithMany {
                table,
                condition_set: cs.unwrap_or_default(),
                linking_column: column,
            }),
    )
}

#[cfg(test)]
mod tests {
    use crate::parser::utils::*;

    use super::*;

    /// A mock expression parser that will never succeed to parse any input. This okay because we
    /// don't test cases that require parsing expressions within paths. Testing for paths which
    /// contain expressions is done at a higher level.
    fn incapable_expression_parser<'src>() -> impl Psr<'src, Expr> {
        exactly("NOPE").map(|_| Expr::Variable("nope".to_string()))
    }

    fn simple_path<'src>() -> impl Psr<'src, Vec<PathPart>> {
        path(incapable_expression_parser()).then_ignore(end())
    }

    #[test]
    fn test_parse_path() {
        let p = |s: &str| simple_path().parse(s).into_result().map_err(|_| ());
        assert_eq!(p("foo"), Ok(vec![PathPart::Column("foo".to_string())]));
        assert_eq!(
            p("foo.bar"),
            Ok(vec![
                PathPart::Column("foo".to_string()),
                PathPart::Column("bar".to_string()),
            ])
        );
        assert_eq!(
            p("#foo"),
            Ok(vec![PathPart::TableWithMany(TableWithMany {
                table: "foo".to_string(),
                linking_column: None,
                condition_set: ConditionSet::default(),
            })])
        );
        assert_eq!(
            p("#foo(bar)"),
            Ok(vec![PathPart::TableWithMany(TableWithMany {
                table: "foo".to_string(),
                linking_column: Some("bar".to_string()),
                condition_set: ConditionSet::default(),
            })])
        );
        assert_eq!(
            p(">>clients.start_date"),
            Ok(vec![
                PathPart::TableWithOne("clients".to_string()),
                PathPart::Column("start_date".to_string()),
            ])
        );
        assert_eq!(
            p("foo.bar.#baz(a).#bat.>>spam.eggs"),
            Ok(vec![
                PathPart::Column("foo".to_string()),
                PathPart::Column("bar".to_string()),
                PathPart::TableWithMany(TableWithMany {
                    table: "baz".to_string(),
                    linking_column: Some("a".to_string()),
                    condition_set: ConditionSet::default(),
                }),
                PathPart::TableWithMany(TableWithMany {
                    table: "bat".to_string(),
                    linking_column: None,
                    condition_set: ConditionSet::default(),
                }),
                PathPart::TableWithOne("spam".to_string()),
                PathPart::Column("eggs".to_string()),
            ])
        );

        assert!(p(".foo").is_err(),);
        assert!(p(".foo#bar").is_err(),);
        assert!(p(".foo>>bar").is_err(),);
        assert!(p(".foo..bar").is_err(),);
        assert!(p("foo. bar").is_err(),);
        assert!(p("foo. #bar").is_err(),);
        assert!(p("foo(bar)").is_err(),);
        assert!(p("foo.bar(baz)").is_err(),);
    }
}
