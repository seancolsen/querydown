use chumsky::{prelude::*, text::*};

use crate::ast::*;
use crate::parser::utils::*;
use crate::tokens::*;

pub fn comparison(expr: impl Psr<Expr>) -> impl Psr<Expr> {
    expr.clone()
        .then(separator().padded().then(expr).repeated())
        .foldl(|left, (sep, right)| {
            Expr::Comparison(Box::new(Comparison {
                left,
                operator: sep.operator,
                right,
                is_expand_left: sep.is_expand_left,
                is_expand_right: sep.is_expand_right,
            }))
        })
}

struct Separator {
    pub operator: Operator,
    pub is_expand_left: bool,
    pub is_expand_right: bool,
}

fn separator() -> impl Psr<Separator> {
    just(COMPARISON_EXPAND)
        .or_not()
        .map(|b| b.is_some())
        .then_ignore(whitespace())
        .then(operator())
        .then_ignore(whitespace())
        .then(just(COMPARISON_EXPAND).or_not().map(|b| b.is_some()))
        .map(|((is_expand_left, operator), is_expand_right)| Separator {
            operator,
            is_expand_left,
            is_expand_right,
        })
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
