use chumsky::{prelude::*, text::*};

use crate::syntax_tree::*;
use crate::tokens::*;

use super::paths::*;
use super::utils::*;
use crate::syntax_tree::Conjunction::*;

/// An explicit condition set, with {} braces for AND or [] braces for OR.
pub fn condition_set(expression: impl QdParser<Expression>) -> impl QdParser<ConditionSet> {
    recursive(|condition_set| {
        let specific_condition_set = |conjunction: Conjunction| {
            let (l_brace, r_brace) = get_braces(conjunction);
            condition_set_entry(condition_set.clone(), expression.clone())
                .padded()
                .repeated()
                .delimited_by(just(l_brace), just(r_brace))
                .map(move |entries| ConditionSet {
                    conjunction,
                    entries,
                })
        };
        choice((specific_condition_set(And), specific_condition_set(Or)))
    })
}

/// A condition set without braces. (Uses AND as the conjunction.)
pub fn implicit_condition_set(
    condition_set: impl QdParser<ConditionSet>,
    expression: impl QdParser<Expression>,
) -> impl QdParser<ConditionSet> {
    condition_set_entry(condition_set, expression)
        .then_ignore(whitespace())
        .repeated()
        .map(move |entries| ConditionSet {
            conjunction: And,
            entries,
        })
}

fn condition_set_entry(
    condition_set: impl QdParser<ConditionSet>,
    expression: impl QdParser<Expression>,
) -> impl QdParser<ConditionSetEntry> {
    use ConditionSetEntry::*;
    choice((
        condition_set.clone().map(ConditionSet),
        scoped_conditional(condition_set, expression.clone()).map(ScopedConditional),
        comparison(expression).map(Comparison),
    ))
}

fn comparison(expression: impl QdParser<Expression>) -> impl QdParser<Comparison> {
    comparison_part(expression.clone())
        .clone()
        .then_ignore(whitespace())
        .then(operator())
        .then_ignore(whitespace())
        .then(comparison_part(expression))
        .map(|((lhs, operator), rhs)| Comparison {
            left: lhs,
            operator,
            right: rhs,
        })
}

fn scoped_conditional(
    condition_set: impl QdParser<ConditionSet>,
    expression: impl QdParser<Expression>,
) -> impl QdParser<ScopedConditional> {
    comparison_part(expression)
        .clone()
        .then_ignore(
            whitespace()
                .then(just(SCOPED_CONDITIONAL))
                .then(whitespace()),
        )
        .then(condition_set)
        .map(|(left, right)| ScopedConditional { left, right })
}

fn comparison_part(expression: impl QdParser<Expression>) -> impl QdParser<ComparisonPart> {
    choice((
        expression.clone().map(ComparisonPart::Expression),
        expression_set(expression).map(ComparisonPart::ExpressionSet),
    ))
}

fn operator() -> impl QdParser<Operator> {
    choice((
        // Three character
        exactly(OPERATOR_NOT_LIKE).to(Operator::NLike),
        // Two character
        exactly(OPERATOR_GTE).to(Operator::Gte),
        exactly(OPERATOR_LTE).to(Operator::Lte),
        exactly(OPERATOR_LIKE).to(Operator::Like),
        exactly(OPERATOR_NEQ).to(Operator::Neq),
        exactly(OPERATOR_MATCH).to(Operator::Match),
        exactly(OPERATOR_NOT_R_LIKE).to(Operator::NRLike),
        // One character
        exactly(OPERATOR_EQ).to(Operator::Eq),
        exactly(OPERATOR_GT).to(Operator::Gt),
        exactly(OPERATOR_LT).to(Operator::Lt),
    ))
}

pub fn expression_set(expression: impl QdParser<Expression>) -> impl QdParser<ExpressionSet> {
    let specific_expression_set = |conjunction: Conjunction| {
        let (l_brace, r_brace) = get_braces(conjunction);
        expression
            .clone()
            .padded()
            .repeated()
            .delimited_by(just(l_brace), just(r_brace))
            .map(move |entries| ExpressionSet {
                conjunction,
                entries,
            })
    };
    choice((specific_expression_set(And), specific_expression_set(Or)))
}

fn get_braces(conjunction: Conjunction) -> (char, char) {
    match conjunction {
        And => (CONDITION_SET_AND_BRACE_L, CONDITION_SET_AND_BRACE_R),
        Or => (CONDITION_SET_OR_BRACE_L, CONDITION_SET_OR_BRACE_R),
    }
}
