use chumsky::prelude::*;

use crate::ast::*;
use crate::labels;
use crate::parser::utils::*;
use crate::tokens::*;

/// Parses a case expression, e.g.
///
/// ```text
/// ? status:="open"   ~ "Open"
///   status:="closed" ~ "Closed"
///   ~~                 "Other"
/// ```
///
/// A case begins with `?`, followed by one or more variants. Each variant pairs a condition
/// expression (before the `~`) with a value expression (after it). The expression ends with `~~`
/// followed by a fallback value used when no condition matches.
///
/// The `expr` parser passed in is the fully general expression parser, so conditions, values, and
/// the fallback may each be any expression. A variant's leading condition can't begin with `~`, so
/// the trailing `~~ fallback` is never mistaken for another variant.
pub fn case<'src>(expr: impl Psr<'src, Expr>) -> impl Psr<'src, Case> {
    let variant = expr
        .clone()
        .then_ignore(pad())
        .then_ignore(just(CASE_VARIANT_SEPARATOR).labelled(labels::CASE_VARIANT_SEPARATOR))
        .then_ignore(pad())
        .then(expr.clone())
        .map(|(condition, value)| CaseVariant { condition, value });

    just(CASE_BEGIN)
        .ignore_then(pad())
        .ignore_then(
            variant
                .then_ignore(pad())
                .repeated()
                .at_least(1)
                .collect::<Vec<CaseVariant>>(),
        )
        .then_ignore(exactly(CASE_FALLBACK).labelled(labels::CASE_FALLBACK))
        .then_ignore(pad())
        .then(expr)
        .map(|(variants, fallback)| Case {
            variants,
            fallback: Box::new(fallback),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::expr::expr;

    fn parse(s: &str) -> Result<Expr, ()> {
        expr()
            .then_ignore(end())
            .parse(s)
            .into_result()
            .map_err(|_| ())
    }

    fn col(name: &str) -> Expr {
        ExprKind::Path(vec![PathPartKind::Column(name.to_string()).into()]).into()
    }

    fn str_(s: &str) -> Expr {
        ExprKind::String(s.to_string()).into()
    }

    // Builds the comparison produced by a `:=` operand (case-insensitive equality).
    fn match_eq(left: &str, right: Expr) -> Expr {
        ExprKind::Comparison(Box::new(Comparison {
            left: ComparisonSide::Expr(col(left)),
            operator: Operator::EqCi,
            right: ComparisonSide::Expr(right),
        }))
        .into()
    }

    #[test]
    fn test_single_variant() {
        assert_eq!(
            parse(r#"? is_active ~ "yes" ~~ "no""#),
            Ok(ExprKind::Case(Case {
                variants: vec![CaseVariant {
                    condition: col("is_active"),
                    value: str_("yes"),
                }],
                fallback: Box::new(str_("no")),
            })
            .into())
        );
    }

    #[test]
    fn test_multiple_variants() {
        // Whitespace (including newlines) between the parts is insignificant.
        let input = "?\n  status:=\"open\"   ~ \"Open\"\n  status:=\"closed\" ~ \"Closed\"\n  ~~                 \"Other\"";
        assert_eq!(
            parse(input),
            Ok(ExprKind::Case(Case {
                variants: vec![
                    CaseVariant {
                        condition: match_eq("status", str_("open")),
                        value: str_("Open"),
                    },
                    CaseVariant {
                        condition: match_eq("status", str_("closed")),
                        value: str_("Closed"),
                    },
                ],
                fallback: Box::new(str_("Other")),
            })
            .into())
        );
    }

    #[test]
    fn test_numeric_values() {
        assert_eq!(
            parse("? is_active ~ 1 ~~ 0"),
            Ok(ExprKind::Case(Case {
                variants: vec![CaseVariant {
                    condition: col("is_active"),
                    value: ExprKind::Number("1".to_string()).into(),
                }],
                fallback: Box::new(ExprKind::Number("0".to_string()).into()),
            })
            .into())
        );
    }

    #[test]
    fn test_fallback_is_required() {
        // Without the `~~` fallback the expression is incomplete.
        assert!(parse(r#"? is_active ~ "yes""#).is_err());
    }

    #[test]
    fn test_at_least_one_variant_required() {
        assert!(parse(r#"? ~~ "no""#).is_err());
    }

    #[test]
    fn test_case_in_parentheses() {
        // A case expression can be nested inside other expressions when parenthesized.
        assert_eq!(
            parse(r#"(? is_active ~ "yes" ~~ "no")"#),
            Ok(ExprKind::Case(Case {
                variants: vec![CaseVariant {
                    condition: col("is_active"),
                    value: str_("yes"),
                }],
                fallback: Box::new(str_("no")),
            })
            .into())
        );
    }
}
