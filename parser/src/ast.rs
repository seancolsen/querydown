use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Serialize, Serializer};

use crate::tokens::LITERAL_NULL;

/// A value in Querydown's JSON-like annotation sub-language. Used to carry arbitrary,
/// application-defined annotations for result columns from the source code through to the compiler
/// output, separate from the generated SQL.
#[derive(Debug, Clone, PartialEq)]
pub enum AnnotationValue {
    Null,
    Bool(bool),
    /// The number is kept as a string (like [`Expr::Number`]) so that we preserve the exact way it
    /// was written. It is serialized as a JSON number.
    Number(String),
    String(String),
    Array(Vec<AnnotationValue>),
    /// Object entries are stored in a `Vec` (rather than a map) so that key order is preserved in
    /// the serialized output.
    Object(Vec<(String, AnnotationValue)>),
}

impl Serialize for AnnotationValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            AnnotationValue::Null => serializer.serialize_none(),
            AnnotationValue::Bool(b) => serializer.serialize_bool(*b),
            AnnotationValue::Number(n) => {
                let number: serde_json::Number = n.parse().map_err(serde::ser::Error::custom)?;
                number.serialize(serializer)
            }
            AnnotationValue::String(s) => serializer.serialize_str(s),
            AnnotationValue::Array(items) => {
                let mut seq = serializer.serialize_seq(Some(items.len()))?;
                for item in items {
                    seq.serialize_element(item)?;
                }
                seq.end()
            }
            AnnotationValue::Object(entries) => {
                let mut map = serializer.serialize_map(Some(entries.len()))?;
                for (key, value) in entries {
                    map.serialize_entry(key, value)?;
                }
                map.end()
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Query {
    /// User-defined constant definitions, written as `@name = expr` before the base table. Each
    /// binds a name to an expression whose value is inlined wherever the constant is referenced.
    pub constants: Vec<ConstantDef>,
    /// User-defined function definitions, written as `@@name = @param => body` before the base
    /// table. Each is a scalar function that can be applied (by name) like a built-in function.
    pub functions: Vec<FunctionDef>,
    /// User-defined custom comparison definitions, written as `#table.name:@param = body` before
    /// the base table. Each defines a named comparison, scoped to a table, that can be used (by
    /// name) like a real column on the left-hand side of a comparison.
    pub custom_comparisons: Vec<CustomComparisonDef>,
    /// Computed column definitions, written before the base table. Each defines a named expression
    /// scoped to a table, which can then be referenced (by name) like a real column elsewhere in the
    /// query — including within the definitions of later computed columns.
    pub computed_columns: Vec<ComputedColumn>,
    pub base_table: String,
    pub transformations: Vec<Transformation>,
}

/// A computed column definition, written as `#table.name = expr` before the query's base table.
#[derive(Debug, Clone, PartialEq)]
pub struct ComputedColumn {
    pub table: String,
    pub name: String,
    pub expr: Expr,
}

/// A user-defined constant definition, written as `@name = expr` before the query's base table. The
/// constant's value is inlined into the generated SQL wherever the constant is referenced (as
/// `@name`).
#[derive(Debug, Clone, PartialEq)]
pub struct ConstantDef {
    pub name: String,
    pub expr: Expr,
}

/// A user-defined function definition, written as `@@name = @param1 @param2 => body` before the
/// query's base table. The function is a scalar function: when applied, its arguments are bound to
/// the parameters and its body is inlined into the generated SQL.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDef {
    pub name: String,
    /// Parameter names, without the `@` sigil, in order.
    pub params: Vec<String>,
    pub body: FunctionBody,
}

/// The body of a [`FunctionDef`]: zero or more local assignments followed by a single result
/// expression. The assignments and the result expression may reference the function's parameters as
/// well as any earlier assignments.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionBody {
    pub assignments: Vec<Assignment>,
    pub expr: Expr,
}

/// A local assignment within a function body, written as `@name = expr`. It binds a name to an
/// expression for use later in the same function body.
#[derive(Debug, Clone, PartialEq)]
pub struct Assignment {
    pub name: String,
    pub expr: Expr,
}

/// A user-defined custom comparison definition, written as `#table.name:@param = body` before the
/// query's base table. When `name` is used on the left-hand side of a comparison against a value,
/// the value is bound to `param` and `body` is expanded in its place.
#[derive(Debug, Clone, PartialEq)]
pub struct CustomComparisonDef {
    pub table: String,
    pub name: String,
    /// The operator used in the definition (between the name and the parameter). Whether a call may
    /// use a different operator depends on this and on the operators used within `body`.
    pub operator: Operator,
    /// The parameter name, without the `@` sigil.
    pub param: String,
    pub body: Expr,
}

#[derive(Debug, PartialEq, Default)]
pub struct Transformation {
    pub conditions: ConditionSet,
    pub sorting: Vec<SortExpr>,
    pub result_columns: Vec<ResultColumnStatement>,
}

/// A standalone sorting expression, written with the `\\` prefix outside of the result columns,
/// e.g. `\\created_at \d`. The order in which these are listed defines their sort precedence.
#[derive(Debug, Clone, PartialEq)]
pub struct SortExpr {
    pub expr: Expr,
    pub direction: SortDirection,
    pub nulls_sort: NullsSort,
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
    Case(Case),
    Call(Call),
    Product(Box<Expr>, Box<Expr>),
    Quotient(Box<Expr>, Box<Expr>),
    Sum(Box<Expr>, Box<Expr>),
    Difference(Box<Expr>, Box<Expr>),
    Comparison(Box<Comparison>),
    /// Boolean negation of an expression, written with a `!` prefix, e.g. `!foo:2` or `!is_deleted`.
    /// This binds more loosely than comparison, so `!foo:2` negates the whole comparison.
    Not(Box<Expr>),
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
    /// Exact equality (`:=`).
    Eq,
    Gt,
    Gte,
    Lt,
    Lte,
    Like,
    /// Regular-expression match (`:~`).
    RegexMatch,
    /// The general-purpose "match" operator (`:`). Behaves like [`Operator::Eq`] except that it
    /// applies type-aware matching (e.g. case-insensitive "contains" for text values).
    Match,
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

/// A case expression, e.g. `? a:1 ~ "one" a:2 ~ "two" ~~ "other"`. Each [`CaseVariant`] pairs a
/// condition with a value; the `fallback` supplies the value used when no condition matches. This
/// compiles to a SQL searched `CASE` expression.
#[derive(Debug, Clone, PartialEq)]
pub struct Case {
    pub variants: Vec<CaseVariant>,
    pub fallback: Box<Expr>,
}

/// One `condition ~ value` arm of a [`Case`] expression.
#[derive(Debug, Clone, PartialEq)]
pub struct CaseVariant {
    pub condition: Expr,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Call {
    pub name: String,
    pub dimension: FunctionDimension,
    pub args: Vec<Expr>,
    pub syntax: CallSyntax,
    pub order_by: Vec<SortExpr>,
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
    /// Optional column-level annotation, written last in the spec as `@{ ... }`. When present, this
    /// is always a [`AnnotationValue::Object`].
    pub annotation: Option<AnnotationValue>,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(entries: Vec<(&str, AnnotationValue)>) -> AnnotationValue {
        AnnotationValue::Object(
            entries
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        )
    }

    #[test]
    fn test_meta_value_serialization() {
        // Numbers are unquoted, booleans are real JSON booleans, null is null, and object key order
        // is preserved.
        let value = AnnotationValue::Array(vec![
            obj(vec![("width", AnnotationValue::Number("100".to_string()))]),
            obj(vec![
                (
                    "formatter",
                    AnnotationValue::String("timeElapsed".to_string()),
                ),
                ("textColor", AnnotationValue::String("light".to_string())),
            ]),
            obj(vec![
                ("format", AnnotationValue::String("YYYY-MM-DD".to_string())),
                ("datePicker", AnnotationValue::Bool(true)),
            ]),
            obj(vec![("missing", AnnotationValue::Null)]),
        ]);
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(
            json,
            r#"[{"width":100},{"formatter":"timeElapsed","textColor":"light"},{"format":"YYYY-MM-DD","datePicker":true},{"missing":null}]"#
        );
    }

    #[test]
    fn test_meta_value_float_serialization() {
        let value = AnnotationValue::Number("-2.5".to_string());
        assert_eq!(serde_json::to_string(&value).unwrap(), "-2.5");
    }
}
