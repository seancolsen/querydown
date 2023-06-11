use crate::syntax_tree::{Conjunction, NullsSort, SortDirection};

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

impl Select {
    pub fn simplify(&mut self) {
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

impl SqlConditionSet {
    pub fn merge(&mut self, mut new: Self) {
        use Conjunction::*;
        if new.entries.len() == 0 {
            return;
        }
        match (self.conjunction, new.conjunction) {
            (And, And) => self.entries.extend(new.entries),
            (Or, Or) => self.entries.extend(new.entries),
            (And, Or) => self.entries.push(SqlConditionSetEntry::ConditionSet(new)),
            (Or, And) => {
                std::mem::swap(self, &mut new);
                self.entries.push(SqlConditionSetEntry::ConditionSet(new));
            }
        }
    }

    pub fn simplify(&mut self) {
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

impl SqlConditionSetEntry {
    pub fn simplify(&mut self) {
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
