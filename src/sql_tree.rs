use crate::{
    dialects::dialect::Dialect,
    rendering::{Render, RenderingContext},
    syntax_tree::{NullsSort, SortDirection},
};

#[derive(Debug)]
pub struct Select {
    pub base_table: String,
    pub columns: Vec<Column>,
    pub ctes: Vec<Cte>,
    pub joins: Vec<Join>,
    pub condition_set: ConditionSet,
    pub sorting: Vec<SortEntry>,
    pub grouping: Vec<Expression>,
}

#[derive(Debug)]
pub struct Column {
    pub expression: Expression,
    pub alias: Option<String>,
}

#[derive(Debug)]
pub struct Cte {
    pub name: String,
    pub select: Select,
    pub purpose: CtePurpose,
}

#[derive(Debug)]
pub enum CtePurpose {
    /// A CTE that is used to filter the base table on the presence of related records. It will be
    /// joined via an inner join to accomplish the filtering.
    Inclusion,
    /// A CTE that is used to filter the base table on the absence of related records. Will be
    /// joined via a left outer join, and a WHERE clause will be added to filter out rows that
    /// have a related record.
    Exclusion,
    /// A CTE that is used to supply a value used by the query. Will be joined via a left outer
    /// join.
    AggregateValue,
}

#[derive(Debug)]
pub struct Join {
    pub table: String,
    pub alias: String,
    pub condition_set: ConditionSet,
}

#[derive(Debug, Default)]
pub struct ConditionSet {
    pub conjunction: Conjunction,
    pub entries: Vec<ConditionSetEntry>,
}

#[derive(Debug)]
pub enum ConditionSetEntry {
    Expression(Expression),
    ConditionSet(ConditionSet),
}

#[derive(Debug, Default)]
pub enum Conjunction {
    #[default]
    And,
    Or,
}

#[derive(Debug)]
pub struct SortEntry {
    pub expression: Expression,
    pub direction: SortDirection,
    pub nulls_sort: NullsSort,
}

type Expression = String;

impl From<String> for Select {
    fn from(base_table: String) -> Self {
        Self {
            base_table,
            columns: vec![],
            ctes: vec![],
            joins: vec![],
            condition_set: ConditionSet::default(),
            sorting: vec![],
            grouping: vec![],
        }
    }
}

impl Render for Select {
    fn render<D: Dialect>(&self, cx: &mut RenderingContext<D>) -> String {
        let mut rendered = String::new();
        rendered.push_str("SELECT ");
        rendered.push_str(&self.columns.render(cx));
        rendered.push_str(" FROM ");
        rendered.push_str(cx.dialect.quote_identifier(&self.base_table).as_str());
        rendered
    }
}

impl Render for Vec<Column> {
    fn render<D: Dialect>(&self, cx: &mut RenderingContext<D>) -> String {
        if self.len() == 0 {
            return "*".to_string();
        }
        let mut rendered = String::new();
        for (i, column) in self.iter().enumerate() {
            if i > 0 {
                rendered.push_str(", ");
            }
            rendered.push_str(&column.render(cx));
        }
        rendered
    }
}

impl Render for Column {
    fn render<D: Dialect>(&self, cx: &mut RenderingContext<D>) -> String {
        let mut rendered = String::new();
        rendered.push_str(&self.expression);
        if let Some(alias) = &self.alias {
            rendered.push_str(" AS ");
            rendered.push_str(cx.dialect.quote_identifier(alias).as_str());
        }
        rendered
    }
}
