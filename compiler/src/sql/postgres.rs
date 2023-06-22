use querydown_parser::ast::{Date, Duration};

use super::{
    dialect::{Dialect, RegExFlags},
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
        format!("INTERVAL '{}'", duration.to_iso())
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
