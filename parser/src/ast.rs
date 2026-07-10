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

#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    /// The definitions (constants, functions, custom comparisons, computed columns, and user-defined
    /// tables) written before the base table.
    pub definitions: Definitions,
    pub base_table: String,
    pub transformations: Vec<Transformation>,
}

impl Query {
    /// Assembles a [`Query`] from independently-parsed sections plus a base table.
    ///
    /// This is the reassembly counterpart to parsing each section of a query in isolation (see the
    /// `parse_definitions`, `parse_conditions`, `parse_sorting`, and `parse_display` functions in
    /// the crate root). It bundles the `conditions`, `sorting`, and `display` sections into a single
    /// [`Transformation`], which — together with the `definitions` and `base_table` — forms a
    /// complete query ready to be handed to the compiler.
    ///
    /// The base table is supplied separately because it is not part of any of the four sections; in
    /// a multi-input UI it is typically chosen on its own (e.g. via a dropdown) rather than typed.
    pub fn from_parts(
        base_table: String,
        definitions: Definitions,
        conditions: ConditionSet,
        sorting: Vec<SortExpr>,
        display: Vec<ResultColumnStatement>,
    ) -> Self {
        Query {
            definitions,
            base_table,
            transformations: vec![Transformation {
                conditions,
                sorting,
                result_columns: display,
            }],
        }
    }
}

/// The definitions that may precede a query's base table. Each kind shares the same position in the
/// source (before the base table) and may be parsed on its own via the crate-root
/// `parse_definitions` function.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Definitions {
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
    /// User-defined table definitions, written as `#name = #( query )` before the base table. Each
    /// names a subquery that can then be used as the base table of the query (or of a later
    /// user-defined table). Compiles to a CTE.
    pub tables: Vec<TableDef>,
}

/// A user-defined table definition, written as `#name = #( query )` before the query's base table.
/// The named subquery is compiled to a CTE that can be used as a base table by name.
#[derive(Debug, Clone, PartialEq)]
pub struct TableDef {
    pub name: String,
    pub query: Query,
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

#[derive(Debug, Clone, PartialEq, Default)]
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
    /// A condition set scoped to a single related record, written as a path immediately followed —
    /// with no space — by a condition set, e.g. `issue{title:dashboard}`. See [`ScopedConditionSet`].
    ScopedConditionSet(ScopedConditionSet),
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
    /// A window function application, written as `%%( ... )%func(args)`. The `%%( ... )` defines the
    /// window (partition and ordering), and the trailing `%func` applies a window function over it.
    Window(WindowFn),
    /// An anonymous function applied to arguments, e.g. `value|(@d => @d:<0)`. Anonymous functions
    /// can only be applied immediately, via a pipe, so this node always carries its arguments. Like
    /// a user-defined function, its parameters are bound to the arguments and its body is inlined.
    AnonymousFunctionCall(Box<AnonymousFunctionCall>),
    /// A scalar subquery, written as `#( query )`. The inner query must produce a single value, which
    /// is inlined as a parenthesized `(SELECT ...)`. This is what gives a constant defined via
    /// `@name = #( ... )` its value.
    Subquery(Box<Query>),
}

/// An anonymous function applied to arguments, written as `value|(@param => body)`. The piped-in
/// value becomes the first argument. There is no way to name or store an anonymous function, so it
/// is always applied at the point where it is written.
#[derive(Debug, Clone, PartialEq)]
pub struct AnonymousFunctionCall {
    /// Parameter names, without the `@` sigil, in order.
    pub params: Vec<String>,
    /// The function body: zero or more local assignments followed by a single result expression.
    pub body: FunctionBody,
    /// The arguments to which the function is applied. The first is the piped-in value; any others
    /// come from parenthesized arguments following the anonymous function.
    pub args: Vec<Expr>,
}

/// A window function application: a function applied over a window defined by partition and ordering
/// expressions. Written as `%%( partition\p ordering\s )%func(args)`. Compiles to a SQL window
/// function (`func(args) OVER (PARTITION BY ... ORDER BY ...)`).
#[derive(Debug, Clone, PartialEq)]
pub struct WindowFn {
    /// The window function name as written, e.g. `row_number`, `sum`, `lag`.
    pub function: String,
    /// The function's value arguments (e.g. the column for `sum`, or the column plus offset and
    /// default for `lag`). Empty for ranking functions like `row_number`.
    pub args: Vec<Expr>,
    /// The `PARTITION BY` expressions, taken from the window definition's `\p`-flagged entries (and
    /// any entries with no flag).
    pub partition_by: Vec<Expr>,
    /// The `ORDER BY` entries, taken from the window definition's `\s`-flagged entries.
    pub order_by: Vec<SortExpr>,
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

/// A condition set scoped to a single related record, written as a path immediately followed by a
/// condition set with no space between them, e.g. `issue{title:dashboard}` or
/// `issue[title:dashboard description:dashboard]`.
///
/// Every entry of the `condition_set` is evaluated as though `path` had been written in front of it,
/// so `issue{title:dashboard}` scopes `title:dashboard` to the base record's issue (the same as
/// `issue.title:dashboard`). Unlike prefixing the path syntactically, this scopes the _whole_
/// condition set — including entries that have no leading column reference to prefix, most notably a
/// bare [default text search](Expr::String) term. That is why the scoping is resolved in the
/// compiler (against the related table) rather than desugared away in the parser: `issue{dashboard}`
/// searches the issue's text columns, which has no flat-syntax equivalent.
#[derive(Debug, Clone, PartialEq)]
pub struct ScopedConditionSet {
    /// The path to the single related record the condition set is scoped to.
    pub path: Vec<PathPart>,
    pub condition_set: ConditionSet,
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

#[derive(Debug, Clone, PartialEq)]
pub enum ResultColumnStatement {
    Spec(ColumnSpec),
    Glob(ColumnGlob),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColumnSpec {
    pub expr: Expr,
    pub alias: Option<String>,
    pub column_control: ColumnControl,
    /// Optional column-level annotation, written last in the spec as `@{ ... }`. When present, this
    /// is always a [`AnnotationValue::Object`].
    pub annotation: Option<AnnotationValue>,
}

#[derive(Debug, Clone, PartialEq, Default)]
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

#[derive(Debug, Default, Clone, PartialEq)]
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
