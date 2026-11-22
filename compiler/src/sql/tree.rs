use querydown_parser::ast::{NullsSort, SortDirection, TableName};

pub use super::expr::SqlExpr;

#[derive(Debug, PartialEq)]
pub struct Select {
    /// The table (or CTE) the query reads from. A table in another schema is addressed by its
    /// qualified name, and its rows by its unqualified name (unless `base_table_alias` says
    /// otherwise).
    pub base_table: TableName,
    /// The alias under which `base_table` is addressed, when it differs from its name. A stage
    /// whose conditions reference window functions reads from a CTE in place of its base table,
    /// under the base table's own name (e.g. `FROM "qdwin_src" AS "issues"`).
    pub base_table_alias: Option<String>,
    pub columns: Vec<Column>,
    pub ctes: Vec<Cte>,
    pub joins: Vec<Join>,
    pub conditions: SqlExpr,
    pub sorting: Vec<SortEntry>,
    pub grouping: Vec<SqlExpr>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    /// Further queries whose results are combined with this one's via `UNION ALL`. The columns, sorting,
    /// limit, and offset of this select then name, sort, and paginate the union as a whole.
    pub unions: Vec<Select>,
}

#[derive(Debug, PartialEq)]
pub struct Column {
    pub expr: SqlExpr,
    pub alias: Option<String>,
}

impl Column {
    pub fn new(expr: SqlExpr, alias: Option<String>) -> Self {
        Self { expr, alias }
    }
}

#[derive(Debug, PartialEq)]
pub struct Cte {
    pub alias: String,
    pub select: Select,
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

#[derive(Debug, PartialEq)]
pub struct Join {
    /// The table (or CTE) joined, which is addressed by `alias`.
    pub table: TableName,
    pub alias: String,
    pub conditions: SqlExpr,
    pub join_type: JoinType,
}

#[derive(Debug, PartialEq)]
pub enum JoinType {
    Inner,
    LeftOuter,
}

#[derive(Debug, PartialEq)]
pub struct SortEntry {
    pub expr: SqlExpr,
    pub direction: SortDirection,
    pub nulls_sort: NullsSort,
}

impl From<String> for Select {
    /// A select from the table or CTE with the given name, in the current schema.
    fn from(base_table: String) -> Self {
        Self::from(TableName::new(base_table))
    }
}

impl From<TableName> for Select {
    fn from(base_table: TableName) -> Self {
        Self {
            base_table,
            base_table_alias: None,
            columns: vec![],
            ctes: vec![],
            joins: vec![],
            conditions: SqlExpr::default(),
            sorting: vec![],
            grouping: vec![],
            limit: None,
            offset: None,
            unions: vec![],
        }
    }
}
