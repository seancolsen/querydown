pub mod build;

use std::fmt::{Display, Formatter};

#[derive(Debug, Default, Clone)]
pub struct SqlExpr {
    pub content: String,
    pub precedence: SqlExprPrecedence,
    /// Whether the expression is known never to evaluate to NULL (e.g. an `IS NULL` test). Only a
    /// builder that can vouch for it sets this; everywhere else it stays `false`, which is always
    /// safe. Negation reads it to emit a plain `NOT` where SQL's own `NOT` already does the job.
    pub never_null: bool,
}

impl SqlExpr {
    pub fn empty() -> SqlExpr {
        SqlExpr {
            content: String::new(),
            precedence: SqlExprPrecedence::Atom,
            never_null: false,
        }
    }
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    pub fn is_null(&self) -> bool {
        self.content == "NULL"
    }

    pub fn atom(content: String) -> SqlExpr {
        SqlExpr {
            content,
            precedence: SqlExprPrecedence::Atom,
            never_null: false,
        }
    }

    /// Marks the expression as one that never evaluates to NULL. See [`SqlExpr::never_null`].
    pub fn never_null(mut self) -> SqlExpr {
        self.never_null = true;
        self
    }

    fn parenthesize(&mut self) {
        self.content = format!("({})", self.content);
        self.precedence = SqlExprPrecedence::Atom;
    }

    pub fn for_precedence(mut self, precedence: SqlExprPrecedence) -> SqlExpr {
        if precedence > self.precedence {
            self.parenthesize();
        }
        self
    }
}

impl Display for SqlExpr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.content)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
/// https://www.postgresql.org/docs/current/sql-syntax-lexical.html#SQL-PRECEDENCE
#[derive(Default)]
pub enum SqlExprPrecedence {
    /// A literal value, a column name, a function call, or parentheses.
    #[default]
    Atom = 0,
    /// `*` `/` `%`
    Multiplication = -1,
    /// `+` `-`
    Addition = -2,
    /// `=` `<>` `>` `>=` `<` `<=` `IS` `IS NOT` `IN` `LIKE` `NOT LIKE
    Comparison = -3,
    /// `NOT`
    LogicalNot = -4,
    /// `AND`
    LogicalAnd = -5,
    /// `OR`
    LogicalOr = -6,
}
