use querydown_parser::ast::{Comparison, ConditionSet, Expr, Operator};

use crate::{
    compiler::expr::convert_expr,
    sql::{
        expr::build::*,
        tree::{CtePurpose, SqlExpr},
        RegExFlags,
    },
};

use super::{
    paths::{clarify_path, ClarifiedPathTail},
    scope::Scope,
};

pub fn convert_comparison(c: Comparison, scope: &mut Scope) -> Result<SqlExpr, String> {
    let left_conditions = if c.is_expand_left {
        match c.left {
            Expr::ConditionSet(condition_set) => condition_set,
            _ => ConditionSet::via_and(vec![c.left]),
        }
    } else {
        ConditionSet::via_and(vec![c.left])
    };

    let right_conditions = if c.is_expand_right {
        match c.right {
            Expr::ConditionSet(condition_set) => condition_set,
            _ => ConditionSet::via_and(vec![c.right]),
        }
    } else {
        ConditionSet::via_and(vec![c.right])
    };

    let mut outer_entries = vec![];
    for left in left_conditions.entries.iter() {
        let mut inner_entries = vec![];
        for right in right_conditions.entries.iter() {
            inner_entries.push(convert_simple_comparison(left, c.operator, right, scope)?);
        }
        outer_entries.push(cmp::condition_set(
            inner_entries,
            &right_conditions.conjunction,
        ));
    }

    Ok(cmp::condition_set(
        outer_entries,
        &left_conditions.conjunction,
    ))
}

fn convert_simple_comparison(
    left: &Expr,
    operator: Operator,
    right: &Expr,
    scope: &mut Scope,
) -> Result<SqlExpr, String> {
    use Operator::*;

    if left.is_zero() && operator == Eq {
        return convert_expression_vs_zero(&right, ComparisonVsZero::Eq, scope);
    }
    if left.is_zero() && operator == Lt {
        return convert_expression_vs_zero(&right, ComparisonVsZero::Gt, scope);
    }
    if right.is_zero() && operator == Eq {
        return convert_expression_vs_zero(&left, ComparisonVsZero::Eq, scope);
    }
    if right.is_zero() && operator == Gt {
        return convert_expression_vs_zero(&left, ComparisonVsZero::Gt, scope);
    }

    if right.is_null() && operator == Eq {
        return convert_expr(left.to_owned(), scope).map(cmp::is_null);
    }
    if right.is_null() && operator == Neq {
        return convert_expr(left.to_owned(), scope).map(cmp::is_not_null);
    }
    if left.is_null() && operator == Eq {
        return convert_expr(right.to_owned(), scope).map(cmp::is_null);
    }
    if left.is_null() && operator == Neq {
        return convert_expr(right.to_owned(), scope).map(cmp::is_not_null);
    }

    let left_converted = convert_expr(left.to_owned(), scope)?;
    let right_converted = convert_expr(right.to_owned(), scope)?;

    let match_regex = |a: SqlExpr, b: SqlExpr, is_positive: bool, scope: &mut Scope| {
        let flags = RegExFlags {
            is_case_sensitive: false,
        };
        scope.options.dialect.match_regex(a, b, is_positive, &flags)
    };

    match &operator {
        Eq => Ok(cmp::eq(left_converted, right_converted)),
        Gt => Ok(cmp::gt(left_converted, right_converted)),
        Gte => Ok(cmp::gte(left_converted, right_converted)),
        Lt => Ok(cmp::lt(left_converted, right_converted)),
        Lte => Ok(cmp::lte(left_converted, right_converted)),
        Like => Ok(cmp::like(left_converted, right_converted)),
        Neq => Ok(cmp::neq(left_converted, right_converted)),
        NLike => Ok(cmp::nlike(left_converted, right_converted)),
        Match => Ok(match_regex(left_converted, right_converted, true, scope)),
        NMatch => Ok(match_regex(left_converted, right_converted, false, scope)),
    }
}

#[derive(Clone, Copy)]
enum ComparisonVsZero {
    Eq,
    Gt,
}

impl From<ComparisonVsZero> for Operator {
    fn from(cmp: ComparisonVsZero) -> Self {
        match cmp {
            ComparisonVsZero::Eq => Operator::Eq,
            ComparisonVsZero::Gt => Operator::Gt,
        }
    }
}

impl From<ComparisonVsZero> for CtePurpose {
    fn from(cmp: ComparisonVsZero) -> Self {
        match cmp {
            ComparisonVsZero::Eq => CtePurpose::Exclusion,
            ComparisonVsZero::Gt => CtePurpose::Inclusion,
        }
    }
}

fn convert_expression_vs_zero(
    expr: &Expr,
    cmp: ComparisonVsZero,
    scope: &mut Scope,
) -> Result<SqlExpr, String> {
    let fallback = |scope: &mut Scope| {
        let op = match cmp {
            ComparisonVsZero::Eq => cmp::eq,
            ComparisonVsZero::Gt => cmp::gt,
        };
        Ok(op(convert_expr(expr.to_owned(), scope)?, value::zero()))
    };
    let Expr::Path(path_parts) = &expr else { return fallback(scope) };
    let Ok(clarified_path) = clarify_path(path_parts.to_owned(), scope) else {
        return fallback(scope);
    };
    let Some(ClarifiedPathTail::ChainToMany((chain, None))) = clarified_path.tail else {
        return fallback(scope);
    };
    let join_result = scope.join_chain_to_many(&clarified_path.head, chain, None, cmp.into());
    let Ok(pk) = join_result else { return fallback(scope) };
    match cmp {
        ComparisonVsZero::Eq => Ok(cmp::is_null(pk)),
        ComparisonVsZero::Gt => Ok(cmp::is_not_null(pk)),
    }
}
