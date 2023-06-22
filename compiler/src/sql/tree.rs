use querydown_parser::ast::{NullsSort, SortDirection};

pub use super::expr::{SqlExpr, SqlExprPrecedence};

#[derive(Debug)]
pub struct Select {
    pub base_table: String,
    pub columns: Vec<Column>,
    pub ctes: Vec<Cte>,
    pub joins: Vec<Join>,
    pub conditions: SqlExpr,
    pub sorting: Vec<SortEntry>,
    pub grouping: Vec<SqlExpr>,
}

#[derive(Debug)]
pub struct Column {
    pub expr: SqlExpr,
    pub alias: Option<String>,
}

impl Column {
    pub fn new(expr: SqlExpr, alias: Option<String>) -> Self {
        Self { expr, alias }
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
    pub conditions: SqlExpr,
    pub join_type: JoinType,
}

#[derive(Debug)]
pub enum JoinType {
    Inner,
    LeftOuter,
}

#[derive(Debug)]
pub struct SortEntry {
    pub expr: SqlExpr,
    pub direction: SortDirection,
    pub nulls_sort: NullsSort,
}

impl From<String> for Select {
    fn from(base_table: String) -> Self {
        Self {
            base_table,
            columns: vec![],
            ctes: vec![],
            joins: vec![],
            conditions: SqlExpr::default(),
            sorting: vec![],
            grouping: vec![],
        }
    }
}
