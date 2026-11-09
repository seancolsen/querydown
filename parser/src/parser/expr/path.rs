use chumsky::prelude::*;

use crate::ast::*;
use crate::highlight::TokenKind;
use crate::labels;
use crate::parser::utils::*;
use crate::tokens::*;

use super::condition_set::condition_set;

pub fn path<'src>(expr: impl Psr<'src, Expr>) -> impl Psr<'src, Vec<PathPart>> {
    path_part(expr.clone())
        .then(
            pad()
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
            pad()
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
    // (`fn` items rather than closures, for the reason given in `utils`.)
    fn table_with_many_part((table, loc): (TableWithMany, Loc)) -> (PathPartKind, Loc) {
        (PathPartKind::TableWithMany(table), loc)
    }
    fn part((kind, loc): (PathPartKind, Loc)) -> PathPart {
        PathPart::new(kind, loc)
    }
    choice((
        with_loc(column().map(PathPartKind::Column)),
        table_with_many(expr).map(table_with_many_part),
        with_loc(table_with_one().map(PathPartKind::TableWithOne)),
    ))
    .map(part)
}

fn path_part_to_one<'src>() -> impl Psr<'src, PathPart> {
    choice((
        column().map(PathPartKind::Column),
        table_with_one().map(PathPartKind::TableWithOne),
    ))
    .map_with(|kind, extra| PathPart::new(kind, loc(extra)))
}

fn column<'src>() -> impl Psr<'src, String> {
    mark(TokenKind::Column, db_identifier())
}

fn table_with_one<'src>() -> impl Psr<'src, String> {
    exactly(PATH_TO_TABLE_WITH_ONE_PREFIX).ignore_then(mark(
        TokenKind::Table,
        db_identifier().labelled(labels::TABLE_NAME),
    ))
}

/// Parses a `#table` path part, along with the location of its head: the table name and any linking
/// column, but not the condition set (see [`PathPart::loc`]).
fn table_with_many<'src>(expr: impl Psr<'src, Expr>) -> impl Psr<'src, (TableWithMany, Loc)> {
    let column = mark(TokenKind::Column, db_identifier()).delimited_by(
        just(TABLE_WITH_MANY_COLUMN_BRACE_L).then(pad()),
        pad().then(just(TABLE_WITH_MANY_COLUMN_BRACE_R)),
    );
    let table = mark(
        TokenKind::Table,
        just(TABLE_SIGIL).ignore_then(db_identifier().labelled(labels::TABLE_NAME)),
    );
    with_loc(table.then(column.or_not()))
        .then(condition_set(expr).or_not())
        .map(|(((table, column), loc), cs)| {
            let table = TableWithMany {
                table,
                condition_set: cs.unwrap_or_default(),
                linking_column: column,
            };
            (table, loc)
        })
}

#[cfg(test)]
mod tests {
    use crate::parser::utils::*;

    use super::*;

    /// A mock expression parser that will never succeed to parse any input. This okay because we
    /// don't test cases that require parsing expressions within paths. Testing for paths which
    /// contain expressions is done at a higher level.
    fn incapable_expression_parser<'src>() -> impl Psr<'src, Expr> {
        exactly("NOPE").map(|_| ExprKind::Variable("nope".to_string()).into())
    }

    fn simple_path<'src>() -> impl Psr<'src, Vec<PathPart>> {
        path(incapable_expression_parser()).then_ignore(end())
    }

    #[test]
    fn test_parse_path() {
        let p = |s: &str| simple_path().parse(s).into_result().map_err(|_| ());
        assert_eq!(
            p("foo"),
            Ok(vec![PathPartKind::Column("foo".to_string()).into()])
        );
        assert_eq!(
            p("foo.bar"),
            Ok(vec![
                PathPartKind::Column("foo".to_string()).into(),
                PathPartKind::Column("bar".to_string()).into(),
            ])
        );
        assert_eq!(
            p("#foo"),
            Ok(vec![PathPartKind::TableWithMany(TableWithMany {
                table: "foo".to_string(),
                linking_column: None,
                condition_set: ConditionSet::default(),
            })
            .into()])
        );
        assert_eq!(
            p("#foo(bar)"),
            Ok(vec![PathPartKind::TableWithMany(TableWithMany {
                table: "foo".to_string(),
                linking_column: Some("bar".to_string()),
                condition_set: ConditionSet::default(),
            })
            .into()])
        );
        assert_eq!(
            p(">>clients.start_date"),
            Ok(vec![
                PathPartKind::TableWithOne("clients".to_string()).into(),
                PathPartKind::Column("start_date".to_string()).into(),
            ])
        );
        assert_eq!(
            p("foo.bar.#baz(a).#bat.>>spam.eggs"),
            Ok(vec![
                PathPartKind::Column("foo".to_string()).into(),
                PathPartKind::Column("bar".to_string()).into(),
                PathPartKind::TableWithMany(TableWithMany {
                    table: "baz".to_string(),
                    linking_column: Some("a".to_string()),
                    condition_set: ConditionSet::default(),
                })
                .into(),
                PathPartKind::TableWithMany(TableWithMany {
                    table: "bat".to_string(),
                    linking_column: None,
                    condition_set: ConditionSet::default(),
                })
                .into(),
                PathPartKind::TableWithOne("spam".to_string()).into(),
                PathPartKind::Column("eggs".to_string()).into(),
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
