use querydown_parser::ast::{Date, Duration};

use super::{
    dialect::{Dialect, RegExFlags},
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
        Postgres().quote_identifier(ident)
    }

    fn quote_string(&self, string: &str) -> String {
        Postgres().quote_string(string)
    }

    fn date(&self, date: &Date) -> String {
        Postgres().date(date)
    }

    fn duration(&self, duration: &Duration) -> String {
        Postgres().duration(duration)
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
}
