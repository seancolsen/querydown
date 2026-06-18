use querydown_parser::ast::{Date, Duration};

use super::{
    dialect::{duration_part, Dialect, NormalizedDuration, RegExFlags},
    expr::{
        build::{cond, sql_func},
        SqlExpr,
    },
    Postgres,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DuckDB();

impl Dialect for DuckDB {
    // For now, DuckDB mirrors Postgres for everything except the methods we've specifically
    // targeted for dialect differences. We delegate the shared behavior to Postgres to avoid
    // duplicating its logic. As we identify more dialect differences, these delegations can be
    // replaced with DuckDB-specific implementations.

    fn quote_identifier(&self, ident: &str) -> String {
        // Unlike Postgres, DuckDB does not support backslash escapes within quoted identifiers. It
        // uses standard SQL escaping, where an embedded double-quote is doubled and a backslash is
        // a literal character.
        format!(r#""{}""#, ident.replace('"', r#""""#))
    }

    fn quote_string(&self, string: &str) -> String {
        // Unlike Postgres, DuckDB does not support backslash escapes within string literals. It
        // uses standard SQL escaping, where an embedded single-quote is doubled and a backslash is
        // a literal character.
        format!("'{}'", string.replace('\'', "''"))
    }

    fn date(&self, date: &Date) -> String {
        Postgres().date(date)
    }

    fn duration(&self, duration: &Duration) -> String {
        // DuckDB has no equivalent to Postgres's `make_interval` function. Instead it provides a
        // family of `to_*` functions, each producing an interval for a single unit, which we add
        // together. We omit parts that are zero, falling back to `to_seconds(0)` when the entire
        // duration is zero.
        let d = NormalizedDuration::new(duration);

        let func = |name: &'static str| move |value: String| format!("{name}({value})");

        #[rustfmt::skip]
        let parts = [
            duration_part(d.years,   func("to_years")),
            duration_part(d.months,  func("to_months")),
            duration_part(d.weeks,   func("to_weeks")),
            duration_part(d.days,    func("to_days")),
            duration_part(d.hours,   func("to_hours")),
            duration_part(d.minutes, func("to_minutes")),
            duration_part(d.seconds, func("to_seconds")),
        ].into_iter().flatten().collect::<Vec<String>>();

        match parts.len() {
            0 => "to_seconds(0)".to_string(),
            1 => parts.into_iter().next().unwrap(),
            // Parenthesize a multi-part sum so that the duration behaves as a single atomic value
            // in surrounding expressions. Without this, `NOW() - to_years(1) + to_months(1)` would
            // wrongly parse as `(NOW() - to_years(1)) + to_months(1)`.
            _ => format!("({})", parts.join(" + ")),
        }
    }

    fn match_regex(
        &self,
        a: SqlExpr,
        b: SqlExpr,
        is_positive: bool,
        flags: &RegExFlags,
    ) -> SqlExpr {
        // DuckDB doesn't support Postgres's `~*` family of regex operators. Instead it provides
        // the `regexp_matches` function, with an optional `'i'` argument for case-insensitivity.
        let positive = if flags.is_case_sensitive {
            sql_func("regexp_matches", [a, b])
        } else {
            sql_func("regexp_matches", [a, b, SqlExpr::atom("'i'".to_string())])
        };
        if is_positive {
            positive
        } else {
            cond::not(positive)
        }
    }

    fn text_contains(&self, haystack: SqlExpr, needle: SqlExpr) -> SqlExpr {
        // `contains` returns a boolean for substring presence. `lower(strip_accents(...))` makes the
        // comparison case- and accent-insensitive. `contains` returns NULL if either argument is
        // NULL, so we coalesce to FALSE.
        SqlExpr::atom(format!(
            "COALESCE(contains(lower(strip_accents({haystack})), lower(strip_accents({needle}))), FALSE)",
            haystack = haystack.content,
            needle = needle.content,
        ))
    }
}
