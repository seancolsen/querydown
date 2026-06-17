use querydown_parser::ast::{Date, Duration};

use super::{
    dialect::{duration_part, Dialect, NormalizedDuration, RegExFlags},
    expr::{build::cmp::comparison, SqlExpr},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Postgres();

// TODO: `quote_identifier` still backslash-escapes embedded quotes/backslashes, whereas standard
// Postgres escapes a `"` inside a quoted identifier by doubling it. No current input exercises an
// identifier containing those characters, so this is left for later.
impl Dialect for Postgres {
    fn quote_identifier(&self, ident: &str) -> String {
        format!(r#""{}""#, ident.replace(r"\", r"\\").replace('"', r#"\""#))
    }

    fn quote_string(&self, string: &str) -> String {
        // Postgres uses standard-conforming string literals by default
        // (standard_conforming_strings=on), so backslashes are literal and a single-quote is
        // escaped by doubling it. Backslash-escaping a quote (`\'`) is only valid inside an
        // `E'...'` string and produces a syntax error under the default setting.
        format!("'{}'", string.replace('\'', "''"))
    }

    fn date(&self, date: &Date) -> String {
        format!("DATE '{}'", date.to_iso())
    }

    fn duration(&self, duration: &Duration) -> String {
        // We render durations using the [make_interval] function, which doesn't show zero values
        // for duration parts. When all parts are zero this produces `make_interval()`, which
        // Postgres renders as a zero interval.
        //
        // [make_interval]: https://www.postgresql.org/docs/current/functions-datetime.html

        let d = NormalizedDuration::new(duration);

        let arg = |name: &'static str| move |value: String| format!("{name} => {value}");

        #[rustfmt::skip]
        let args = [
            duration_part(d.years,   arg("years")),
            duration_part(d.months,  arg("months")),
            duration_part(d.weeks,   arg("weeks")),
            duration_part(d.days,    arg("days")),
            duration_part(d.hours,   arg("hours")),
            duration_part(d.minutes, arg("mins")),
            duration_part(d.seconds, arg("secs")),
        ].into_iter().flatten().collect::<Vec<String>>().join(", ");

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
