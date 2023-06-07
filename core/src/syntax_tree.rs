#[derive(Debug, PartialEq)]
pub struct Query {
    pub base_table: String,
    pub transformations: Vec<Transformation>,
}

#[derive(Debug, PartialEq, Default)]
pub struct Transformation {
    pub condition_set: ConditionSet,
    pub column_layout: ColumnLayout,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ConditionSet {
    pub conjunction: Conjunction,
    pub entries: Vec<ConditionSetEntry>,
}

impl ConditionSet {
    pub fn is_empty(&self) -> bool {
        self.entries.iter().all(|entry| entry.is_empty())
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExpressionSet {
    pub conjunction: Conjunction,
    pub entries: Vec<Expression>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum Conjunction {
    #[default]
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConditionSetEntry {
    Comparison(Comparison),
    ConditionSet(ConditionSet),
}

impl ConditionSetEntry {
    pub fn is_empty(&self) -> bool {
        match self {
            ConditionSetEntry::Comparison(_) => false,
            ConditionSetEntry::ConditionSet(condition_set) => condition_set.is_empty(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Comparison {
    pub left: ComparisonPart,
    pub operator: Operator,
    pub right: ComparisonPart,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComparisonPart {
    Expression(Expression),
    ExpressionSet(ExpressionSet),
}

impl From<ComparisonPart> for ExpressionSet {
    fn from(part: ComparisonPart) -> Self {
        match part {
            ComparisonPart::Expression(expr) => ExpressionSet {
                entries: vec![expr],
                ..Default::default()
            },
            ComparisonPart::ExpressionSet(expr_set) => expr_set,
        }
    }
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

#[derive(Debug, Default, PartialEq)]
pub struct ColumnLayout {
    pub column_specs: Vec<ColumnSpec>,
}

#[derive(Debug, PartialEq)]
pub struct ColumnSpec {
    pub expression: Expression,
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

#[derive(Debug, Clone, PartialEq)]
pub struct Expression {
    pub base: Value,
    pub compositions: Vec<Composition>,
}

impl Expression {
    pub fn is_zero(&self) -> bool {
        let base_is_zero = match self.base {
            Value::Literal(Literal::Number(ref num)) => num == "0",
            _ => false,
        };
        base_is_zero && self.compositions.is_empty()
    }

    pub fn is_null(&self) -> bool {
        let base_is_null = match self.base {
            Value::Literal(Literal::Null) => true,
            _ => false,
        };
        base_is_null && self.compositions.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Path(Vec<PathPart>),
    Literal(Literal),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    String(String),
    Number(String),
    Date(Date),
    Duration(Duration),
    Null,
    True,
    False,
    Now,
    Infinity,
    /// (table_name, column_name)
    TableColumnReference(String, String),
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
pub struct Composition {
    pub function: Function,
    pub argument: Option<Expression>,
}

impl Composition {
    pub fn count() -> Self {
        Composition {
            function: Function {
                name: "count".to_string(),
                dimension: FunctionDimension::Aggregate,
            },
            argument: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub dimension: FunctionDimension,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FunctionDimension {
    Scalar,
    Aggregate,
}
