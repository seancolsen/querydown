use std::fmt::Display;

use querydown_parser::ast::{Date, Duration, RegexFlags};

use super::expr::{build, SqlExpr, SqlExprPrecedence};

/// An operand whose value may be known when the query is compiled: a literal, kept apart from
/// other expressions so that a dialect can write SQL that suits the value (or merge it with
/// something else at compile time), or any other expression, already compiled.
#[derive(Debug, Clone)]
pub enum MaybeLiteral<T> {
    Literal(T),
    Expr(SqlExpr),
}

/// The pattern of a regular-expression match. A literal pattern is kept apart so that the flags a
/// dialect writes into the pattern itself (e.g. `(?m)`) can be merged into the literal at compile
/// time rather than concatenated at run time.
pub type RegexPattern = MaybeLiteral<String>;

/// The position of an array's element, counting from 0, or from the end when negative.
pub type ArrayIndex = MaybeLiteral<i64>;

/// The separator that `split` splits text at, or that `join` joins elements with.
pub type Separator = MaybeLiteral<String>;

/// A step of the path that `json_path` follows into JSON: an object's key, or an array's position
/// (counting from 0, or from the end when negative).
#[derive(Debug, Clone)]
pub enum JsonStep {
    Key(MaybeLiteral<String>),
    Position(MaybeLiteral<i64>),
}

impl MaybeLiteral<i64> {
    /// The SQL of the number.
    pub fn into_sql(self) -> SqlExpr {
        match self {
            MaybeLiteral::Literal(n) => SqlExpr::atom(n.to_string()),
            MaybeLiteral::Expr(e) => e,
        }
    }
}

/// The names that the SQL of a function given to `map`, `filter`, `some`, or `every` gives to the
/// array, to an element of it, and to the element's position. Each is unique within the query, so that a function
/// given within another one's body can't capture the other's names.
#[derive(Debug, Clone)]
pub struct ElementNames {
    /// A name for the array itself, for a dialect that refers to it more than once.
    pub array: String,
    pub element: String,
    pub position: String,
    /// Whether the function's body uses the position. When it doesn't, a dialect may leave it out.
    pub uses_position: bool,
}

/// The names that the SQL of a function given to `reduce` gives to the array, to the value
/// accumulated so far, to the next element, and to the next element's position. Each is unique within the query (see
/// [`ElementNames`]).
#[derive(Debug, Clone)]
pub struct ReduceNames {
    /// A name for the array itself, for a dialect that refers to it more than once.
    pub array: String,
    pub acc: String,
    pub element: String,
    pub position: String,
}

const SECONDS_PER_MINUTE: i64 = 60;
const SECONDS_PER_HOUR: i64 = 60 * SECONDS_PER_MINUTE;
const SECONDS_PER_DAY: i64 = 24 * SECONDS_PER_HOUR;
const SECONDS_PER_WEEK: i64 = 7 * SECONDS_PER_DAY;
/// We treat a month as 30 days. This matches Postgres, which can be verified with:
///
/// ```sql
/// select extract(epoch from make_interval(months => 1)) / (60 * 60 * 24);
/// ```
const SECONDS_PER_MONTH: i64 = 30 * SECONDS_PER_DAY;
/// We treat a year as 365.25 days. This matches Postgres, which can be verified with:
///
/// ```sql
/// select extract(epoch from make_interval(years => 1)) / (60 * 60 * 24);
/// ```
const SECONDS_PER_YEAR: i64 = 365 * SECONDS_PER_DAY + 6 * SECONDS_PER_HOUR;

pub trait Zero {
    const ZERO: Self;
}

impl Zero for i64 {
    const ZERO: Self = 0;
}
impl Zero for f64 {
    const ZERO: Self = 0.0;
}

/// A [Duration] with each of its (potentially fractional) parts normalized into integer values,
/// with any fractional remainders rolled down into `seconds`.
///
/// Querydown lets users define durations in terms of floats, but the duration-construction
/// functions in most SQL dialects only accept integer values for parts larger than seconds. So we
/// convert the float values into integers, accumulating the fractional remainders into `seconds`.
/// This logic is dialect-agnostic; only the rendering of these parts into SQL differs per dialect.
pub struct NormalizedDuration {
    pub years: i64,
    pub months: i64,
    pub weeks: i64,
    pub days: i64,
    pub hours: i64,
    pub minutes: i64,
    pub seconds: f64,
}

impl NormalizedDuration {
    pub fn new(duration: &Duration) -> Self {
        let mut seconds = duration.seconds;

        let mut convert = |v: f64, multiplier: i64| -> i64 {
            seconds += v.fract() * multiplier as f64;
            v.floor() as i64
        };

        // We must perform all the conversions before reading the final `seconds` value because
        // each conversion may add to it.
        let years = convert(duration.years, SECONDS_PER_YEAR);
        let months = convert(duration.months, SECONDS_PER_MONTH);
        let weeks = convert(duration.weeks, SECONDS_PER_WEEK);
        let days = convert(duration.days, SECONDS_PER_DAY);
        let hours = convert(duration.hours, SECONDS_PER_HOUR);
        let minutes = convert(duration.minutes, SECONDS_PER_MINUTE);

        Self {
            years,
            months,
            weeks,
            days,
            hours,
            minutes,
            seconds,
        }
    }
}

/// Render a single part of a duration via a helper, omitting parts that are zero.
///
/// * `value` - The numeric value of the part
/// * `render` - A closure that renders the part into SQL, given its value formatted as a string
pub fn duration_part<T>(value: T, render: impl FnOnce(String) -> String) -> Option<String>
where
    T: PartialEq + Zero + Display,
{
    if value == T::ZERO {
        None
    } else {
        Some(render(value.to_string()))
    }
}

pub trait Dialect {
    /// The static SQL query that introspects a live database of this dialect, producing the schema
    /// JSON the compiler consumes. It yields a single row with a single column containing the JSON.
    /// Consumers run this query themselves and feed the result back into [`Compiler::new`].
    ///
    /// [`Compiler::new`]: crate::Compiler::new
    fn introspection_sql(&self) -> &'static str;

    /// Quote a table or column for use in SQL.
    fn quote_identifier(&self, ident: &str) -> String;

    /// Quote a string for use in SQL.
    fn quote_string(&self, string: &str) -> String;

    /// Render a date literal
    fn date(&self, date: &Date) -> String;

    /// Render a duration literal
    fn duration(&self, duration: &Duration) -> String;

    /// Render a table and column reference
    fn table_column(&self, table: &str, column: &str) -> String {
        let quoted_table = self.quote_identifier(table);
        let quoted_column = self.quote_identifier(column);
        format!("{}.{}", quoted_table, quoted_column)
    }

    /// The inline options (e.g. `(?m)`) that, written at the start of a pattern, give the
    /// dialect's regex engine the newline handling that `flags` ask for. Every regex function
    /// writes them, so that the flags mean the same everywhere. (Case-sensitivity is set apart, by
    /// each function's own means.)
    fn regex_newline_options(&self, flags: &RegexFlags) -> &'static str;

    /// Render a regular expression comparison between two values
    ///
    /// * `a` - The left-hand side of the comparison
    /// * `pattern` - The right-hand side of the comparison
    /// * `flags` - Flags to control the behavior of the regular expression. Every dialect must
    ///   give them the same meaning, which follows Python's inline flags; see [`RegexFlags`].
    fn match_regex(&self, a: SqlExpr, pattern: RegexPattern, flags: &RegexFlags) -> SqlExpr;

    /// Render `regex_replace`: `a` with every match of `pattern` replaced by `replacement`, in
    /// which `\1` stands for the first capture group. `flags` mean what they do for
    /// [`Dialect::match_regex`].
    fn regex_replace(
        &self,
        a: SqlExpr,
        pattern: RegexPattern,
        replacement: SqlExpr,
        flags: &RegexFlags,
    ) -> SqlExpr;

    /// Render `regex_extract`: the first match of `pattern` in `a`, or with `group`, the text of
    /// that capture group (where 0 is the whole match). NULL when there is no match. `flags` mean
    /// what they do for [`Dialect::match_regex`].
    fn regex_extract(
        &self,
        a: SqlExpr,
        pattern: RegexPattern,
        group: u32,
        flags: &RegexFlags,
    ) -> SqlExpr;

    /// Render `round`: `a` rounded to `digits` decimal places (or to an integer, without them),
    /// with a half rounded away from zero. A negative `digits` rounds to tens, hundreds, and so on.
    /// NaN and infinity stay as they are. `digits` is an integer, as an expression too.
    fn round(&self, a: SqlExpr, digits: Option<MaybeLiteral<i64>>) -> SqlExpr;

    /// Render `is_nan`: whether the number `a` is the floating-point value NaN.
    fn is_nan(&self, a: SqlExpr) -> SqlExpr;

    /// Render `seconds_to_duration`: a duration (an interval) of `a` seconds.
    fn seconds_to_duration(&self, a: SqlExpr) -> SqlExpr;

    /// Render an array of the given values, of which there is at least one.
    fn array(&self, values: Vec<SqlExpr>) -> SqlExpr;

    /// Render `at`: the element of the array `a` at `index`, counting from 0, or from the end when
    /// it's negative (so -1 is the last element). NULL when there is no such element.
    fn array_at(&self, a: SqlExpr, index: ArrayIndex) -> SqlExpr;

    /// Render the number of elements of the array `a`.
    fn array_length(&self, a: SqlExpr) -> SqlExpr;

    /// Render `split`: the parts of the text `a` between occurrences of `separator`, as an array.
    /// Empty text has no parts. An empty separator splits the text into its characters, and a
    /// NULL one gives NULL.
    fn split(&self, a: SqlExpr, separator: Separator) -> SqlExpr;

    /// Render `join`: the elements of the array `a` that aren't NULL, as text, joined by
    /// `separator`. An array without such elements gives the empty string, and a NULL separator
    /// gives NULL.
    fn join(&self, a: SqlExpr, separator: Separator) -> SqlExpr;

    /// The names that SQL which this dialect writes by itself gives to the variables of lambdas,
    /// such as those of `join`. The names of the functions given to `map` (and the like) avoid
    /// them, so that they can't be captured.
    fn reserved_lambda_names(&self) -> &'static [&'static str] {
        &[]
    }

    /// Whether the body of a function given to `map` (and the like) may contain a subquery.
    fn allows_subquery_in_function_arg(&self) -> bool {
        true
    }

    /// The SQL that stands, within the body of a function given to `map`, `filter`, `some`, or
    /// `every`, for an element of the array, and for its position, counting from 0. Both
    /// dialects count positions from 1, so 1 is subtracted.
    fn element_params(&self, names: &ElementNames) -> (SqlExpr, SqlExpr) {
        let element = SqlExpr::atom(self.quote_identifier(&names.element));
        let position = SqlExpr::atom(self.quote_identifier(&names.position));
        (
            element,
            build::math::subtract(position, build::value::one()),
        )
    }

    /// Render `map`: an array of `body`'s value for each element of the array `a`, in order. NULL
    /// for a NULL array. `body` refers to the element and its position by `names` (see
    /// [`Dialect::element_params`]).
    fn array_map(&self, a: SqlExpr, names: &ElementNames, body: SqlExpr) -> SqlExpr;

    /// Render `filter`: the elements of the array `a` for which `body` is true, in order. NULL for
    /// a NULL array.
    fn array_filter(&self, a: SqlExpr, names: &ElementNames, body: SqlExpr) -> SqlExpr;

    /// Render `some`: whether `body` is true for at least one element of the array `a` (false for
    /// an empty array, and NULL for a NULL one). A NULL `body` counts as false.
    fn array_some(&self, a: SqlExpr, names: &ElementNames, body: SqlExpr) -> SqlExpr;

    /// Render `every`: whether `body` is true for every element of the array `a` (true for an
    /// empty array, and NULL for a NULL one). A NULL `body` counts as false.
    fn array_every(&self, a: SqlExpr, names: &ElementNames, body: SqlExpr) -> SqlExpr;

    /// The SQL that stands, within the body of a function given to `reduce` over the array `a`, for
    /// the value accumulated so far, and for the next element.
    fn reduce_params(&self, a: &SqlExpr, names: &ReduceNames) -> (SqlExpr, SqlExpr);

    /// Render `reduce`: the elements of the array `a` combined from the left by `body`, starting
    /// from the first element (see [`Dialect::reduce_params`]). NULL for an empty or NULL array.
    fn array_reduce(&self, a: SqlExpr, names: &ReduceNames, body: SqlExpr) -> SqlExpr;

    /// Render `json_path`: the value within the JSON `a` that `steps` (of which there's at least
    /// one) lead to, as text. A string is unquoted, and an object or array is JSON text. NULL when
    /// the path leads nowhere, or to a JSON null. `is_json` says whether `a` is known to be JSON
    /// already, rather than text that holds it.
    fn json_path(&self, a: SqlExpr, is_json: bool, steps: Vec<JsonStep>) -> SqlExpr;

    /// Render `to_json`: the text `a` read as JSON.
    fn to_json(&self, a: SqlExpr) -> SqlExpr;

    /// Render `to_array`: the elements of the JSON array `a` (JSON, or text that holds it), as an
    /// array of text, as `json_path` gives them. NULL for anything that isn't an array. `name` is
    /// unique within the query, for a dialect that names `a` in a subquery.
    fn to_array(&self, a: SqlExpr, name: &str) -> SqlExpr;

    /// Render `json_object`: a JSON object of the given keys (text) and values.
    fn json_object(&self, pairs: Vec<(SqlExpr, SqlExpr)>) -> SqlExpr;

    /// Render `json_array`: a JSON array of the given values.
    fn json_array(&self, values: Vec<SqlExpr>) -> SqlExpr;

    /// Render a value given to `json_object` or `json_array`, so that the dialect writes it into
    /// JSON as the other does. `is_timestamp` says whether it's known to be a timestamp, which
    /// becomes an ISO 8601 string.
    fn json_value(&self, value: SqlExpr, _is_timestamp: bool) -> SqlExpr {
        value
    }

    /// Render a regex `pattern` with the inline options `prefix` (e.g. `(?m)`) written before it.
    /// A literal pattern absorbs the prefix; any other is concatenated with it at run time, which
    /// keeps a NULL pattern NULL.
    fn prefixed_regex_pattern(&self, prefix: &str, pattern: RegexPattern) -> SqlExpr {
        match pattern {
            RegexPattern::Literal(s) => SqlExpr::atom(self.quote_string(&format!("{prefix}{s}"))),
            RegexPattern::Expr(e) if prefix.is_empty() => e,
            RegexPattern::Expr(e) => SqlExpr::atom(format!(
                "({} || {})",
                self.quote_string(prefix),
                e.for_precedence(SqlExprPrecedence::Atom)
            )),
        }
    }

    /// Render a case-insensitive "contains" text comparison: true when `haystack` contains `needle`
    /// as a substring. A NULL `haystack` or `needle` yields FALSE (not NULL), and the result is
    /// marked [`SqlExpr::never_null`] to say so.
    ///
    /// * `haystack` - The text being searched (the left-hand side of the `:` comparison)
    /// * `needle` - The text being searched for (the right-hand side of the `:` comparison)
    fn text_contains(&self, haystack: SqlExpr, needle: SqlExpr) -> SqlExpr;

    /// Render a case-insensitive text equality comparison (the `:=` operator applied to text). This
    /// is the full-string counterpart to [`Dialect::text_contains`], applying the same per-dialect
    /// normalization. Standard SQL NULL semantics apply (a NULL operand yields NULL, not FALSE),
    /// matching a plain `=` comparison.
    fn text_eq(&self, a: SqlExpr, b: SqlExpr) -> SqlExpr;

    /// Render an aggregate that multiplies the values of `arg` together. DuckDB has a native
    /// `product` aggregate, but Postgres does not, so the dialects diverge here.
    fn aggregate_product(&self, arg: SqlExpr) -> SqlExpr;

    /// Render a deterministic hash of `arg`, normalized to a `double` uniformly distributed over
    /// `[0, 1]`. Both dialects hash to a fixed-width integer and divide by that integer type's
    /// maximum value, but the hash function and integer width differ: DuckDB's `hash` produces an
    /// unsigned 64-bit value, while Postgres's `hashtext` produces a signed 32-bit value.
    fn unit_hash(&self, arg: SqlExpr) -> SqlExpr;

    /// Format a date or timestamp as a `YYYY-MM` string (e.g. `2023-03`). The dialects have no
    /// common date-formatting function: Postgres uses `to_char`, DuckDB uses `strftime`.
    fn year_month(&self, arg: SqlExpr) -> SqlExpr;

    /// Coerce a zoned timestamp expression to a naive (zone-less) one, for use when a binary
    /// operation mixes a zoned and a naive temporal operand. The default is the identity: most
    /// databases (including Postgres) reconcile the two sides natively, so no coercion is needed.
    /// DuckDB without the ICU extension overrides this, because it cannot mix the two types. See
    /// [`crate::compiler::temporal`] for when this is invoked.
    fn coerce_temporal_to_naive(&self, e: SqlExpr) -> SqlExpr {
        e
    }

    /// Whether this dialect cannot operate on zoned timestamps (`timestamptz`) at all — not even
    /// `timestamptz - interval` or a `timestamptz` vs `timestamptz` mix with a naive value. DuckDB
    /// built without the ICU extension is such a dialect. When true, the compiler works entirely in
    /// naive timestamp space: `@now` is coerced to naive at its source, and any zoned column that
    /// meets a naive value is coerced too (see [`crate::compiler::temporal`]).
    fn coerces_timestamptz(&self) -> bool {
        false
    }
}
