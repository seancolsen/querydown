use crate::{
    dialects::dialect::Dialect,
    rendering::{JoinTree, Render, RenderingContext, SimpleExpression},
    schema::{
        chain::{Chain, ChainIntersecting},
        links::{ForwardLinkToOne, GenericLink, Link, LinkToOne},
        schema::{ChainSearchBase, Table},
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

pub fn simplify_expression<D: Dialect>(
    expr: &Expression,
    cx: &mut RenderingContext<D>,
) -> SimpleExpression {
    let expr = expr.clone();
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
    parts: Vec<PathPart>,
    compositions: Vec<Composition>,
    cx: &mut RenderingContext<D>,
) -> Result<SimpleExpression, String> {
    let clarified_path = clarify_path(parts, cx)?;
    let table_name = if let Some(chain_to_one) = clarified_path.head {
        cx.join_chain_to_one(&chain_to_one)
    } else {
        cx.get_base_table().name.clone()
    };
    match clarified_path.tail {
        ClarifiedPathTail::Column(column_name) => Ok(SimpleExpression {
            base: Literal::TableColumnReference(table_name, column_name),
            compositions,
        }),
        ClarifiedPathTail::ChainToMany(_) => todo!(),
    }
}

struct ClarifiedPath {
    head: Option<Chain<LinkToOne>>,
    tail: ClarifiedPathTail,
}

enum ClarifiedPathTail {
    Column(String),
    ChainToMany(Chain<GenericLink>),
}

impl ClarifiedPathTail {
    fn with_column(&self, column: String) -> Self {
        match self {
            Self::Column(_) => Self::Column(column),
            Self::ChainToMany(_) => todo!(),
        }
    }
}

/// Error messages
mod msg {
    pub fn no_current_table() -> String {
        "Non-FK columns can only appear at the end of a path.".to_string()
    }

    pub fn col_not_in_table(column_name: &str, table_name: &str) -> String {
        format!("Column {column_name} not found within table {table_name}.")
    }

    pub fn no_path_parts() -> String {
        "Cannot build a LinkedPath without any PathParts".to_string()
    }
}

fn clarify_path<D: Dialect>(
    parts: Vec<PathPart>,
    cx: &RenderingContext<D>,
) -> Result<ClarifiedPath, String> {
    let linked_path = build_linked_path(parts, cx)?;
    println!("linked_path: {:#?}", linked_path);
    todo!()
}

#[derive(Debug)]
enum LinkedPath {
    Column(String),
    Chain(Chain<GenericLink>),
    ChainWithColumn(Chain<GenericLink>, String),
}

fn build_linked_path<D: Dialect>(
    parts: Vec<PathPart>,
    cx: &RenderingContext<D>,
) -> Result<LinkedPath, String> {
    let mut is_first = true;
    let mut current_table_opt: Option<&Table> = Some(cx.get_base_table());
    let mut prev_fw_link_to_one: Option<ForwardLinkToOne> = None;
    let mut chain_opt: Option<Chain<GenericLink>> = None;
    let mut final_column_name: Option<String> = None;
    for part in parts {
        if !is_first {
            if let Some(link) = prev_fw_link_to_one {
                let link_to_one = GenericLink::ForwardLinkToOne(link);
                chain_opt = match chain_opt {
                    Some(mut c) => {
                        c.try_append(link_to_one)?;
                        Some(c)
                    }
                    None => Some(Chain::try_new(link_to_one, ChainIntersecting::Allowed)?),
                };
                current_table_opt = cx.schema.tables.get(&link.get_end().table_id);
            }
        }
        let current_table = current_table_opt.ok_or_else(msg::no_current_table)?;
        match part {
            PathPart::Column(column_name) => {
                let column_id = current_table
                    .column_lookup
                    .get(&column_name)
                    .copied()
                    .ok_or_else(|| msg::col_not_in_table(&column_name, &current_table.name))?;
                prev_fw_link_to_one = current_table.forward_links_to_one.get(&column_id).copied();
                final_column_name = Some(column_name);
            }
            PathPart::TableWithOne(table_name) => {
                todo!()
            }
            PathPart::TableWithMany(table_with_many) => {
                let base = ChainSearchBase::TableId(current_table.id);
                let mut new_chain =
                    cx.schema
                        .get_chain_to_table_with_many(base, &table_with_many, None)?;
                new_chain.allow_intersecting();
                current_table_opt = cx.schema.tables.get(&new_chain.get_ending_table_id());
                chain_opt = match chain_opt {
                    Some(mut chain) => {
                        chain.try_connect(new_chain)?;
                        Some(chain)
                    }
                    None => Some(new_chain),
                };
                final_column_name = None;
            }
        };
        is_first = false;
    }
    match (chain_opt, final_column_name) {
        (Some(chain), Some(column_name)) => Ok(LinkedPath::ChainWithColumn(chain, column_name)),
        (Some(chain), None) => Ok(LinkedPath::Chain(chain)),
        (None, Some(column_name)) => Ok(LinkedPath::Column(column_name)),
        (None, None) => Err(msg::no_path_parts()),
    }
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
