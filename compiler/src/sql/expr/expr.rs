use std::fmt::{Display, Formatter};

#[derive(Debug, Default, Clone)]
pub struct SqlExpr {
    pub content: String,
    pub precedence: SqlExprPrecedence,
}

impl SqlExpr {
    pub fn empty() -> SqlExpr {
        SqlExpr {
            content: String::new(),
            precedence: SqlExprPrecedence::Atom,
        }
    }
    pub fn is_empty(&self) -> bool {
        self.content.len() == 0
    }

    pub fn is_null(&self) -> bool {
        self.content == "NULL"
    }

    pub fn atom(content: String) -> SqlExpr {
        SqlExpr {
            content,
            precedence: SqlExprPrecedence::Atom,
        }
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
pub enum SqlExprPrecedence {
    /// A literal value, a column name, a function call, or parentheses.
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

impl Default for SqlExprPrecedence {
    fn default() -> Self {
        Self::Atom
    }
}
