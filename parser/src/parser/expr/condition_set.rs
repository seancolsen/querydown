use chumsky::prelude::*;

use crate::ast::*;
use crate::parser::utils::*;
use crate::tokens::*;

pub fn condition_set<'src>(expr: impl Psr<'src, Expr>) -> impl Psr<'src, ConditionSet> {
    choice((
        specific_condition_set(Conjunction::And, expr.clone()),
        specific_condition_set(Conjunction::Or, expr),
    ))
}

fn specific_condition_set<'src>(
    conjunction: Conjunction,
    expr: impl Psr<'src, Expr>,
) -> impl Psr<'src, ConditionSet> {
    let (brace_l, brace_r) = match conjunction {
        Conjunction::And => (CONDITION_SET_AND_BRACE_L, CONDITION_SET_AND_BRACE_R),
        Conjunction::Or => (CONDITION_SET_OR_BRACE_L, CONDITION_SET_OR_BRACE_R),
    };
    expr.padded_by(pad())
        .repeated()
        .collect::<Vec<Expr>>()
        .delimited_by(just(brace_l), just(brace_r))
        .map(move |entries| ConditionSet {
            conjunction,
            entries,
        })
}
