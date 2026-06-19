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

pub fn sql_func(name: &str, args: impl IntoIterator<Item = SqlExpr>) -> SqlExpr {
    SqlExpr::atom(format!("{}({})", name, args.into_iter().join(", ")))
}

pub mod agg {
    use super::*;

    fn with_order(name: &str, a: SqlExpr, order_by: String) -> SqlExpr {
        if order_by.is_empty() {
            sql_func(name, [a])
        } else {
            SqlExpr::atom(format!("{}({} ORDER BY {})", name, a.content, order_by))
        }
    }

    pub fn array_agg(a: SqlExpr, order_by: String) -> SqlExpr {
        with_order("array_agg", a, order_by)
    }

    pub fn bool_and(a: SqlExpr, order_by: String) -> SqlExpr {
        with_order("bool_and", a, order_by)
    }

    pub fn bool_or(a: SqlExpr, order_by: String) -> SqlExpr {
        with_order("bool_or", a, order_by)
    }

    pub fn avg(a: SqlExpr, order_by: String) -> SqlExpr {
        with_order("avg", a, order_by)
    }

    pub fn count(a: SqlExpr, order_by: String) -> SqlExpr {
        with_order("count", a, order_by)
    }

    pub fn count_star() -> SqlExpr {
        SqlExpr::atom("count(*)".to_string())
    }

    pub fn count_distinct(a: SqlExpr, order_by: String) -> SqlExpr {
        // TODO: We should alter the query at a higher level to use an approach like this for
        // better performance:
        // https://stackoverflow.com/questions/11250253/postgresql-countdistinct-very-slow
        if order_by.is_empty() {
            SqlExpr::atom(format!("count(DISTINCT {})", a.content))
        } else {
            SqlExpr::atom(format!(
                "count(DISTINCT {} ORDER BY {})",
                a.content, order_by
            ))
        }
    }

    pub fn max(a: SqlExpr, order_by: String) -> SqlExpr {
        with_order("max", a, order_by)
    }

    pub fn min(a: SqlExpr, order_by: String) -> SqlExpr {
        with_order("min", a, order_by)
    }

    pub fn sum(a: SqlExpr, order_by: String) -> SqlExpr {
        with_order("sum", a, order_by)
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

    pub fn or(conditions: impl IntoIterator<Item = SqlExpr>) -> SqlExpr {
        condition_set(conditions, &Conjunction::Or)
    }

    /// Exclusive-or across the values, true when an odd number of them are true. Built by chaining
    /// `<>` (inequality), since for booleans `a <> b` is true exactly when they differ.
    pub fn xor(conditions: impl IntoIterator<Item = SqlExpr>) -> SqlExpr {
        conditions
            .into_iter()
            .reduce(|acc, b| {
                // `<>` is non-associative in SQL, so each operand that is itself a comparison (or
                // looser) must be parenthesized. Forcing `Atom` precedence wraps anything that
                // isn't already atomic, keeping the chain valid as `((a <> b) <> c)`.
                let a = acc.for_precedence(SqlExprPrecedence::Atom);
                let b = b.for_precedence(SqlExprPrecedence::Atom);
                SqlExpr {
                    content: format!("{} <> {}", a.content, b.content),
                    precedence: SqlExprPrecedence::Comparison,
                }
            })
            .unwrap_or_else(super::value::false_)
    }

    pub fn comparison(a: SqlExpr, op: &str, b: SqlExpr) -> SqlExpr {
        binary_op(a, op, b, SqlExprPrecedence::Comparison)
    }

    pub fn eq(a: SqlExpr, b: SqlExpr) -> SqlExpr {
        comparison(a, "=", b)
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

    /// Build a searched `CASE` expression. Each `(condition, value)` pair becomes a `WHEN ... THEN
    /// ...` arm, and `fallback` becomes the `ELSE`. The syntax is identical for Postgres and DuckDB.
    ///
    /// The result is atomic (delimited by `CASE`/`END`), so it never needs parenthesizing.
    pub fn case(variants: Vec<(SqlExpr, SqlExpr)>, fallback: SqlExpr) -> SqlExpr {
        let mut content = String::from("CASE");
        for (condition, value) in variants {
            content.push_str(&format!(
                "\n  WHEN {} THEN {}",
                condition.content, value.content
            ));
        }
        content.push_str(&format!("\n  ELSE {}\nEND", fallback.content));
        SqlExpr::atom(content)
    }

    pub fn coalesce(args: Vec<SqlExpr>) -> SqlExpr {
        sql_func("COALESCE", args)
    }

    pub fn not(a: SqlExpr) -> SqlExpr {
        SqlExpr {
            content: format!(
                "NOT {}",
                a.for_precedence(SqlExprPrecedence::LogicalNot).content
            ),
            precedence: SqlExprPrecedence::LogicalNot,
        }
    }
}

pub mod date_time {
    use super::*;

    pub fn years(a: SqlExpr) -> SqlExpr {
        // A year is treated as 365.25 days (31557600 seconds), matching the duration handling in
        // `crate::sql::dialect`.
        math::divide(extract_epoch(a), SqlExpr::atom("31557600".to_string()))
    }

    pub fn days(a: SqlExpr) -> SqlExpr {
        math::divide(extract_epoch(a), SqlExpr::atom("86400".to_string()))
    }

    pub fn hours(a: SqlExpr) -> SqlExpr {
        math::divide(extract_epoch(a), SqlExpr::atom("3600".to_string()))
    }

    pub fn minutes(a: SqlExpr) -> SqlExpr {
        math::divide(extract_epoch(a), SqlExpr::atom("60".to_string()))
    }

    pub fn seconds(a: SqlExpr) -> SqlExpr {
        extract_epoch(a)
    }

    pub fn extract_epoch(a: SqlExpr) -> SqlExpr {
        SqlExpr::atom(format!("EXTRACT(epoch FROM {})", a.content))
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

    pub fn concat(args: Vec<SqlExpr>) -> SqlExpr {
        sql_func("concat", args)
    }

    pub fn trim(a: SqlExpr) -> SqlExpr {
        sql_func("trim", [a])
    }

    pub fn md5(a: SqlExpr) -> SqlExpr {
        sql_func("md5", [a])
    }
}

pub mod value {
    use super::*;

    pub fn infinity() -> SqlExpr {
        // A bare `INFINITY` keyword is not valid in either Postgres or DuckDB; both require casting
        // the `'infinity'` string literal to a floating-point type.
        SqlExpr::atom("CAST('infinity' AS double precision)".to_string())
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
