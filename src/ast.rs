#[derive(Debug, PartialEq)]
pub struct Query {
    pub base_table: String,
    pub transformations: Vec<Transformation>,
}

#[derive(Debug, PartialEq)]
pub struct Transformation {
    pub condition_set: ConditionSet,
    pub column_layout: ColumnLayout,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ConditionSet {
    pub conjunction: Conjunction,
    pub entries: Vec<ConditionSetEntry>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum Conjunction {
    #[default]
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConditionSetEntry {
    Condition(Condition),
    ConditionSet(ConditionSet),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Condition {
    pub left: Expression,
    pub operator: Operator,
    pub right: Expression,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Operator {
    Eq,
    Gt,
    Gte,
    Lt,
    Lte,
    Like,
    Neq,
    NLike,
    RLike,
    NRLike,
}

#[derive(Debug, Default, PartialEq)]
pub struct ColumnLayout {
    pub column_specs: Vec<ColumnSpec>,
}

#[derive(Debug, PartialEq)]
pub struct ColumnSpec {
    pub column_control: ColumnControl,
    pub expression: Expression,
    pub alias: Option<String>,
}

#[derive(Debug, PartialEq)]
pub struct ColumnControl {
    pub sort: Option<SortSpec>,
    pub is_group_by: bool,
    pub is_partition_by: bool,
    pub is_hidden: bool,
}

impl Default for ColumnControl {
    fn default() -> Self {
        ColumnControl {
            sort: None,
            is_group_by: false,
            is_partition_by: false,
            is_hidden: false,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct SortSpec {
    /// A SortSpec without an ordinal means that we'd like to sort by the column, but we want to
    /// infer the ordinality from the ColumnSpec's position within the ColumnLayout.
    pub ordinal: Option<u32>,
    pub direction: SortDirection,
    pub nulls_sort: NullsSort,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub enum NullsSort {
    First,
    #[default]
    Last,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub enum SortDirection {
    #[default]
    Asc,
    Desc,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expression {
    pub base: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Path(Path),
    String(String),
    Number(String),
    Null,
    True,
    False,
    Now,
    Infinity,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    pub parts: Vec<PathPart>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PathPart {
    LocalColumn(String),
    LinkToOne(String),
    LinkToMany(LinkToMany),
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinkToMany {
    pub table: String,
    pub condition_set: ConditionSet,
    pub column: Option<String>,
}
