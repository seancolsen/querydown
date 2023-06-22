use querydown_parser::ast::{Date, Duration};

use super::expr::SqlExpr;

pub struct RegExFlags {
    pub is_case_sensitive: bool,
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
}
