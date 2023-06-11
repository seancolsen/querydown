use crate::{
    compiling::{
        rendering::rendering::Render,
        scope::Scope,
        sql_tree::{CtePurpose, SqlConditionSet, SqlConditionSetEntry},
    },
    dialects::{dialect::RegExFlags, sql},
    syntax_tree::{
        Comparison, ComparisonPart, ConditionSet, ConditionSetEntry, Conjunction, Expression,
        Operator, Value,
    },
};

use super::paths::{clarify_path, ClarifiedPathTail};

struct SimpleConditionSet {
    conjunction: Conjunction,
    entries: Vec<SimpleConditionSetEntry>,
}

impl SimpleConditionSet {
    pub fn new(conjunction: Conjunction, entries: Vec<SimpleConditionSetEntry>) -> Self {
        Self {
            conjunction,
            entries,
        }
    }
}

enum SimpleConditionSetEntry {
    SimpleComparison(SimpleComparison),
    SimpleConditionSet(SimpleConditionSet),
}

impl SimpleConditionSetEntry {
    pub fn new_comparison(left: Expression, operator: Operator, right: Expression) -> Self {
        Self::SimpleComparison(SimpleComparison::new(left, operator, right))
    }

    pub fn new_set(conjunction: Conjunction, entries: Vec<SimpleConditionSetEntry>) -> Self {
        Self::SimpleConditionSet(SimpleConditionSet::new(conjunction, entries))
    }
}

struct SimpleComparison {
    left: Expression,
    operator: Operator,
    right: Expression,
}

impl SimpleComparison {
    pub fn new(left: Expression, operator: Operator, right: Expression) -> Self {
        Self {
            left,
            operator,
            right,
        }
    }
}

pub fn convert_condition_set(condition_set: &ConditionSet, scope: &mut Scope) -> SqlConditionSet {
    SqlConditionSet {
        conjunction: condition_set.conjunction,
        entries: condition_set
            .entries
            .iter()
            .map(|entry| convert_condition_set_entry(entry, scope))
            .collect(),
    }
}

fn convert_condition_set_entry(
    condition_set_entry: &ConditionSetEntry,
    scope: &mut Scope,
) -> SqlConditionSetEntry {
    match condition_set_entry {
        ConditionSetEntry::Comparison(comparison) => convert_comparison(comparison, scope),
        ConditionSetEntry::ConditionSet(condition_set) => {
            SqlConditionSetEntry::ConditionSet(convert_condition_set(condition_set, scope))
        }
    }
}

fn convert_comparison(comparison: &Comparison, scope: &mut Scope) -> SqlConditionSetEntry {
    convert_simple_condition_set_entry(&expand_comparison(comparison), scope)
}

fn convert_simple_condition_set_entry(
    entry: &SimpleConditionSetEntry,
    scope: &mut Scope,
) -> SqlConditionSetEntry {
    match entry {
        SimpleConditionSetEntry::SimpleComparison(comparison) => {
            convert_simple_comparison(comparison, scope)
        }
        SimpleConditionSetEntry::SimpleConditionSet(condition_set) => {
            SqlConditionSetEntry::ConditionSet(SqlConditionSet {
                conjunction: condition_set.conjunction,
                entries: condition_set
                    .entries
                    .iter()
                    .map(|entry| convert_simple_condition_set_entry(entry, scope))
                    .collect(),
            })
        }
    }
}

fn convert_simple_comparison(
    simple_comparison: &SimpleComparison,
    scope: &mut Scope,
) -> SqlConditionSetEntry {
    use Operator::*;

    let SimpleComparison {
        left,
        operator,
        right,
    } = simple_comparison;

    if left.is_zero() && operator == &Eq {
        return convert_expression_vs_zero(&right, ComparisonVsZero::Eq, scope);
    }
    if left.is_zero() && operator == &Lt {
        return convert_expression_vs_zero(&right, ComparisonVsZero::Gt, scope);
    }
    if right.is_zero() && operator == &Eq {
        return convert_expression_vs_zero(&left, ComparisonVsZero::Eq, scope);
    }
    if right.is_zero() && operator == &Gt {
        return convert_expression_vs_zero(&left, ComparisonVsZero::Gt, scope);
    }

    if right.is_null() && operator == &Eq {
        return SqlConditionSetEntry::Expression(sql::value_is_null(left.render(scope)));
    }
    if right.is_null() && operator == &Neq {
        return SqlConditionSetEntry::Expression(sql::value_is_not_null(left.render(scope)));
    }
    if left.is_null() && operator == &Eq {
        return SqlConditionSetEntry::Expression(sql::value_is_null(right.render(scope)));
    }
    if left.is_null() && operator == &Neq {
        return SqlConditionSetEntry::Expression(sql::value_is_not_null(right.render(scope)));
    }

    let expr = |s: String| SqlConditionSetEntry::Expression(s);

    let plain_comparison = |op: &str, scope: &mut Scope| {
        expr(format!(
            "{} {} {}",
            left.render(scope),
            op,
            right.render(scope)
        ))
    };

    let match_regex = |is_positive: bool, scope: &mut Scope| {
        expr(scope.options.dialect.match_regex(
            &left.render(scope),
            &right.render(scope),
            is_positive,
            &RegExFlags {
                is_case_sensitive: false,
            },
        ))
    };

    match operator {
        Eq => plain_comparison(sql::EQ, scope),
        Gt => plain_comparison(sql::GT, scope),
        Gte => plain_comparison(sql::GTE, scope),
        Lt => plain_comparison(sql::LT, scope),
        Lte => plain_comparison(sql::LTE, scope),
        Like => plain_comparison(sql::LIKE, scope),
        Neq => plain_comparison(sql::NEQ, scope),
        NLike => plain_comparison(sql::NLIKE, scope),
        Match => match_regex(true, scope),
        NMatch => match_regex(false, scope),
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
    expr: &Expression,
    cmp: ComparisonVsZero,
    scope: &mut Scope,
) -> SqlConditionSetEntry {
    let fallback = |scope: &mut Scope| {
        let rendered_expr = expr.render(scope);
        let op = match cmp {
            ComparisonVsZero::Eq => sql::EQ,
            ComparisonVsZero::Gt => sql::GT,
        };
        SqlConditionSetEntry::Expression(format!("{} {} {}", rendered_expr, op, 0))
    };
    if expr.compositions.len() > 0 {
        return fallback(scope);
    }
    let Value::Path(path_parts) = &expr.base else { return fallback(scope) };
    let Ok(clarified_path) = clarify_path(path_parts.clone(), scope) else { return fallback(scope) };
    let ClarifiedPathTail::ChainToMany((chain, None)) = clarified_path.tail else {
        return fallback(scope)
    };
    let join_result =
        scope.join_chain_to_many(&clarified_path.head, chain, None, vec![], cmp.into());
    let Ok(simple_expr) = join_result else { return fallback(scope) };
    // We're confident that `simple_expr` doesn't have any compositions because we
    // checked that `expr` doesn't have any above.
    let rendered_expr = simple_expr.base.render(scope);
    let rendered_cmp = match cmp {
        ComparisonVsZero::Eq => sql::value_is_null(rendered_expr),
        ComparisonVsZero::Gt => sql::value_is_not_null(rendered_expr),
    };
    SqlConditionSetEntry::Expression(rendered_cmp)
}

fn expand_comparison(comparison: &Comparison) -> SimpleConditionSetEntry {
    use ComparisonPart::{Expression as Expr, ExpressionSet as ExprSet};
    let make_comparison = |left: Expression, right: Expression| {
        SimpleConditionSetEntry::new_comparison(left, comparison.operator, right)
    };
    let make_set = SimpleConditionSetEntry::new_set;
    // All the `clone()` calls in here are kind of unfortunate. Cloning an expression is not
    // necessarily cheap because the expression could be quite deep. In theory, we could perform
    // this expansion, after the expression is rendered, in which case we'd be cloning strings
    // instead. Holding references to the objects instead of cloning them would be nice although
    // it seems like that could get messy. We could consider attempting to eliminate these clone
    // calls if we find that this is a performance bottleneck.
    match (&comparison.left, &comparison.right) {
        (Expr(l), Expr(r)) => make_comparison(l.clone(), r.clone()),
        (ExprSet(l), Expr(r)) => make_set(
            l.conjunction,
            l.entries
                .iter()
                .map(|e| make_comparison(e.clone(), r.clone()))
                .collect(),
        ),
        (Expr(l), ExprSet(r)) => make_set(
            r.conjunction,
            r.entries
                .iter()
                .map(|e| make_comparison(l.clone(), e.clone()))
                .collect(),
        ),
        (ExprSet(l), ExprSet(r)) => make_set(
            l.conjunction,
            l.entries
                .iter()
                .map(|l_exp| {
                    make_set(
                        r.conjunction,
                        r.entries
                            .iter()
                            .map(|r_exp| make_comparison(l_exp.clone(), r_exp.clone()))
                            .collect(),
                    )
                })
                .collect(),
        ),
    }
}
