use itertools::Itertools;
use querydown_parser::ast::Conjunction;

use super::{SqlExpr, SqlExprPrecedence};

fn binary_op(a: SqlExpr, op: &str, b: SqlExpr, precedence: SqlExprPrecedence) -> SqlExpr {
    SqlExpr {
        content: format!(
            "{} {} {}",
            a.for_precedence(precedence),
            op,
            b.for_precedence(precedence)
        ),
        precedence,
    }
}

fn sql_func(name: &str, args: impl IntoIterator<Item = SqlExpr>) -> SqlExpr {
    SqlExpr::atom(format!("{}({})", name, args.into_iter().join(", ")))
}

pub mod agg {
    use super::*;

    pub fn bool_and(a: SqlExpr) -> SqlExpr {
        sql_func("bool_and", [a])
    }

    pub fn bool_or(a: SqlExpr) -> SqlExpr {
        sql_func("bool_or", [a])
    }

    pub fn avg(a: SqlExpr) -> SqlExpr {
        sql_func("avg", [a])
    }

    pub fn count(a: SqlExpr) -> SqlExpr {
        sql_func("count", [a])
    }

    pub fn count_star() -> SqlExpr {
        SqlExpr::atom("count(*)".to_string())
    }

    pub fn count_distinct(a: SqlExpr) -> SqlExpr {
        // TODO: We should alter the query at a higher level to use an approach like this for
        // better performance:
        // https://stackoverflow.com/questions/11250253/postgresql-countdistinct-very-slow
        SqlExpr::atom(format!("count(DISTINCT {})", a.content))
    }

    pub fn max(a: SqlExpr) -> SqlExpr {
        sql_func("max", [a])
    }

    pub fn min(a: SqlExpr) -> SqlExpr {
        sql_func("min", [a])
    }

    pub fn string_agg(a: SqlExpr) -> SqlExpr {
        // TODO:
        // - Let user customize the separator
        // - Use dialect-specific string quoting logic
        let separator = SqlExpr::atom("', '".to_string());
        sql_func("string_agg", [a, separator])
    }

    pub fn sum(a: SqlExpr) -> SqlExpr {
        sql_func("sum", [a])
    }
}

pub mod cmp {
    use super::*;

    /// A set of conditions joined by `AND` or `OR`
    pub fn condition_set(
        conditions: impl IntoIterator<Item = SqlExpr>,
        conjunction: &Conjunction,
    ) -> SqlExpr {
        let separator = match conjunction {
            Conjunction::And => " AND\n",
            Conjunction::Or => " OR ",
        };
        let precedence = match conjunction {
            Conjunction::And => SqlExprPrecedence::LogicalAnd,
            Conjunction::Or => SqlExprPrecedence::LogicalOr,
        };
        SqlExpr {
            content: conditions
                .into_iter()
                .filter(|e| !e.is_empty())
                .map(|c| c.for_precedence(precedence).content)
                .collect::<Vec<_>>()
                .join(separator),
            precedence,
        }
    }

    pub fn and(conditions: impl IntoIterator<Item = SqlExpr>) -> SqlExpr {
        condition_set(conditions, &Conjunction::And)
    }

    pub fn comparison(a: SqlExpr, op: &str, b: SqlExpr) -> SqlExpr {
        binary_op(a, op, b, SqlExprPrecedence::Comparison)
    }

    pub fn eq(a: SqlExpr, b: SqlExpr) -> SqlExpr {
        comparison(a, "=", b)
    }

    pub fn neq(a: SqlExpr, b: SqlExpr) -> SqlExpr {
        comparison(a, "<>", b)
    }

    pub fn gt(a: SqlExpr, b: SqlExpr) -> SqlExpr {
        comparison(a, ">", b)
    }

    pub fn gte(a: SqlExpr, b: SqlExpr) -> SqlExpr {
        comparison(a, ">=", b)
    }

    pub fn lt(a: SqlExpr, b: SqlExpr) -> SqlExpr {
        comparison(a, "<", b)
    }

    pub fn lte(a: SqlExpr, b: SqlExpr) -> SqlExpr {
        comparison(a, "<=", b)
    }

    pub fn like(a: SqlExpr, b: SqlExpr) -> SqlExpr {
        comparison(a, "LIKE", b)
    }

    pub fn nlike(a: SqlExpr, b: SqlExpr) -> SqlExpr {
        comparison(a, "NOT LIKE", b)
    }

    pub fn is_null(a: SqlExpr) -> SqlExpr {
        SqlExpr {
            content: format!("{} IS NULL", a.content),
            precedence: SqlExprPrecedence::Comparison,
        }
    }

    pub fn is_not_null(a: SqlExpr) -> SqlExpr {
        SqlExpr {
            content: format!("{} IS NOT NULL", a.content),
            precedence: SqlExprPrecedence::Comparison,
        }
    }
}

pub mod cond {
    use super::*;

    pub fn coalesce(a: SqlExpr) -> SqlExpr {
        sql_func("COALESCE", [a])
    }
}

pub mod func {
    use super::*;

    pub fn now() -> SqlExpr {
        SqlExpr::atom("NOW()".to_string())
    }
}

pub mod math {
    use super::*;

    pub fn abs(a: SqlExpr) -> SqlExpr {
        sql_func("ABS", [a])
    }

    pub fn add(a: SqlExpr, b: SqlExpr) -> SqlExpr {
        binary_op(a, "+", b, SqlExprPrecedence::Addition)
    }

    pub fn ceil(a: SqlExpr) -> SqlExpr {
        sql_func("CEIL", [a])
    }

    pub fn divide(a: SqlExpr, b: SqlExpr) -> SqlExpr {
        binary_op(a, "/", b, SqlExprPrecedence::Multiplication)
    }

    pub fn floor(a: SqlExpr) -> SqlExpr {
        sql_func("FLOOR", [a])
    }

    pub fn greatest(args: Vec<SqlExpr>) -> SqlExpr {
        sql_func("GREATEST", args)
    }

    pub fn least(args: Vec<SqlExpr>) -> SqlExpr {
        sql_func("LEAST", args)
    }

    pub fn modulo(a: SqlExpr, b: SqlExpr) -> SqlExpr {
        binary_op(a, "%", b, SqlExprPrecedence::Multiplication)
    }

    pub fn multiply(a: SqlExpr, b: SqlExpr) -> SqlExpr {
        binary_op(a, "*", b, SqlExprPrecedence::Multiplication)
    }

    pub fn subtract(a: SqlExpr, b: SqlExpr) -> SqlExpr {
        binary_op(a, "-", b, SqlExprPrecedence::Addition)
    }
}

pub mod strings {
    use super::*;

    pub fn lower(a: SqlExpr) -> SqlExpr {
        sql_func("lower", [a])
    }

    pub fn upper(a: SqlExpr) -> SqlExpr {
        sql_func("upper", [a])
    }

    pub fn char_length(a: SqlExpr) -> SqlExpr {
        sql_func("char_length", [a])
    }
}

pub mod value {
    use super::*;

    pub fn infinity() -> SqlExpr {
        SqlExpr::atom("INFINITY".to_string())
    }

    pub fn null() -> SqlExpr {
        SqlExpr::atom("NULL".to_string())
    }

    pub fn true_() -> SqlExpr {
        SqlExpr::atom("TRUE".to_string())
    }

    pub fn false_() -> SqlExpr {
        SqlExpr::atom("FALSE".to_string())
    }

    pub fn zero() -> SqlExpr {
        SqlExpr::atom("0".to_string())
    }
}
