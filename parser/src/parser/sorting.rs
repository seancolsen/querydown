use chumsky::prelude::*;

use crate::ast::*;
use crate::tokens::*;

use super::expr::{expr, path};
use super::nesting::prefix_leading_path;
use super::utils::*;

/// Parses a sequence of standalone sorting expressions, each prefixed with `\\`. These sit between
/// the filtering expressions and the result columns within a transformation. The order in which
/// they are listed defines their sort precedence. Yields an empty vector when none are present.
pub fn sorting<'src>() -> impl Psr<'src, Vec<SortExpr>> {
    sort_statement()
        .then_ignore(pad())
        .repeated()
        .collect::<Vec<Vec<SortExpr>>>()
        .map(|groups| groups.into_iter().flatten().collect())
}

/// Parses one `\\`-prefixed sorting statement. A plain sort expression yields a single `SortExpr`,
/// but a nesting group like `\\issue.( \\id \\title )` desugars into one `SortExpr` per nested
/// entry (`\\issue.id \\issue.title`), so this yields a `Vec`.
fn sort_statement<'src>() -> impl Psr<'src, Vec<SortExpr>> {
    recursive(|statement| {
        just(SORT_EXPR_PREFIX).then(pad()).ignore_then(choice((
            // `nested_sort_exprs` is tried before `sort_expr_tail` because both begin with the same
            // path; they diverge only at the `.(` that follows the nesting group's head.
            nested_sort_exprs(statement),
            sort_expr_tail().map(|sort_expr| vec![sort_expr]),
        )))
    })
}

/// Parses a sorting nesting group, e.g. `\\issue.( \\id \\title )`, desugaring it into the
/// equivalent flat sort expressions (`\\issue.id \\issue.title`) by applying the head path to each
/// nested entry. Groups may be nested within groups; the heads compose.
fn nested_sort_exprs<'src>(
    statement: impl Psr<'src, Vec<SortExpr>> + 'src,
) -> impl Psr<'src, Vec<SortExpr>> {
    let entries = statement
        .then_ignore(pad())
        .repeated()
        .collect::<Vec<Vec<SortExpr>>>();
    path(expr())
        .then_ignore(pad().then(just(PATH_SEPARATOR)))
        .then(
            just(SORTING_NESTING_BRACE_L)
                .then(pad())
                .ignore_then(entries)
                .then_ignore(just(SORTING_NESTING_BRACE_R)),
        )
        .try_map(|(head, groups), span| {
            groups
                .into_iter()
                .flatten()
                .map(|sort_expr| apply_path_prefix(&head, sort_expr))
                .collect::<Result<Vec<SortExpr>, String>>()
                .map_err(|message| Rich::custom(span, message))
        })
}

/// Applies a nesting group's head path to one (already desugared) sort expression within the
/// group, by splicing the head onto the leading column reference of the expression.
fn apply_path_prefix(head: &[PathPart], sort_expr: SortExpr) -> Result<SortExpr, String> {
    Ok(SortExpr {
        expr: prefix_leading_path(head, sort_expr.expr)?,
        ..sort_expr
    })
}

/// Parses the part of a sort expression that follows the `\\` prefix: the expression itself,
/// followed by optional sort flags.
fn sort_expr_tail<'src>() -> impl Psr<'src, SortExpr> {
    expr()
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

    /// Parses a full sorting section, requiring that all input is consumed. Errors are discarded
    /// because these tests only compare desugared output across equivalent inputs.
    fn parse_sorting(source: &str) -> Result<Vec<SortExpr>, ()> {
        sorting()
            .then_ignore(end())
            .parse(source)
            .into_result()
            .map_err(|_| ())
    }

    #[test]
    fn test_scoped_sorting_desugars_to_flat_form() {
        // A nesting group is exactly equivalent to writing the head before each nested entry.
        let nested = parse_sorting(
            r"\\project.(
              \\is_active
              \\product.name
            )
            \\due_date",
        );
        let flat = parse_sorting(
            r"\\project.is_active
            \\project.product.name
            \\due_date",
        );
        assert!(nested.is_ok());
        assert_eq!(nested, flat);
    }

    #[test]
    fn test_scoped_sorting_equivalences() {
        let eq = |nested: &str, flat: &str| {
            let nested_result = parse_sorting(nested);
            assert!(nested_result.is_ok(), "failed to parse: {nested}");
            assert_eq!(nested_result, parse_sorting(flat), "for: {nested}");
        };
        // A multi-part head, and a group containing a single entry.
        eq(r"\\a.b.( \\c )", r"\\a.b.c");
        // Groups nest; the heads compose.
        eq(r"\\a.( \\b.( \\c ) \\d )", r"\\a.b.c \\a.d");
        // The head is spliced onto the piped-in value of a function application.
        eq(r"\\issue.( \\title|upper )", r"\\issue.title|upper");
        // ... and onto the left side of arithmetic and comparisons, through a `!` prefix, and onto
        // a `++`/`--` quantity path.
        eq(r"\\issue.( \\id * 2 )", r"\\issue.id * 2");
        eq(r"\\issue.( \\status:done )", r"\\issue.status:done");
        eq(r"\\issue.( \\!is_open )", r"\\!issue.is_open");
        eq(r"\\issue.( \\++#labels )", r"\\++issue.#labels");
        // A head may traverse to-many paths.
        eq(
            r"\\#comments.( \\created_at%max )",
            r"\\#comments.created_at%max",
        );
        // Sort flags ride along.
        eq(
            r"\\issue.( \\id \d \\created_at \n )",
            r"\\issue.id \d \\issue.created_at \n",
        );
        // An empty group contributes no sort expressions.
        assert_eq!(parse_sorting(r"\\issue.( ) \\id"), parse_sorting(r"\\id"));
    }

    #[test]
    fn test_scoped_sorting_requires_leading_column_reference() {
        // A nested entry whose expression has no leading column reference cannot receive the head
        // path, so it is a parse error.
        assert!(parse_sorting(r"\\issue.( \\8 )").is_err());
        assert!(parse_sorting(r"\\issue.( \\'foo' )").is_err());
        assert!(parse_sorting(r"\\issue.( \\%count )").is_err());
        assert!(parse_sorting(r"\\issue.( \\@@now() )").is_err());
        // Even when other entries are fine.
        assert!(parse_sorting(r"\\issue.( \\id \\8 )").is_err());
    }
}
