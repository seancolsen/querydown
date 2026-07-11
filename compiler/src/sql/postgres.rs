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
    fn introspection_sql(&self) -> &'static str {
        include_str!("../../resources/introspection/postgres.sql")
    }

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

    fn text_contains(&self, haystack: SqlExpr, needle: SqlExpr) -> SqlExpr {
        // `strpos` returns the 1-based position of the first match, or 0 when not found, so `> 0`
        // means "contains". `lower(... COLLATE "C")` gives a case-insensitive, byte-wise
        // comparison. `strpos` returns NULL if either argument is NULL, so we coalesce to FALSE.
        SqlExpr::atom(format!(
            r#"COALESCE(strpos(lower({haystack} COLLATE "C"), lower({needle} COLLATE "C")) > 0, FALSE)"#,
            haystack = haystack.content,
            needle = needle.content,
        ))
    }

    fn aggregate_product(&self, arg: SqlExpr) -> SqlExpr {
        // Postgres has no native product aggregate, so we reconstruct it from sums of logarithms:
        // the product of the magnitudes is `exp(sum(ln(abs(x))))`, and the sign is negative only
        // when an odd number of values are negative. Zeros are excluded from the logarithm (where
        // `ln` is undefined) and instead short-circuit the whole result to 0.
        let a = arg.content;
        SqlExpr::atom(format!(
            "CASE \
             WHEN bool_or({a} = 0) THEN 0 \
             ELSE (CASE WHEN count(*) FILTER (WHERE {a} < 0) % 2 = 1 THEN -1 ELSE 1 END) \
             * round(exp(sum(ln(abs({a})::double precision)) FILTER (WHERE {a} <> 0))) \
             END"
        ))
    }

    fn unit_hash(&self, arg: SqlExpr) -> SqlExpr {
        // `hashtext` returns a signed 32-bit integer over the full `int4` range. Shifting it up by
        // 2^31 and dividing by the range's width (2^32 - 1) maps it onto a uniform double in
        // [0, 1], mirroring DuckDB's normalization of its unsigned 64-bit hash.
        SqlExpr::atom(format!(
            "(CAST(hashtext({a}) AS DOUBLE PRECISION) + 2147483648) / 4294967295.0",
            a = arg.content
        ))
    }
}
