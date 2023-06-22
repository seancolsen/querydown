use chumsky::{prelude::*, text::*};

use crate::ast::*;
use crate::parser::utils::*;
use crate::tokens::*;

use super::condition_set::condition_set;

pub fn path(expr: impl Psr<Expr>) -> impl Psr<Vec<PathPart>> {
    path_part(expr.clone()).chain(
        whitespace()
            .then(just(PATH_SEPARATOR))
            .ignore_then(path_part(expr))
            .repeated(),
    )
}

fn path_part(expr: impl Psr<Expr>) -> impl Psr<PathPart> {
    choice((
        db_identifier().map(PathPart::Column),
        table_with_many(expr).map(PathPart::TableWithMany),
        table_with_one().map(PathPart::TableWithOne),
    ))
}

fn table_with_one() -> impl Psr<String> {
    exactly(PATH_TO_TABLE_WITH_ONE_PREFIX).ignore_then(db_identifier())
}

fn table_with_many(expr: impl Psr<Expr>) -> impl Psr<TableWithMany> {
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
    fn incapable_expression_parser() -> impl Psr<Expr> {
        exactly("NOPE").map(|_| Expr::Variable("nope".to_string()))
    }

    fn simple_path() -> impl Psr<Vec<PathPart>> {
        path(incapable_expression_parser()).then_ignore(end())
    }

    #[test]
    fn test_parse_path() {
        assert_eq!(
            simple_path().parse("foo"),
            Ok(vec![PathPart::Column("foo".to_string())])
        );
        assert_eq!(
            simple_path().parse("foo.bar"),
            Ok(vec![
                PathPart::Column("foo".to_string()),
                PathPart::Column("bar".to_string()),
            ])
        );
        assert_eq!(
            simple_path().parse("#foo"),
            Ok(vec![PathPart::TableWithMany(TableWithMany {
                table: "foo".to_string(),
                linking_column: None,
                condition_set: ConditionSet::default(),
            })])
        );
        assert_eq!(
            simple_path().parse("#foo(bar)"),
            Ok(vec![PathPart::TableWithMany(TableWithMany {
                table: "foo".to_string(),
                linking_column: Some("bar".to_string()),
                condition_set: ConditionSet::default(),
            })])
        );
        assert_eq!(
            simple_path().parse(">>clients.start_date"),
            Ok(vec![
                PathPart::TableWithOne("clients".to_string()),
                PathPart::Column("start_date".to_string()),
            ])
        );
        assert_eq!(
            simple_path().parse("foo.bar.#baz(a).#bat.>>spam.eggs"),
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
