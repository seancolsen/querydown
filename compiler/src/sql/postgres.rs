use itertools::Itertools;
use querydown_parser::ast::{Date, Duration, RegexFlags};

use super::{
    dialect::{
        duration_part, ArrayIndex, Dialect, ElementNames, MaybeLiteral, NormalizedDuration,
        ReduceNames, RegexPattern, Separator,
    },
    expr::{
        build::{
            cmp::{comparison, is_not_null, lt},
            cond::{case, not_true},
            math::{add, subtract},
            sql_func,
            value::{null, one, zero},
        },
        SqlExpr, SqlExprPrecedence,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Postgres();

impl Postgres {
    /// The `FROM` clause of a subquery over the elements of the array named as `names` say (see
    /// [`Postgres::unless_null`]), whose column is named after the element. With `with_position`,
    /// each element's position (counting from 1) is a column too. The subquery is aliased by the
    /// element's name, which is unique within the query.
    fn elements(&self, names: &ElementNames, with_position: bool) -> String {
        let array = self.quote_identifier(&names.array);
        let element = self.quote_identifier(&names.element);
        match with_position {
            true => {
                let position = self.quote_identifier(&names.position);
                format!("FROM unnest({array}) WITH ORDINALITY AS {element}({element}, {position})")
            }
            false => format!("FROM unnest({array}) AS {element}({element})"),
        }
    }

    /// `value`, which refers to the array `a` by the name that `names` give it, or NULL when the
    /// array is NULL. (A subquery over the elements of a NULL array finds none, which would make
    /// the array look empty.) The array is named in a subquery of its own, so that it's written
    /// only once, however many times `value` refers to it.
    fn unless_null(&self, a: SqlExpr, names: &ElementNames, value: String) -> SqlExpr {
        let array = self.quote_identifier(&names.array);
        SqlExpr::atom(format!(
            "(SELECT {value} FROM (SELECT {a}) AS {array}({array}) WHERE {array} IS NOT NULL)"
        ))
    }
}

impl Dialect for Postgres {
    fn introspection_sql(&self) -> &'static str {
        include_str!("../../resources/introspection/postgres.sql")
    }

    fn quote_identifier(&self, ident: &str) -> String {
        // A quoted identifier has no backslash escapes: an embedded double-quote is doubled, and a
        // backslash is a literal character.
        format!(r#""{}""#, ident.replace('"', r#""""#))
    }

    fn quote_string(&self, string: &str) -> String {
        // Postgres uses standard-conforming string literals by default
        // (standard_conforming_strings=on), so backslashes are literal and a single-quote is
        // escaped by doubling it. Backslash-escaping a quote (`\'`) is only valid inside an
        // `E'...'` string and produces a syntax error under the default setting.
        format!("'{}'", string.replace('\'', "''"))
    }

    fn date(&self, date: &Date) -> String {
        format!("DATE '{}'", date.to_iso())
    }

    fn duration(&self, duration: &Duration) -> String {
        // We render durations using the [make_interval] function, which doesn't show zero values
        // for duration parts. When all parts are zero this produces `make_interval()`, which
        // Postgres renders as a zero interval.
        //
        // [make_interval]: https://www.postgresql.org/docs/current/functions-datetime.html

        let d = NormalizedDuration::new(duration);

        let arg = |name: &'static str| move |value: String| format!("{name} => {value}");

        #[rustfmt::skip]
        let args = [
            duration_part(d.years,   arg("years")),
            duration_part(d.months,  arg("months")),
            duration_part(d.weeks,   arg("weeks")),
            duration_part(d.days,    arg("days")),
            duration_part(d.hours,   arg("hours")),
            duration_part(d.minutes, arg("mins")),
            duration_part(d.seconds, arg("secs")),
        ].into_iter().flatten().collect::<Vec<String>>().join(", ");

        format!("make_interval({args})")
    }

    fn regex_newline_options(&self, flags: &RegexFlags) -> &'static str {
        // Postgres's newline handling is set by an embedded option, which must begin the pattern
        // (and of which there may only be one group). Its options don't share Python's letters:
        //
        // | Python flags | `.` matches newline | `^` `$` at lines | Postgres option  |
        // | ------------ | ------------------- | ---------------- | ---------------- |
        // | (none)       | no                  | no               | `p` (partial)    |
        // | `s`          | yes                 | no               | `s` (default)    |
        // | `m`          | no                  | yes              | `n` (newline)    |
        // | `m` `s`      | yes                 | yes              | `w` (weird)      |
        //
        // One divergence remains: when `.` doesn't match a newline, Postgres's negated bracket
        // expressions (e.g. `[^a]`) don't either, whereas Python's do.
        match (flags.multiline, flags.dot_all) {
            (false, false) => "(?p)",
            (false, true) => "",
            (true, false) => "(?n)",
            (true, true) => "(?w)",
        }
    }

    fn match_regex(&self, a: SqlExpr, pattern: RegexPattern, flags: &RegexFlags) -> SqlExpr {
        let op = if flags.case_sensitive { "~" } else { "~*" };
        let prefix = self.regex_newline_options(flags);
        comparison(a, op, self.prefixed_regex_pattern(prefix, pattern))
    }

    fn regex_replace(
        &self,
        a: SqlExpr,
        pattern: RegexPattern,
        replacement: SqlExpr,
        flags: &RegexFlags,
    ) -> SqlExpr {
        // `g` replaces every match rather than the first, and `i` ignores case.
        let pattern = self.prefixed_regex_pattern(self.regex_newline_options(flags), pattern);
        let options = if flags.case_sensitive { "'g'" } else { "'gi'" };
        sql_func(
            "regexp_replace",
            [a, pattern, replacement, SqlExpr::atom(options.to_string())],
        )
    }

    fn regex_extract(
        &self,
        a: SqlExpr,
        pattern: RegexPattern,
        group: u32,
        flags: &RegexFlags,
    ) -> SqlExpr {
        // `regexp_match` gives an array of the text of each capture group, when the pattern has
        // any, and otherwise of the whole match; or NULL when nothing matches. Wrapping the pattern
        // in a group of its own makes the whole match the first element, and each of the pattern's
        // groups the one after. (The embedded options still go first.)
        let pattern = match pattern {
            RegexPattern::Literal(p) => RegexPattern::Literal(format!("({p})")),
            RegexPattern::Expr(e) => RegexPattern::Expr(SqlExpr::atom(format!(
                "('(' || {} || ')')",
                e.for_precedence(SqlExprPrecedence::Atom)
            ))),
        };
        let pattern = self.prefixed_regex_pattern(self.regex_newline_options(flags), pattern);
        let mut args = vec![a, pattern];
        if !flags.case_sensitive {
            args.push(SqlExpr::atom("'i'".to_string()));
        }
        SqlExpr::atom(format!(
            "({})[{}]",
            sql_func("regexp_match", args),
            u64::from(group) + 1
        ))
    }

    fn round(&self, a: SqlExpr, digits: Option<MaybeLiteral<i64>>) -> SqlExpr {
        // Postgres rounds a half to even on `double precision`, but away from zero on `numeric`.
        let a = SqlExpr::atom(format!("CAST({a} AS numeric)"));
        let digits = digits.map(MaybeLiteral::into_sql);
        sql_func("ROUND", std::iter::once(a).chain(digits))
    }

    fn is_nan(&self, a: SqlExpr) -> SqlExpr {
        // Postgres has no `isnan`, but unlike IEEE 754, it treats NaN as equal to itself.
        comparison(
            a,
            "=",
            SqlExpr::atom("CAST('NaN' AS double precision)".to_string()),
        )
    }

    fn seconds_to_duration(&self, a: SqlExpr) -> SqlExpr {
        SqlExpr::atom(format!("make_interval(secs => {a})"))
    }

    fn array(&self, values: Vec<SqlExpr>) -> SqlExpr {
        SqlExpr::atom(format!("ARRAY[{}]", values.iter().join(", ")))
    }

    fn array_at(&self, a: SqlExpr, index: ArrayIndex) -> SqlExpr {
        // Postgres counts an array's elements from 1, and gives NULL for a subscript outside the
        // array, as for 0 or less. Counting from the end takes the array's length.
        let length = || self.array_length(a.clone());
        let subscript = match index {
            ArrayIndex::Literal(i) if i >= 0 => SqlExpr::atom(i.saturating_add(1).to_string()),
            ArrayIndex::Literal(-1) => length(),
            ArrayIndex::Literal(i) => subtract(
                length(),
                SqlExpr::atom((-(i.saturating_add(1))).to_string()),
            ),
            ArrayIndex::Expr(i) => case(
                vec![(lt(i.clone(), zero()), add(add(length(), i.clone()), one()))],
                add(i, one()),
            ),
        };
        // A subscript applies only to a column or to an expression in parentheses.
        SqlExpr::atom(format!("({a})[{subscript}]"))
    }

    fn array_length(&self, a: SqlExpr) -> SqlExpr {
        sql_func("cardinality", [a])
    }

    fn split(&self, a: SqlExpr, separator: Separator) -> SqlExpr {
        // `string_to_array` splits text into its characters at a NULL separator, and doesn't split
        // it at an empty one. So an empty separator becomes NULL, and a NULL one is handled apart.
        match separator {
            Separator::Literal(s) if s.is_empty() => sql_func("string_to_array", [a, null()]),
            Separator::Literal(s) => {
                sql_func("string_to_array", [a, SqlExpr::atom(self.quote_string(&s))])
            }
            Separator::Expr(e) => {
                let nonempty = sql_func("NULLIF", [e.clone(), SqlExpr::atom("''".to_string())]);
                case(
                    vec![(is_not_null(e), sql_func("string_to_array", [a, nonempty]))],
                    null(),
                )
            }
        }
    }

    fn join(&self, a: SqlExpr, separator: Separator) -> SqlExpr {
        // `array_to_string` skips NULL elements.
        let separator = match separator {
            Separator::Literal(s) => SqlExpr::atom(self.quote_string(&s)),
            Separator::Expr(e) => e,
        };
        sql_func("array_to_string", [a, separator])
    }

    fn array_map(&self, a: SqlExpr, names: &ElementNames, body: SqlExpr) -> SqlExpr {
        // Postgres has no lambdas, so the body is evaluated in a subquery over the elements.
        let elements = self.elements(names, true);
        let position = self.quote_identifier(&names.position);
        let array = format!("ARRAY(SELECT {body} {elements} ORDER BY {position})");
        self.unless_null(a, names, array)
    }

    fn array_filter(&self, a: SqlExpr, names: &ElementNames, body: SqlExpr) -> SqlExpr {
        let elements = self.elements(names, true);
        let element = self.quote_identifier(&names.element);
        let position = self.quote_identifier(&names.position);
        let array = format!("ARRAY(SELECT {element} {elements} WHERE {body} ORDER BY {position})");
        self.unless_null(a, names, array)
    }

    fn array_some(&self, a: SqlExpr, names: &ElementNames, body: SqlExpr) -> SqlExpr {
        // A NULL body counts as false, as `WHERE` takes it.
        let elements = self.elements(names, names.uses_position);
        let exists = format!("EXISTS (SELECT 1 {elements} WHERE {body})");
        self.unless_null(a, names, exists)
    }

    fn array_every(&self, a: SqlExpr, names: &ElementNames, body: SqlExpr) -> SqlExpr {
        let elements = self.elements(names, names.uses_position);
        let body = not_true(body);
        let none_fails = format!("NOT EXISTS (SELECT 1 {elements} WHERE {body})");
        self.unless_null(a, names, none_fails)
    }

    fn reduce_params(&self, _a: &SqlExpr, names: &ReduceNames) -> (SqlExpr, SqlExpr) {
        // The next element is the one after the position reached so far, which counts from 1.
        let acc = SqlExpr::atom(self.quote_identifier(&names.acc));
        let array = self.quote_identifier(&names.array);
        let position = self.quote_identifier(&names.position);
        let next = SqlExpr::atom(format!("{array}[{position} + 1]"));
        (acc, next)
    }

    fn array_reduce(&self, a: SqlExpr, names: &ReduceNames, body: SqlExpr) -> SqlExpr {
        // Postgres has no lambdas, nor a function like `reduce`, so the elements are combined by a
        // recursive query, whose rows hold each position (counting from 1), the value accumulated
        // up to it, and the array. It starts at the first element, which is NULL for an empty
        // array, and for a NULL one.
        let acc = self.quote_identifier(&names.acc);
        let array = self.quote_identifier(&names.array);
        let position = self.quote_identifier(&names.position);
        SqlExpr::atom(format!(
            "(WITH RECURSIVE {acc}({position}, {acc}, {array}) AS (\
             SELECT 1, {array}[1], {array} FROM (SELECT {a}) AS {array}({array}) \
             UNION ALL \
             SELECT {position} + 1, {body}, {array} FROM {acc} \
             WHERE {position} < cardinality({array})\
             ) SELECT {acc} FROM {acc} ORDER BY {position} DESC LIMIT 1)"
        ))
    }

    fn text_contains(&self, haystack: SqlExpr, needle: SqlExpr) -> SqlExpr {
        // `strpos` returns the 1-based position of the first match, or 0 when not found, so `> 0`
        // means "contains". `lower(... COLLATE "C")` gives a case-insensitive, byte-wise
        // comparison. `strpos` returns NULL if either argument is NULL, so we coalesce to FALSE.
        //
        // `COLLATE` binds more tightly than any operator, so an operand that isn't atomic is
        // parenthesized: `lower(a + b COLLATE "C")` would collate `b` alone.
        SqlExpr::atom(format!(
            r#"COALESCE(strpos(lower({haystack} COLLATE "C"), lower({needle} COLLATE "C")) > 0, FALSE)"#,
            haystack = haystack.for_precedence(SqlExprPrecedence::Atom).content,
            needle = needle.for_precedence(SqlExprPrecedence::Atom).content,
        )).never_null()
    }

    fn text_eq(&self, a: SqlExpr, b: SqlExpr) -> SqlExpr {
        // The case-insensitive counterpart to `=`, mirroring the `lower(... COLLATE "C")`
        // normalization used by `text_contains`. Standard NULL semantics are preserved.
        let normalize = |e: SqlExpr| {
            let e = e.for_precedence(SqlExprPrecedence::Atom);
            SqlExpr::atom(format!(r#"lower({} COLLATE "C")"#, e.content))
        };
        comparison(normalize(a), "=", normalize(b))
    }

    fn aggregate_product(&self, arg: SqlExpr) -> SqlExpr {
        // Postgres has no native product aggregate, so we reconstruct it from sums of logarithms:
        // the product of the magnitudes is `exp(sum(ln(abs(x))))`, and the sign is negative only
        // when an odd number of values are negative. Zeros are excluded from the logarithm (where
        // `ln` is undefined) and instead short-circuit the whole result to 0.
        let a = arg.content;
        SqlExpr::atom(format!(
            "CASE \
             WHEN bool_or({a} = 0) THEN 0 \
             ELSE (CASE WHEN count(*) FILTER (WHERE {a} < 0) % 2 = 1 THEN -1 ELSE 1 END) \
             * round(exp(sum(ln(abs({a})::double precision)) FILTER (WHERE {a} <> 0))) \
             END"
        ))
    }

    fn unit_hash(&self, arg: SqlExpr) -> SqlExpr {
        // `hashtext` returns a signed 32-bit integer over the full `int4` range. Shifting it up by
        // 2^31 and dividing by the range's width (2^32 - 1) maps it onto a uniform double in
        // [0, 1], mirroring DuckDB's normalization of its unsigned 64-bit hash.
        SqlExpr {
            content: format!(
                "(CAST(hashtext({a}) AS DOUBLE PRECISION) + 2147483648) / 4294967295.0",
                a = arg.content
            ),
            // A division, so that a surrounding operation parenthesizes it where needed.
            precedence: SqlExprPrecedence::Multiplication,
            never_null: false,
        }
    }

    fn year_month(&self, arg: SqlExpr) -> SqlExpr {
        sql_func("to_char", [arg, SqlExpr::atom("'YYYY-MM'".to_string())])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quote_identifier() {
        let quote = |ident: &str| Postgres().quote_identifier(ident);
        assert_eq!(quote("issues"), r#""issues""#);
        assert_eq!(quote(r#"say "hi""#), r#""say ""hi""""#);
        assert_eq!(quote(r"a\b"), r#""a\b""#);
    }
}
