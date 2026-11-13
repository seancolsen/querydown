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

    /// Prepares the expression to be an operand of an operator with the given precedence, by
    /// parenthesizing it if it binds more loosely than that operator.
    ///
    /// This suits the *left* operand of a left-associative operator (and any operand of an
    /// associative one like `AND`), where an operand of the same precedence groups the way it's
    /// written: `(a - b) - c` is `a - b - c`. For the other operands, see
    /// [`SqlExpr::for_tighter_precedence`].
    pub fn for_precedence(mut self, precedence: SqlExprPrecedence) -> SqlExpr {
        if precedence > self.precedence {
            self.parenthesize();
        }
        self
    }

    /// Like [`SqlExpr::for_precedence`], but also parenthesizes an expression that binds exactly
    /// as tightly as the operator.
    ///
    /// This suits the *right* operand of a left-associative operator, where an operand of the same
    /// precedence would otherwise regroup: `a - (b + c)` is not `a - b + c`. It also suits both
    /// operands of an operator that can't be chained at all, like `=`.
    pub fn for_tighter_precedence(mut self, precedence: SqlExprPrecedence) -> SqlExpr {
        if precedence >= self.precedence {
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
