use querydown_parser::ast::{Date, Duration};

use super::{
    dialect::{duration_part, Dialect, NormalizedDuration, RegExFlags},
    expr::{build::cmp::comparison, SqlExpr},
};

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
