use std::fmt::{Display, Formatter};

#[derive(Debug, Default, Clone)]
pub struct SqlExpr {
    pub content: String,
    pub precedence: SqlExprPrecedence,
}

impl SqlExpr {
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
pub enum SqlExprPrecedence {
    /// A literal value, a column name, a function call, or parentheses.
    Atom = 0,
    /// `*` `/` `%`
    Multiplication = -1,
    /// `+` `-`
    Addition = -2,
    /// `=` `<>` `>` `>=` `<` `<=` `IS` `IS NOT` `IN` `LIKE` `NOT LIKE
    Comparison = -3,
    /// `AND`
    LogicalAnd = -4,
    /// `OR`
    LogicalOr = -5,
}

impl Default for SqlExprPrecedence {
    fn default() -> Self {
        Self::Atom
    }
}
