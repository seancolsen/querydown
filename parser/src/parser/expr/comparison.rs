use chumsky::prelude::*;

use crate::ast::*;
use crate::parser::expr::condition_set::expansion_set;
use crate::parser::utils::*;
use crate::tokens::*;

/// Combines a left side, an operator, and a right side into a [`Comparison`]. The two sides are
/// passed in pre-built (see [`left_comparison_side`] and [`right_comparison_side`]) because they draw
/// from different parsing modes: a bare word on the left is a column reference, while a bare word on
/// the right — at any depth — is a string literal. See `comparison_rhs_value` for the rationale.
pub fn comparison<'src>(
    left: impl Psr<'src, ComparisonSide>,
    right: impl Psr<'src, ComparisonSide>,
) -> impl Psr<'src, Comparison> {
    left.then(operator().padded_by(pad()))
        .then(right)
        .map(|((left, operator), right)| Comparison {
            left,
            operator,
            right,
        })
}

/// The left side of a comparison. A condition set here (`{ ... }` or `[ ... ]`) is automatically an
/// **expansion**: the comparison is distributed across the set's entries, joined by the set's
/// conjunction. Because this is the left side, bare words within `expansion_set`, `range_atom`, and
/// `expr` are column references; the caller supplies the column-reference ("identifier-mode") parsers.
pub fn left_comparison_side<'src>(
    expansion_set_expr: impl Psr<'src, Expr>,
    range_atom: impl Psr<'src, Expr>,
    expr: impl Psr<'src, Expr>,
) -> impl Psr<'src, ComparisonSide> {
    choice((
        expansion_set(expansion_set_expr).map(ComparisonSide::Expansion),
        range(range_atom).map(ComparisonSide::Range),
        expr.map(ComparisonSide::Expr),
    ))
}

/// The right side of a comparison. A condition set here (`{ ... }` or `[ ... ]`) is automatically an
/// **expansion**, just as on the left. Because this is the right side, bare words within
/// `expansion_set`, `range_atom`, and `expr` — at any depth — are string literals; the caller supplies
/// the string-literal ("string-mode") parsers.
pub fn right_comparison_side<'src>(
    expansion_set_expr: impl Psr<'src, Expr>,
    range_atom: impl Psr<'src, Expr>,
    expr: impl Psr<'src, Expr>,
) -> impl Psr<'src, ComparisonSide> {
    choice((
        expansion_set(expansion_set_expr).map(ComparisonSide::Expansion),
        range(range_atom).map(ComparisonSide::Range),
        expr.map(ComparisonSide::Expr),
    ))
}

fn range<'src>(expr: impl Psr<'src, Expr>) -> impl Psr<'src, Range> {
    let exclusivity = just(COMPARISON_RANGE_BOUND_EXCLUSIVE)
        .or_not()
        .map(|b| match b {
            Some(_) => Exclusivity::Exclusive,
            None => Exclusivity::Inclusive,
        });

    let lower = expr
        .clone()
        .then_ignore(pad())
        .then(exclusivity)
        .map(|(expr, exclusivity)| RangeBound { expr, exclusivity });

    let upper = exclusivity
        .then_ignore(pad())
        .then(expr.clone())
        .map(|(exclusivity, expr)| RangeBound { expr, exclusivity });

    lower
        .then_ignore(just(COMPARISON_RANGE_BOUND_SEPARATOR).padded_by(pad()))
        .then(upper)
        .map(|(lower, upper)| Range { lower, upper })
}

pub fn operator<'src>() -> impl Psr<'src, Operator> {
    choice((
        // Three character
        exactly(COMPARE_EQ_CASE_SENSITIVE).to(Operator::Eq),
        exactly(COMPARE_GTE).to(Operator::Gte),
        exactly(COMPARE_LTE).to(Operator::Lte),
        exactly(COMPARE_LIKE).to(Operator::Like),
        // Two character
        exactly(COMPARE_REGEX).to(Operator::RegexMatch),
        exactly(COMPARE_EQ).to(Operator::EqCi),
        exactly(COMPARE_GT).to(Operator::Gt),
        exactly(COMPARE_LT).to(Operator::Lt),
        // One character
        exactly(COMPARE_MATCH).to(Operator::Match),
    ))
}
