use std::fmt::Display;

use querydown_parser::ast::{Date, Duration};

use super::expr::SqlExpr;

pub struct RegExFlags {
    pub is_case_sensitive: bool,
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

    /// Render a regular expression comparison between two values
    ///
    /// * `a` - The left-hand side of the comparison
    /// * `b` - The right-hand side of the comparison
    /// * `is_positive` - true when you want the comparison to return true for matching values,
    ///   false when the comparison is negated.
    /// * `flags` - Flags to control the behavior of the regular expression
    fn match_regex(&self, a: SqlExpr, b: SqlExpr, is_positive: bool, flags: &RegExFlags)
        -> SqlExpr;

    /// Render a case-insensitive "contains" text comparison: true when `haystack` contains `needle`
    /// as a substring. A NULL `haystack` or `needle` yields FALSE (not NULL).
    ///
    /// * `haystack` - The text being searched (the left-hand side of the `:` comparison)
    /// * `needle` - The text being searched for (the right-hand side of the `:` comparison)
    fn text_contains(&self, haystack: SqlExpr, needle: SqlExpr) -> SqlExpr;

    /// Render an aggregate that multiplies the values of `arg` together. DuckDB has a native
    /// `product` aggregate, but Postgres does not, so the dialects diverge here.
    fn aggregate_product(&self, arg: SqlExpr) -> SqlExpr;
}
