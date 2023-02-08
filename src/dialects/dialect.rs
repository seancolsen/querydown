use crate::syntax_tree::{Date, Duration};

pub trait Dialect {
    /// Quote a table or column for use in SQL.
    fn quote_identifier(&self, ident: &str) -> String;

    /// Quote a string for use in SQL.
    fn quote_string(&self, string: &str) -> String;

    /// Render a date literal
    fn date(&self, date: &Date) -> String;

    /// Render a duration literal
    fn duration(&self, duration: &Duration) -> String;
}
