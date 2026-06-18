use chumsky::prelude::*;

use crate::ast::*;
use crate::parser::expr::condition_set::condition_set;
use crate::parser::utils::*;
use crate::tokens::*;

pub fn comparison<'src>(
    left_side_expr: impl Psr<'src, Expr>,
    right_side_expr: impl Psr<'src, Expr>,
    condition_set_expr: impl Psr<'src, Expr>,
    range_expr: impl Psr<'src, Expr>,
) -> impl Psr<'src, Comparison> {
    let left = choice((
        condition_set(condition_set_expr.clone())
            .then_ignore(pad().then(just(COMPARISON_EXPAND)))
            .map(ComparisonSide::Expansion),
        range(range_expr.clone()).map(ComparisonSide::Range),
        left_side_expr.map(ComparisonSide::Expr),
    ));
    let right = choice((
        just(COMPARISON_EXPAND)
            .then(pad())
            .ignore_then(condition_set(condition_set_expr).map(ComparisonSide::Expansion)),
        range(range_expr).map(ComparisonSide::Range),
        right_side_expr.map(ComparisonSide::Expr),
    ));

    left.then(operator().padded_by(pad()))
        .then(right)
        .map(|((left, operator), right)| Comparison {
            left,
            operator,
            right,
        })
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

fn operator<'src>() -> impl Psr<'src, Operator> {
    choice((
        // Three character
        exactly(COMPARE_GTE).to(Operator::Gte),
        exactly(COMPARE_LTE).to(Operator::Lte),
        exactly(COMPARE_LIKE).to(Operator::Like),
        // Two character
        exactly(COMPARE_REGEX).to(Operator::RegexMatch),
        exactly(COMPARE_EQ).to(Operator::Eq),
        exactly(COMPARE_GT).to(Operator::Gt),
        exactly(COMPARE_LT).to(Operator::Lt),
        // One character
        exactly(COMPARE_MATCH).to(Operator::Match),
    ))
}
