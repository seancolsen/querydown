use std::fmt::Display;

use querydown_parser::ast::{Date, Duration};

use super::{
    dialect::{Dialect, RegExFlags},
    expr::{build::cmp::comparison, SqlExpr},
};

const SECONDS_PER_MINUTE: i64 = 60;
const SECONDS_PER_HOUR: i64 = 60 * SECONDS_PER_MINUTE;
const SECONDS_PER_DAY: i64 = 24 * SECONDS_PER_HOUR;
const SECONDS_PER_WEEK: i64 = 7 * SECONDS_PER_DAY;
/// Postgres treats a month as 30 days. For proof, try:
///
/// ```sql
/// select extract(epoch from make_interval(months => 1)) / (60 * 60 * 24);
/// ```
const SECONDS_PER_MONTH: i64 = 30 * SECONDS_PER_DAY;
/// Postgres treats a year as 365.25 days. For proof, try:
///
/// ```sql
/// select extract(epoch from make_interval(years => 1)) / (60 * 60 * 24);
/// ```
const SECONDS_PER_YEAR: i64 = 365 * SECONDS_PER_DAY + 6 * SECONDS_PER_HOUR;

trait Zero {
    const ZERO: Self;
}

impl Zero for i64 {
    const ZERO: Self = 0;
}
impl Zero for f64 {
    const ZERO: Self = 0.0;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Postgres();

// TODO: we need to make sure other escape sequences which find their way into the string value
// stored in the AST are not unintentionally processed as escape sequences by Postgres. See
// https://www.postgresql.org/docs/current/sql-syntax-lexical.html for continued research.
impl Dialect for Postgres {
    fn quote_identifier(&self, ident: &str) -> String {
        format!(r#""{}""#, ident.replace(r"\", r"\\").replace('"', r#"\""#))
    }

    fn quote_string(&self, string: &str) -> String {
        format!("'{}'", string.replace(r"\", r"\\").replace("'", r"\'"))
    }

    fn date(&self, date: &Date) -> String {
        format!("DATE '{}'", date.to_iso())
    }

    fn duration(&self, duration: &Duration) -> String {
        // The complexity in this function is due to the following requirements:
        //
        // - In Querydown code, we'd like to support defining durations in terms of floats, but
        // Postgres only supports integer values for most of the arguments to the [make_interval]
        // function. So we have to convert the float values into integers.
        //
        // - We'd like to render SQL that doesn't show zero values for duration parts.
        //
        // [make_interval]: https://www.postgresql.org/docs/current/functions-datetime.html

        let mut seconds = duration.seconds;

        let mut convert = |v: f64, multiplier: i64| -> i64 {
            seconds += v.fract() * multiplier as f64;
            v.floor() as i64
        };

        fn part<T>(value: T, name: &str) -> Option<String>
        where
            T: PartialEq + Zero + Display,
        {
            if value == T::ZERO {
                None
            } else {
                Some(format!("{name} => {value}"))
            }
        }

        #[rustfmt::skip]
        let args = [
            part(convert(duration.years,   SECONDS_PER_YEAR  ), "years"),
            part(convert(duration.months,  SECONDS_PER_MONTH ), "months"),
            part(convert(duration.weeks,   SECONDS_PER_WEEK  ), "weeks"),
            part(convert(duration.days,    SECONDS_PER_DAY   ), "days"),
            part(convert(duration.hours,   SECONDS_PER_HOUR  ), "hours"),
            part(convert(duration.minutes, SECONDS_PER_MINUTE), "mins"),
            part(seconds, "secs"),
        ].into_iter().filter_map(|v| v).collect::<Vec<String>>().join(", ");

        format!("make_interval({args})")
    }

    fn match_regex(
        &self,
        a: SqlExpr,
        b: SqlExpr,
        is_positive: bool,
        flags: &RegExFlags,
    ) -> SqlExpr {
        let op = match (is_positive, flags.is_case_sensitive) {
            (true, true) => "~",
            (true, false) => "~*",
            (false, true) => "!~",
            (false, false) => "!~*",
        };
        comparison(a, op, b)
    }
}
