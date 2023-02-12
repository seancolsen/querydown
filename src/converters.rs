use crate::{
    dialects::dialect::Dialect,
    rendering::{Render, RenderingContext},
    sql_tree::*,
    syntax_tree::*,
};

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

impl Render for SimpleComparison {
    fn render<D: Dialect>(&self, cx: &mut RenderingContext<D>) -> String {
        format!(
            "{} {} {}",
            self.left.render(cx),
            self.operator.render(cx),
            self.right.render(cx)
        )
    }
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

pub fn convert_condition_set<D: Dialect>(
    condition_set: &ConditionSet,
    cx: &mut RenderingContext<D>,
) -> SqlConditionSet {
    SqlConditionSet {
        conjunction: condition_set.conjunction,
        entries: condition_set
            .entries
            .iter()
            .map(|entry| convert_condition_set_entry(entry, cx))
            .collect(),
    }
}

fn convert_condition_set_entry<D: Dialect>(
    condition_set_entry: &ConditionSetEntry,
    cx: &mut RenderingContext<D>,
) -> SqlConditionSetEntry {
    match condition_set_entry {
        ConditionSetEntry::Comparison(comparison) => convert_comparison(comparison, cx),
        ConditionSetEntry::ConditionSet(condition_set) => {
            SqlConditionSetEntry::ConditionSet(convert_condition_set(condition_set, cx))
        }
        ConditionSetEntry::ScopedConditional(s) => convert_scoped_conditional(s, cx),
        ConditionSetEntry::Has(h) => todo!(),
    }
}

fn convert_comparison<D: Dialect>(
    comparison: &Comparison,
    cx: &mut RenderingContext<D>,
) -> SqlConditionSetEntry {
    convert_simple_condition_set_entry(&expand_comparison(comparison), cx)
}

fn convert_simple_condition_set_entry<D: Dialect>(
    entry: &SimpleConditionSetEntry,
    cx: &mut RenderingContext<D>,
) -> SqlConditionSetEntry {
    match entry {
        SimpleConditionSetEntry::SimpleComparison(comparison) => {
            SqlConditionSetEntry::Expression(comparison.render(cx))
        }
        SimpleConditionSetEntry::SimpleConditionSet(condition_set) => {
            SqlConditionSetEntry::ConditionSet(SqlConditionSet {
                conjunction: condition_set.conjunction,
                entries: condition_set
                    .entries
                    .iter()
                    .map(|entry| convert_simple_condition_set_entry(entry, cx))
                    .collect(),
            })
        }
    }
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

fn convert_scoped_conditional<D: Dialect>(
    scoped_conditional: &ScopedConditional,
    cx: &mut RenderingContext<D>,
) -> SqlConditionSetEntry {
    let ScopedConditional { left, right } = scoped_conditional;
    let mut convert_with_left_expr = |left_expr: &Expression| -> SqlConditionSet {
        cx.with_slot_value(left_expr.clone(), |cx| SqlConditionSet {
            conjunction: right.conjunction,
            entries: right
                .entries
                .iter()
                .map(|entry| convert_condition_set_entry(entry, cx))
                .collect(),
        })
    };
    let condition_set = match left {
        ComparisonPart::Expression(expr) => convert_with_left_expr(expr),
        ComparisonPart::ExpressionSet(expr_set) => SqlConditionSet {
            conjunction: expr_set.conjunction,
            entries: expr_set
                .entries
                .iter()
                .map(|expr| SqlConditionSetEntry::ConditionSet(convert_with_left_expr(expr)))
                .collect(),
        },
    };
    SqlConditionSetEntry::ConditionSet(condition_set)
}
