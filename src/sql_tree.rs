use crate::{
    dialects::dialect::Dialect,
    rendering::{Render, RenderingContext},
    syntax_tree::{Conjunction, NullsSort, SortDirection},
};

pub trait Simplify {
    fn simplify(&mut self);
}

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

impl Simplify for Select {
    fn simplify(&mut self) {
        self.condition_set.simplify();
    }
}

#[derive(Debug)]
pub struct Column {
    pub expression: SqlExpression,
    pub alias: Option<String>,
}

impl Column {
    pub fn new(expression: SqlExpression, alias: Option<String>) -> Self {
        Self { expression, alias }
    }
}

#[derive(Debug)]
pub struct Cte {
    pub alias: String,
    pub select: Select,
    pub purpose: CtePurpose,
    /// The name of the column in the other table to which this CTE is joined. We don't need the
    /// table name because we already have that from the JoinTree. This column name is usually the
    /// primary key of that table.
    pub join_column_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    pub join_type: JoinType,
}

#[derive(Debug)]
pub enum JoinType {
    Inner,
    LeftOuter,
}

#[derive(Debug, Default)]
pub struct SqlConditionSet {
    pub conjunction: Conjunction,
    pub entries: Vec<SqlConditionSetEntry>,
}
impl Simplify for SqlConditionSet {
    fn simplify(&mut self) {
        let mut new_entries = Vec::new();
        for mut entry in std::mem::take(&mut self.entries) {
            entry.simplify();
            if !entry.is_empty() {
                new_entries.push(entry);
            }
        }
        self.entries = new_entries;
    }
}

#[derive(Debug)]
pub enum SqlConditionSetEntry {
    Expression(SqlExpression),
    ConditionSet(SqlConditionSet),
}

impl Simplify for SqlConditionSetEntry {
    fn simplify(&mut self) {
        match self {
            SqlConditionSetEntry::Expression(_) => {}
            SqlConditionSetEntry::ConditionSet(condition_set) => {
                condition_set.simplify();
            }
        }
    }
}

impl SqlConditionSetEntry {
    pub fn empty() -> Self {
        Self::ConditionSet(SqlConditionSet::default())
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Self::Expression(_) => false,
            Self::ConditionSet(condition_set) => condition_set.entries.len() == 0,
        }
    }
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
        let indentation = cx.get_indentation();
        let mut rendered = String::new();
        rendered.push_str(&self.ctes.render(cx));
        rendered.push_str(&indentation);
        rendered.push_str("SELECT\n");
        cx.indented(|cx| rendered.push_str(&self.columns.render(cx)));
        rendered.push_str(&indentation);
        rendered.push_str("FROM ");
        rendered.push_str(cx.dialect.quote_identifier(&self.base_table).as_str());
        rendered.push_str(&self.joins.render(cx));
        if self.condition_set.entries.len() > 0 {
            rendered.push_str("\n");
            rendered.push_str(&indentation);
            rendered.push_str("WHERE\n");
            cx.indented(|cx| rendered.push_str(&self.condition_set.render(cx)))
        }
        if self.grouping.len() > 0 {
            rendered.push_str("\n");
            rendered.push_str(&indentation);
            rendered.push_str("GROUP BY ");
            rendered.push_str(&self.grouping.join(", "));
        }
        rendered
    }
}

impl Render for Vec<Cte> {
    fn render<D: Dialect>(&self, cx: &mut RenderingContext<D>) -> String {
        let mut rendered = String::new();
        if self.len() == 0 {
            return rendered;
        }
        rendered.push_str("WITH ");
        let mut is_first = true;
        for cte in self {
            rendered.push_str(&cx.dialect.quote_identifier(&cte.alias));
            rendered.push_str(" AS (\n");
            cx.indented(|cx| rendered.push_str(&cte.select.render(cx)));
            rendered.push_str("\n");
            rendered.push_str(")");
            if !is_first {
                rendered.push_str(",");
            }
            rendered.push_str("\n");
            is_first = false;
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
        let condition_set = cx.indented(|cx| self.condition_set.render(cx));
        let join_type = match self.join_type {
            JoinType::Inner => "JOIN",
            JoinType::LeftOuter => "LEFT JOIN",
        };
        format!("{join_type} {table_expr} ON\n{condition_set}")
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
