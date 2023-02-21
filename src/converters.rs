use crate::{
    dialects::dialect::Dialect,
    rendering::{DecontextualizedExpression, JoinTree, Render, RenderingContext, SimpleExpression},
    schema::{
        chain::ChainToOne,
        links::{Link, LinkToOne},
        schema::Table,
    },
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
            convert_simple_comparison(comparison, cx)
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

fn convert_simple_comparison<D: Dialect>(
    s: &SimpleComparison,
    cx: &mut RenderingContext<D>,
) -> SqlConditionSetEntry {
    // When we see that we're comparing an expression equal to zero or greater to zero, then we
    // hand off the conversion to the context because, depending on the expression, the context
    // may choose to handle this condition via a join instead of a condition set entry. In that
    // case we'll receive an empty SqlConditionSet back, and that will get filtered out later on.
    if s.left.is_zero() && s.operator == Operator::Eq {
        return convert_expression_eq_0(&s.right, cx);
    }
    if s.left.is_zero() && s.operator == Operator::Lt {
        return convert_expression_gt_0(&s.right, cx);
    }
    if s.right.is_zero() && s.operator == Operator::Eq {
        return convert_expression_eq_0(&s.left, cx);
    }
    if s.right.is_zero() && s.operator == Operator::Gt {
        return convert_expression_gt_0(&s.left, cx);
    }
    SqlConditionSetEntry::Expression(format!(
        "{} {} {}",
        s.left.render(cx),
        s.operator.render(cx),
        s.right.render(cx)
    ))
}

fn convert_expression_gt_0<D: Dialect>(
    expr: &Expression,
    cx: &mut RenderingContext<D>,
) -> SqlConditionSetEntry {
    // TODO_CODE
    todo!()
}

fn convert_expression_eq_0<D: Dialect>(
    expr: &Expression,
    cx: &mut RenderingContext<D>,
) -> SqlConditionSetEntry {
    // TODO_CODE
    todo!()
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
        // TODO_ERR handle error when attempting to set a slot value which contains an empty slot
        // value
        let slot_value = combine_expression_with_slot(left_expr, cx).unwrap();
        cx.with_slot_value(slot_value, |cx| SqlConditionSet {
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

// Try to remove the Slot from the expression by incorporating the slot value from the context.
pub fn combine_expression_with_slot<D: Dialect>(
    expr: &Expression,
    cx: &RenderingContext<D>,
) -> Result<DecontextualizedExpression, &'static str> {
    match &expr.base {
        ContextualValue::Value(value) => Ok(DecontextualizedExpression {
            base: value.clone(),
            compositions: expr.compositions.clone(),
        }),
        ContextualValue::Slot => {
            let slot_value = cx.get_slot_value();
            match slot_value {
                None => Err("Cannot use slot outside of a scoped conditional."),
                Some(slot_expr) => {
                    let mut compositions = slot_expr.compositions.clone();
                    compositions.extend_from_slice(&expr.compositions);
                    let base = slot_expr.base.clone();
                    Ok(DecontextualizedExpression { base, compositions })
                }
            }
        }
    }
}

pub fn simplify_expression<D: Dialect>(
    expr: DecontextualizedExpression,
    cx: &mut RenderingContext<D>,
) -> SimpleExpression {
    match expr.base {
        Value::Literal(literal) => SimpleExpression {
            base: literal,
            compositions: expr.compositions,
        },
        // TODO_ERR handle error
        Value::Path(path) => simplify_path_expression(path, expr.compositions, cx).unwrap(),
    }
}

fn simplify_path_expression<D: Dialect>(
    path: Path,
    compositions: Vec<Composition>,
    cx: &mut RenderingContext<D>,
) -> Result<SimpleExpression, String> {
    match path {
        Path::ToOne(parts) => {
            let clarified_path = clarify_path_to_one(parts, cx)?;
            let table_name = if let Some(chain) = clarified_path.chain {
                cx.join_chain_to_one(&chain)
            } else {
                cx.get_base_table().name.clone()
            };
            let column_name = clarified_path.final_column_name;
            Ok(SimpleExpression {
                base: Literal::TableColumnReference(table_name, column_name),
                compositions,
            })
        }
        Path::ToMany(parts) => simplify_path_to_many_expression(parts, compositions, cx),
    }
}

struct ClarifiedPathToOne {
    chain: Option<ChainToOne>,
    final_column_name: String,
}

fn clarify_path_to_one<D: Dialect>(
    parts: Vec<PathPartToOne>,
    cx: &RenderingContext<D>,
) -> Result<ClarifiedPathToOne, String> {
    let mut chain: Option<ChainToOne> = None;
    let mut final_column_name_opt: Option<String> = None;
    let mut base_table: Option<&Table> = Some(cx.get_base_table());
    for part in parts {
        match part {
            PathPartToOne::Column(column_name) => {
                if let Some(table) = base_table {
                    if let Some(column_id) = table.column_lookup.get(&column_name).copied() {
                        final_column_name_opt = Some(column_name);
                        if let Some(link) = table.forward_links_to_one.get(&column_id).copied() {
                            let link_to_one = LinkToOne::ForwardLinkToOne(link);
                            chain = chain.map_or_else(
                                || ChainToOne::new(&link_to_one).ok(),
                                |c| c.with(&link_to_one).ok(),
                            );
                            base_table = cx.schema.tables.get(&link.get_end().table_id);
                        } else {
                            // If the current column is not an FK column, then we need to ensure
                            // that no more columns can be used in the path expression.
                            base_table = None;
                        }
                    } else {
                        return Err(format!(
                            "Column {} not found within table {}.",
                            column_name, table.name
                        ));
                    }
                } else {
                    return Err("Non-FK columns can only appear at the end of a path.".to_owned());
                }
            }
            PathPartToOne::TableWithOne(table_name) => {
                todo!()
            }
        }
    }
    if let Some(final_column_name) = final_column_name_opt {
        Ok(ClarifiedPathToOne {
            chain,
            final_column_name,
        })
    } else {
        Err("Scalar path expressions must specify a column name at the end.".to_owned())
    }
}

fn simplify_path_to_many_expression<D: Dialect>(
    parts: Vec<GeneralPathPart>,
    compositions: Vec<Composition>,
    cx: &mut RenderingContext<D>,
) -> Result<SimpleExpression, String> {
    // TODO_CODE finish this function
    todo!()
}

pub fn convert_join_tree<D: Dialect>(tree: &JoinTree, cx: &RenderingContext<D>) -> Vec<Join> {
    let mut joins: Vec<Join> = vec![];
    for (link, subtree) in tree.get_dependents().iter() {
        let start = link.get_start();
        let starting_table_id = start.table_id;
        let starting_table = cx.schema.tables.get(&starting_table_id).unwrap();
        let starting_column_id = start.column_id;
        let starting_column = starting_table.columns.get(&starting_column_id).unwrap();
        let starting_alias = tree.get_alias();

        let end = link.get_end();
        let ending_table_id = end.table_id;
        let ending_table = cx.schema.tables.get(&ending_table_id).unwrap();
        let ending_column_id = end.column_id;
        let ending_column = ending_table.columns.get(&ending_column_id).unwrap();
        let ending_alias = subtree.get_alias();

        let condition = format!(
            "{} = {}",
            cx.dialect
                .table_column(starting_alias, &starting_column.name),
            cx.dialect.table_column(&ending_alias, &ending_column.name),
        );
        joins.push(Join {
            table: cx.schema.tables.get(&ending_table_id).unwrap().name.clone(),
            alias: ending_alias.to_owned(),
            condition_set: SqlConditionSet {
                conjunction: Conjunction::And,
                entries: vec![SqlConditionSetEntry::Expression(condition)],
            },
        });
        joins.extend(convert_join_tree(subtree, cx));
    }
    joins
}
