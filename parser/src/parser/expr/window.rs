use chumsky::{prelude::*, text::*};

use crate::ast::*;
use crate::parser::column_layout::column_control;
use crate::parser::utils::*;
use crate::tokens::*;

/// Parses a window function application, e.g. `%%(account\p date\s)%sum(amount)`.
///
/// The `%%( ... )` defines the window. Inside, each entry uses the same syntax as a [column
/// glob][crate::ast::ColumnGlob] spec: an expression followed by optional flags. The `\p` flag marks
/// a partition expression and `\s` (with the usual `\d`/`\n`/ordinal modifiers) marks an ordering
/// expression. An entry with no flag is treated as a partition expression.
///
/// After the window definition, `%func` names the window function and an optional parenthesized,
/// space-separated argument list supplies its value arguments (e.g. the column for `sum`, or the
/// column plus offset and default for `lag`). Ranking functions like `row_number` take no arguments.
pub fn window<'src>(full_expr: impl Psr<'src, Expr>) -> impl Psr<'src, Expr> {
    let item = full_expr
        .clone()
        .then(pad().ignore_then(column_control()).or_not());

    let definition = item
        .padded_by(pad())
        .repeated()
        .collect::<Vec<(Expr, Option<ColumnControl>)>>()
        .delimited_by(
            just(WINDOW_DEFINITION_BRACE_L),
            just(WINDOW_DEFINITION_BRACE_R),
        );

    let args = full_expr
        .padded_by(pad())
        .repeated()
        .collect::<Vec<Expr>>()
        .delimited_by(
            just(COMPOSITION_ARGUMENT_BRACE_L),
            just(COMPOSITION_ARGUMENT_BRACE_R),
        );

    let function = just(COMPOSITION_PIPE_AGGREGATE)
        .ignore_then(ident())
        .then(args.or_not());

    just(WINDOW_DEFINITION_PREFIX)
        .ignore_then(pad())
        .ignore_then(definition)
        .then(pad().ignore_then(function))
        .map(|(items, (name, args))| {
            let (partition_by, order_by) = split_window_items(items);
            Expr::Window(WindowFn {
                function: name.to_string(),
                args: args.unwrap_or_default(),
                partition_by,
                order_by,
            })
        })
}

/// Splits the window definition's entries into partition expressions and ordering entries. An entry
/// flagged with `\s` becomes an ordering entry (honoring its direction, nulls, and ordinal); any
/// other entry (including `\p` and unflagged entries) becomes a partition expression. Ordering
/// entries are placed according to their ordinals, mirroring how `\s` ordinals work for result
/// columns.
fn split_window_items(items: Vec<(Expr, Option<ColumnControl>)>) -> (Vec<Expr>, Vec<SortExpr>) {
    let mut partition_by = Vec::new();
    let mut ordering: Vec<(Option<u32>, SortExpr)> = Vec::new();
    for (expr, control) in items {
        match control.and_then(|c| c.sort) {
            Some(sort) => ordering.push((
                sort.ordinal,
                SortExpr {
                    expr,
                    direction: sort.direction,
                    nulls_sort: sort.nulls_sort,
                },
            )),
            None => partition_by.push(expr),
        }
    }
    let max_ordinal = ordering.iter().filter_map(|(o, _)| *o).max().unwrap_or(0);
    ordering.sort_by_key(|(ordinal, _)| ordinal.unwrap_or(max_ordinal.saturating_add(1)));
    let order_by = ordering.into_iter().map(|(_, sort)| sort).collect();
    (partition_by, order_by)
}

#[cfg(test)]
mod tests {
    use crate::ast::*;
    use crate::parser::expr::expr;
    use chumsky::prelude::*;

    fn parse(s: &str) -> Result<Expr, ()> {
        expr()
            .then_ignore(end())
            .parse(s)
            .into_result()
            .map_err(|_| ())
    }

    fn col(name: &str) -> Expr {
        Expr::Path(vec![PathPart::Column(name.to_string())])
    }

    #[test]
    fn test_parse_ranking_window() {
        assert_eq!(
            parse(r"%%(project\p created_at\s)%row_number"),
            Ok(Expr::Window(WindowFn {
                function: "row_number".to_string(),
                args: vec![],
                partition_by: vec![col("project")],
                order_by: vec![SortExpr {
                    expr: col("created_at"),
                    direction: SortDirection::Asc,
                    nulls_sort: NullsSort::Last,
                }],
            }))
        );
    }

    #[test]
    fn test_parse_window_with_value_args() {
        // The value column and extra arguments are supplied in parentheses after the function name.
        assert_eq!(
            parse(r"%%(account\p date\sd)%lag(balance 1 0)"),
            Ok(Expr::Window(WindowFn {
                function: "lag".to_string(),
                args: vec![
                    col("balance"),
                    Expr::Number("1".to_string()),
                    Expr::Number("0".to_string())
                ],
                partition_by: vec![col("account")],
                order_by: vec![SortExpr {
                    expr: col("date"),
                    direction: SortDirection::Desc,
                    nulls_sort: NullsSort::Last,
                }],
            }))
        );
    }

    #[test]
    fn test_window_order_by_ordinals() {
        // Ordering entries are placed by their `\s` ordinals; an unflagged entry partitions.
        let Ok(Expr::Window(window)) = parse(r"%%(a b\s2 c\s1)%row_number") else {
            panic!("expected a window expression");
        };
        assert_eq!(window.partition_by, vec![col("a")]);
        let order_cols: Vec<Expr> = window.order_by.into_iter().map(|s| s.expr).collect();
        assert_eq!(order_cols, vec![col("c"), col("b")]);
    }

    #[test]
    fn test_window_used_in_comparison() {
        // A window function can be the left-hand side of a comparison (used to filter on it).
        let result = parse(r"%%(project\p created_at\sd)%row_number:1");
        assert!(matches!(result, Ok(Expr::Comparison(_))));
    }
}
