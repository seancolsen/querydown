use crate::{
    dialects::dialect::Dialect,
    syntax_tree::{NullsSort, SortDirection},
};

#[derive(Debug)]
pub struct Select {
    pub base_table: String,
    pub columns: Vec<Column>,
    pub ctes: Vec<Cte>,
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
            condition_set: ConditionSet::default(),
            sorting: vec![],
            grouping: vec![],
        }
    }
}

impl Select {
    pub fn render<D: Dialect>(&self, dialect: &D) -> String {
        let mut rendered = String::new();
        rendered.push_str("SELECT *");
        rendered.push_str(" FROM ");
        rendered.push_str(dialect.quote_identifier(&self.base_table).as_str());
        rendered.push_str(";");
        rendered
    }
}
