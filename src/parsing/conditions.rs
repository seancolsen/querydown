use chumsky::{prelude::*, text::*};

use crate::syntax_tree::*;
use crate::tokens::*;

use super::utils::*;

pub fn condition_set<E>(expression: E) -> impl Parser<char, ConditionSet, Error = Simple<char>>
where
    E: Parser<char, Expression, Error = Simple<char>> + Clone + 'static,
{
    recursive(|condition_set| {
        choice((
            generic_condition_set(
                Conjunction::And,
                AND_CONDITION_L_BRACE,
                condition_set.clone(),
                expression.clone(),
                AND_CONDITION_R_BRACE,
            ),
            generic_condition_set(
                Conjunction::Or,
                OR_CONDITION_L_BRACE,
                condition_set,
                expression,
                OR_CONDITION_R_BRACE,
            ),
        ))
    })
}

/// A condition set without braces. (Uses AND as the conjunction.)
pub fn implicit_condition_set<C, E>(
    condition_set: C,
    expression: E,
) -> impl Parser<char, ConditionSet, Error = Simple<char>>
where
    C: Parser<char, ConditionSet, Error = Simple<char>>,
    E: Parser<char, Expression, Error = Simple<char>> + Clone,
{
    condition_set_entry(condition_set, expression)
        .then_ignore(whitespace())
        .repeated()
        .map(move |entries| ConditionSet {
            conjunction: Conjunction::And,
            entries,
        })
}

fn generic_condition_set<C, E>(
    conjunction: Conjunction,
    l_brace: char,
    condition_set: C,
    expression: E,
    r_brace: char,
) -> impl Parser<char, ConditionSet, Error = Simple<char>>
where
    C: Parser<char, ConditionSet, Error = Simple<char>>,
    E: Parser<char, Expression, Error = Simple<char>> + Clone,
{
    just(l_brace).then(whitespace()).ignore_then(
        condition_set_entry(condition_set, expression)
            .then_ignore(whitespace())
            .repeated()
            .then_ignore(just(r_brace))
            .map(move |entries| ConditionSet {
                conjunction,
                entries,
            }),
    )
}

fn condition_set_entry<C, E>(
    condition_set: C,
    expression: E,
) -> impl Parser<char, ConditionSetEntry, Error = Simple<char>>
where
    C: Parser<char, ConditionSet, Error = Simple<char>>,
    E: Parser<char, Expression, Error = Simple<char>> + Clone,
{
    choice((
        condition(expression).map(ConditionSetEntry::Condition),
        condition_set.map(ConditionSetEntry::ConditionSet),
    ))
}

fn condition<E>(expression: E) -> impl Parser<char, Condition, Error = Simple<char>>
where
    E: Parser<char, Expression, Error = Simple<char>> + Clone,
{
    expression
        .clone()
        .then_ignore(whitespace())
        .then(operator())
        .then_ignore(whitespace())
        .then(expression)
        .map(|((lhs, operator), rhs)| Condition {
            left: lhs,
            operator,
            right: rhs,
        })
}

fn operator() -> impl Parser<char, Operator, Error = Simple<char>> {
    choice((
        exactly(OPERATOR_EQ).to(Operator::Eq),
        exactly(OPERATOR_GT).to(Operator::Gt),
        exactly(OPERATOR_GTE).to(Operator::Gte),
        exactly(OPERATOR_LT).to(Operator::Lt),
        exactly(OPERATOR_LTE).to(Operator::Lte),
        exactly(OPERATOR_LIKE).to(Operator::Like),
        exactly(OPERATOR_NEQ).to(Operator::Neq),
        exactly(OPERATOR_NOT_LIKE).to(Operator::NLike),
        exactly(OPERATOR_R_LIKE).to(Operator::RLike),
        exactly(OPERATOR_NOT_R_LIKE).to(Operator::NRLike),
        just(OPERATOR_SCOPED_CONDITIONAL).to(Operator::ScopedConditional),
    ))
}
