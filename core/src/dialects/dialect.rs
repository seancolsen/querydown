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

    /// Render a table and column reference
    fn table_column(&self, table: &str, column: &str) -> String {
        let quoted_table = self.quote_identifier(table);
        let quoted_column = self.quote_identifier(column);
        format!("{}.{}", quoted_table, quoted_column)
    }
}
