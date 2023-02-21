use crate::{
    dialects::dialect::Dialect,
    rendering::{Render, RenderingContext},
    syntax_tree::{ConditionSet, Conjunction, NullsSort, SortDirection},
};

#[derive(Debug)]
pub struct Select {
    pub base_table: String,
    pub columns: Vec<Column>,
    pub ctes: Vec<Cte>,
    pub joins: Vec<Join>,
    pub condition_set: SqlConditionSet,
    pub sorting: Vec<SortEntry>,
    pub grouping: Vec<SqlExpression>,
}

#[derive(Debug)]
pub struct Column {
    pub expression: SqlExpression,
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
    pub condition_set: SqlConditionSet,
}

#[derive(Debug, Default)]
pub struct SqlConditionSet {
    pub conjunction: Conjunction,
    pub entries: Vec<SqlConditionSetEntry>,
}

#[derive(Debug)]
pub enum SqlConditionSetEntry {
    Expression(SqlExpression),
    ConditionSet(SqlConditionSet),
}

#[derive(Debug)]
pub struct SortEntry {
    pub expression: SqlExpression,
    pub direction: SortDirection,
    pub nulls_sort: NullsSort,
}

type SqlExpression = String;

impl From<String> for Select {
    fn from(base_table: String) -> Self {
        Self {
            base_table,
            columns: vec![],
            ctes: vec![],
            joins: vec![],
            condition_set: SqlConditionSet::default(),
            sorting: vec![],
            grouping: vec![],
        }
    }
}

impl Render for Select {
    fn render<D: Dialect>(&self, cx: &mut RenderingContext<D>) -> String {
        let mut rendered = String::new();
        rendered.push_str("SELECT\n");
        cx.indented(|cx| rendered.push_str(&self.columns.render(cx)));
        rendered.push_str("FROM ");
        rendered.push_str(cx.dialect.quote_identifier(&self.base_table).as_str());
        rendered.push_str(&self.joins.render(cx));
        if self.condition_set.entries.len() > 0 {
            rendered.push_str("\nWHERE\n");
            cx.indented(|cx| rendered.push_str(&self.condition_set.render(cx)))
        }
        rendered
    }
}

impl Render for Vec<Join> {
    fn render<D: Dialect>(&self, cx: &mut RenderingContext<D>) -> String {
        let mut rendered = String::new();
        for join in self.iter() {
            rendered.push_str("\n");
            rendered.push_str(cx.get_indentation().as_str());
            rendered.push_str(&join.render(cx));
        }
        rendered
    }
}

impl Render for Join {
    fn render<D: Dialect>(&self, cx: &mut RenderingContext<D>) -> String {
        let quoted_table = cx.dialect.quote_identifier(&self.table);
        let table_expr = if self.alias == self.table {
            quoted_table
        } else {
            let quoted_alias = cx.dialect.quote_identifier(&self.alias);
            format!("{} AS {}", quoted_table, quoted_alias)
        };
        let condition_set = self.condition_set.render(cx);
        format!("LEFT JOIN {} ON {}", table_expr, condition_set)
    }
}

impl Render for Vec<Column> {
    fn render<D: Dialect>(&self, cx: &mut RenderingContext<D>) -> String {
        let mut rendered = String::new();
        if self.len() == 0 {
            rendered.push_str(cx.get_indentation().as_str());
            rendered.push('*');
            rendered.push('\n');
            return rendered;
        }
        for (i, column) in self.iter().enumerate() {
            if i > 0 {
                rendered.push_str(",\n");
            }
            rendered.push_str(cx.get_indentation().as_str());
            rendered.push_str(&column.render(cx));
        }
        rendered.push('\n');
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

impl Render for SqlConditionSet {
    fn render<D: Dialect>(&self, cx: &mut RenderingContext<D>) -> String {
        let mut rendered = String::new();
        if self.entries.len() == 0 {
            return rendered;
        }
        for (i, entry) in self.entries.iter().enumerate() {
            if i > 0 {
                rendered.push(' ');
                rendered.push_str(self.conjunction.render(cx).as_str());
                rendered.push('\n');
            }
            rendered.push_str(cx.get_indentation().as_str());
            rendered.push_str(&entry.render(cx));
        }
        rendered
    }
}

impl Render for SqlConditionSetEntry {
    fn render<D: Dialect>(&self, cx: &mut RenderingContext<D>) -> String {
        let mut rendered = String::new();
        match self {
            SqlConditionSetEntry::Expression(expression) => {
                rendered.push_str(expression.as_str());
            }
            SqlConditionSetEntry::ConditionSet(condition_set) => {
                if condition_set.entries.len() == 0 {
                    return rendered;
                }
                rendered.push_str("(\n");
                cx.indented(|cx| rendered.push_str(&condition_set.render(cx)));
                rendered.push('\n');
                rendered.push_str(cx.get_indentation().as_str());
                rendered.push_str(")\n");
            }
        }
        rendered
    }
}
