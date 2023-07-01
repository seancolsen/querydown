use chumsky::{prelude::*, text::*};

use crate::ast::*;
use crate::parser::expr::condition_set::condition_set;
use crate::parser::utils::*;
use crate::tokens::*;

pub fn comparison(
    comparison_side_expr: impl Psr<Expr>,
    condition_set_expr: impl Psr<Expr>,
    range_expr: impl Psr<Expr>,
) -> impl Psr<Comparison> {
    let left = choice((
        condition_set(condition_set_expr.clone())
            .then_ignore(whitespace().then(just(COMPARISON_EXPAND)))
            .map(ComparisonSide::Expansion),
        range(range_expr.clone()).map(ComparisonSide::Range),
        comparison_side_expr.clone().map(ComparisonSide::Expr),
    ));
    let right = choice((
        just(COMPARISON_EXPAND)
            .then(whitespace())
            .ignore_then(condition_set(condition_set_expr.clone()).map(ComparisonSide::Expansion)),
        range(range_expr.clone()).map(ComparisonSide::Range),
        comparison_side_expr.clone().map(ComparisonSide::Expr),
    ));

    left.then(operator().padded())
        .then(right)
        .map(|((left, operator), right)| Comparison {
            left,
            operator,
            right,
        })
}

fn range(expr: impl Psr<Expr>) -> impl Psr<Range> {
    let exclusivity = just(COMPARISON_RANGE_BOUND_EXCLUSIVE)
        .or_not()
        .map(|b| match b {
            Some(_) => Exclusivity::Exclusive,
            None => Exclusivity::Inclusive,
        });

    let lower = expr
        .clone()
        .then_ignore(whitespace())
        .then(exclusivity)
        .map(|(expr, exclusivity)| RangeBound { expr, exclusivity });

    let upper = exclusivity
        .then_ignore(whitespace())
        .then(expr.clone())
        .map(|(exclusivity, expr)| RangeBound { expr, exclusivity });

    lower
        .then_ignore(just(COMPARISON_RANGE_BOUND_SEPARATOR).padded())
        .then(upper)
        .map(|(lower, upper)| Range { lower, upper })
}

fn operator() -> impl Psr<Operator> {
    choice((
        // Three character
        exactly(COMPARE_NOT_LIKE).to(Operator::NLike),
        exactly(COMPARE_GTE).to(Operator::Gte),
        exactly(COMPARE_LTE).to(Operator::Lte),
        exactly(COMPARE_LIKE).to(Operator::Like),
        // Two character
        exactly(COMPARE_MATCH).to(Operator::Match),
        exactly(COMPARE_NOT_MATCH).to(Operator::NMatch),
        exactly(COMPARE_GT).to(Operator::Gt),
        exactly(COMPARE_LT).to(Operator::Lt),
        // One character
        exactly(COMPARE_EQ).to(Operator::Eq),
        exactly(COMPARE_NEQ).to(Operator::Neq),
    ))
}
