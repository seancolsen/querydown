use crate::tokens::LITERAL_NULL;

#[derive(Debug, PartialEq)]
pub struct Query {
    pub base_table: String,
    pub transformations: Vec<Transformation>,
}

#[derive(Debug, PartialEq, Default)]
pub struct Transformation {
    pub conditions: ConditionSet,
    pub result_columns: Vec<ResultColumnStatement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(String),
    Date(Date),
    Duration(Duration),
    String(String),
    Variable(String),
    Path(Vec<PathPart>),
    ConditionSet(ConditionSet),
    HasQuantity(HasQuantity),
    Call(Call),
    Product(Box<Expr>, Box<Expr>),
    Quotient(Box<Expr>, Box<Expr>),
    Sum(Box<Expr>, Box<Expr>),
    Difference(Box<Expr>, Box<Expr>),
    Comparison(Box<Comparison>),
}

impl Expr {
    pub fn zero() -> Self {
        Expr::Number("0".to_string())
    }

    pub fn is_zero(&self) -> bool {
        match self {
            Expr::Number(ref num) => num == "0",
            _ => false,
        }
    }

    pub fn is_null(&self) -> bool {
        match self {
            Expr::Variable(ref name) => name == LITERAL_NULL,
            _ => false,
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Expr::ConditionSet(condition_set) => condition_set.is_empty(),
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Date {
    pub year: u32,
    pub month: u32,
    pub day: u32,
}

impl Date {
    pub fn to_iso(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Duration {
    pub years: f64,
    pub months: f64,
    pub weeks: f64,
    pub days: f64,
    pub hours: f64,
    pub minutes: f64,
    pub seconds: f64,
}

impl Duration {
    pub fn to_iso(&self) -> String {
        let mut result = String::new();
        if self.years != 0.0 {
            result.push_str(&format!("{}Y", self.years));
        }
        if self.months != 0.0 {
            result.push_str(&format!("{}M", self.months));
        }
        if self.weeks != 0.0 {
            result.push_str(&format!("{}W", self.weeks));
        }
        if self.days != 0.0 {
            result.push_str(&format!("{}D", self.days));
        }
        if self.hours != 0.0 || self.minutes != 0.0 || self.seconds != 0.0 {
            result.push('T');
            if self.hours != 0.0 {
                result.push_str(&format!("{}H", self.hours));
            }
            if self.minutes != 0.0 {
                result.push_str(&format!("{}M", self.minutes));
            }
            if self.seconds != 0.0 {
                result.push_str(&format!("{}S", self.seconds));
            }
        }
        if result.is_empty() {
            "PT0S".to_string()
        } else {
            result
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PathPart {
    Column(String),
    TableWithOne(String),
    TableWithMany(TableWithMany),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableWithMany {
    pub table: String,
    pub condition_set: ConditionSet,
    pub linking_column: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Comparison {
    pub left: ComparisonSide,
    pub operator: Operator,
    pub right: ComparisonSide,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComparisonSide {
    Expr(Expr),
    Expansion(ConditionSet),
    Range(Range),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Range {
    pub lower: RangeBound,
    pub upper: RangeBound,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RangeBound {
    pub expr: Expr,
    pub exclusivity: Exclusivity,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Exclusivity {
    Inclusive,
    Exclusive,
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
    Match,
    NMatch,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ConditionSet {
    pub conjunction: Conjunction,
    pub entries: Vec<Expr>,
}

impl ConditionSet {
    pub fn is_empty(&self) -> bool {
        self.entries.iter().all(|entry| entry.is_empty())
    }

    pub fn via_and(entries: Vec<Expr>) -> Self {
        ConditionSet {
            conjunction: Conjunction::And,
            entries,
        }
    }

    pub fn via_or(entries: Vec<Expr>) -> Self {
        ConditionSet {
            conjunction: Conjunction::Or,
            entries,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum Conjunction {
    #[default]
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HasQuantity {
    pub quantity: Quantity,
    pub path_parts: Vec<PathPart>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Quantity {
    AtLeastOne,
    Zero,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Call {
    pub name: String,
    pub dimension: FunctionDimension,
    pub args: Vec<Expr>,
    pub syntax: CallSyntax,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CallSyntax {
    Standalone,
    Piped,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FunctionDimension {
    Scalar,
    Aggregate,
}

#[derive(Debug, PartialEq)]
pub enum ResultColumnStatement {
    Spec(ColumnSpec),
    Glob(ColumnGlob),
}

#[derive(Debug, PartialEq)]
pub struct ColumnSpec {
    pub expr: Expr,
    pub alias: Option<String>,
    pub column_control: ColumnControl,
}

#[derive(Debug, PartialEq, Default)]
pub struct ColumnControl {
    pub sort: Option<SortSpec>,
    pub group: Option<GroupSpec>,
    pub is_partition_by: bool,
    pub is_hidden: bool,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct GroupSpec {
    /// A GroupSpec without an ordinal means that we'd like to group by the column, but we want to
    /// infer the ordinality from the ColumnSpec's position within the ColumnLayout.
    pub ordinal: Option<u32>,
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

#[derive(Debug, Default, PartialEq)]
pub struct ColumnGlob {
    pub head: Vec<PathPart>,
    pub specs: Vec<ColumnSpec>,
}
