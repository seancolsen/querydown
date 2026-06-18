use chumsky::prelude::*;

use crate::ast::*;
use crate::tokens::*;

use super::expr::expr;
use super::utils::*;

/// Parses a sequence of standalone sorting expressions, each prefixed with `\\`. These sit between
/// the filtering expressions and the result columns within a transformation. The order in which
/// they are listed defines their sort precedence. Yields an empty vector when none are present.
pub fn sorting<'src>() -> impl Psr<'src, Vec<SortExpr>> {
    sort_expr()
        .then_ignore(pad())
        .repeated()
        .collect::<Vec<SortExpr>>()
}

fn sort_expr<'src>() -> impl Psr<'src, SortExpr> {
    just(SORT_EXPR_PREFIX)
        .then(pad())
        .ignore_then(expr())
        .then(pad().ignore_then(sort_flags()).or_not())
        .map(|(expr, flags)| {
            let (direction, nulls_sort) = flags.unwrap_or_default();
            SortExpr {
                expr,
                direction,
                nulls_sort,
            }
        })
}

/// Parses the optional `\d` / `\n` flags that may follow a standalone sorting expression. Only `d`
/// (descending) and `n` (nulls first) are accepted — `s` is implied by the `\\` prefix. Flags may
/// be combined within a single backslash group, e.g. `\dn`.
pub(crate) fn sort_flags<'src>() -> impl Psr<'src, (SortDirection, NullsSort)> {
    #[derive(Clone)]
    enum Flag {
        Desc,
        NullsFirst,
    }
    let flag = choice((
        just(COLUMN_CONTROL_FLAG_DESC).to(Flag::Desc),
        just(COLUMN_CONTROL_FLAG_NULLS_FIRST).to(Flag::NullsFirst),
    ));
    just(COLUMN_CONTROL_FLAGS_PREFIX).ignore_then(
        flag.repeated()
            .at_least(1)
            .collect::<Vec<Flag>>()
            .map(|flags| {
                let mut direction = SortDirection::default();
                let mut nulls_sort = NullsSort::default();
                for flag in flags {
                    match flag {
                        Flag::Desc => direction = SortDirection::Desc,
                        Flag::NullsFirst => nulls_sort = NullsSort::First,
                    }
                }
                (direction, nulls_sort)
            }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn col(name: &str) -> Expr {
        Expr::Path(vec![PathPart::Column(name.to_string())])
    }

    #[test]
    fn test_parse_sort_expr_default() {
        assert_eq!(
            sorting().parse(r"\\created_at").into_result(),
            Ok(vec![SortExpr {
                expr: col("created_at"),
                direction: SortDirection::Asc,
                nulls_sort: NullsSort::Last,
            }])
        );
    }

    #[test]
    fn test_parse_sort_expr_descending() {
        assert_eq!(
            sorting().parse(r"\\created_at \d").into_result(),
            Ok(vec![SortExpr {
                expr: col("created_at"),
                direction: SortDirection::Desc,
                nulls_sort: NullsSort::Last,
            }])
        );
    }

    #[test]
    fn test_parse_sort_expr_combined_flags() {
        assert_eq!(
            sorting().parse(r"\\created_at \dn").into_result(),
            Ok(vec![SortExpr {
                expr: col("created_at"),
                direction: SortDirection::Desc,
                nulls_sort: NullsSort::First,
            }])
        );
    }

    #[test]
    fn test_parse_sort_expr_whitespace_after_prefix() {
        assert_eq!(
            sorting().parse(r"\\ created_at \d").into_result(),
            Ok(vec![SortExpr {
                expr: col("created_at"),
                direction: SortDirection::Desc,
                nulls_sort: NullsSort::Last,
            }])
        );
    }

    #[test]
    fn test_parse_multiple_sort_exprs() {
        assert_eq!(
            sorting().parse(r"\\a \d \\b").into_result(),
            Ok(vec![
                SortExpr {
                    expr: col("a"),
                    direction: SortDirection::Desc,
                    nulls_sort: NullsSort::Last,
                },
                SortExpr {
                    expr: col("b"),
                    direction: SortDirection::Asc,
                    nulls_sort: NullsSort::Last,
                },
            ])
        );
    }

    #[test]
    fn test_parse_no_sort_exprs() {
        assert_eq!(sorting().parse("").into_result(), Ok(vec![]));
    }
}
