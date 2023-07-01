use querydown_parser::ast::*;

use crate::{
    compiler::expr::convert_expr,
    errors::msg,
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
    use ComparisonSide::{Expansion as CmpExpansion, Expr as CmpExpr, Range as CmpRange};

    let mut simple = |l: &Expr, r: &Expr| convert_simple_comparison(l, c.operator, r, scope);

    match (c.left, c.right) {
        // Two expressions
        (CmpExpr(left), CmpExpr(right)) => simple(&left, &right),

        // Expression vs expansion
        (CmpExpr(ref left), CmpExpansion(conditions)) => conditions
            .entries
            .iter()
            .map(|right| simple(left, right))
            .collect::<Result<Vec<_>, _>>()
            .map(|exprs| cmp::condition_set(exprs, &conditions.conjunction)),
        (CmpExpansion(conditions), CmpExpr(ref right)) => conditions
            .entries
            .iter()
            .map(|left| simple(left, right))
            .collect::<Result<Vec<_>, _>>()
            .map(|exprs| cmp::condition_set(exprs, &conditions.conjunction)),

        // Dual expansion
        (CmpExpansion(left_conditions), CmpExpansion(right_conditions)) => {
            let mut outer_entries = vec![];
            for left in left_conditions.entries.iter() {
                let mut inner_entries = vec![];
                for right in right_conditions.entries.iter() {
                    inner_entries.push(simple(left, right)?);
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

        // Range vs Expr
        (CmpRange(range), CmpExpr(expr)) | (CmpExpr(expr), CmpRange(range)) => {
            if c.operator != Operator::Eq {
                return Err(msg::compare_range_without_eq());
            }
            convert_range_comparison(&expr, &range, scope)
        }

        // Range vs Expansion
        (CmpExpansion(conditions), CmpRange(r)) | (CmpRange(r), CmpExpansion(conditions)) => {
            if c.operator != Operator::Eq {
                return Err(msg::compare_range_without_eq());
            }
            conditions
                .entries
                .iter()
                .map(|expr| convert_range_comparison(expr, &r, scope))
                .collect::<Result<Vec<_>, _>>()
                .map(|exprs| cmp::condition_set(exprs, &conditions.conjunction))
        }

        // Two ranges
        (CmpRange(_), CmpRange(_)) => Err(msg::compare_two_ranges()),
    }
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

fn convert_range_comparison(
    expr: &Expr,
    range: &Range,
    scope: &mut Scope,
) -> Result<SqlExpr, String> {
    let lower_op = match range.lower.exclusivity {
        Exclusivity::Inclusive => Operator::Gte,
        Exclusivity::Exclusive => Operator::Gt,
    };
    let lower = convert_simple_comparison(expr, lower_op, &range.lower.expr, scope)?;

    let upper_op = match range.upper.exclusivity {
        Exclusivity::Inclusive => Operator::Lte,
        Exclusivity::Exclusive => Operator::Lt,
    };
    let upper = convert_simple_comparison(expr, upper_op, &range.upper.expr, scope)?;

    Ok(cmp::condition_set([lower, upper], &Conjunction::And))
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
