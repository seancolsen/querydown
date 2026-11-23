use itertools::Itertools;
use querydown_parser::ast::{Date, Duration, RegexFlags};

use super::{
    dialect::{
        duration_part, ArrayIndex, Dialect, ElementNames, MaybeLiteral, NormalizedDuration,
        ReduceNames, RegexPattern, Separator,
    },
    expr::{
        build::{
            cmp::{comparison, eq, gt, is_not_null, is_null, lt},
            cond::{case, not_true},
            func,
            math::add,
            sql_func,
            value::{null, one, zero},
        },
        SqlExpr, SqlExprPrecedence,
    },
    Postgres,
};

/// The arguments of a DuckDB regex function that takes options last: `args`, then `'i'` to ignore
/// case unless `flags` ask to match it.
fn regex_args<const N: usize>(args: [SqlExpr; N], flags: &RegexFlags) -> Vec<SqlExpr> {
    let mut args = Vec::from(args);
    if !flags.case_sensitive {
        args.push(SqlExpr::atom("'i'".to_string()));
    }
    args
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DuckDB();

impl DuckDB {
    /// A lambda whose variables are an element of a list and, when the body uses it, the element's
    /// position (counting from 1), named as `names` say.
    fn element_lambda(&self, names: &ElementNames, body: SqlExpr) -> SqlExpr {
        let mut params = self.quote_identifier(&names.element);
        if names.uses_position {
            params = format!("{params}, {}", self.quote_identifier(&names.position));
        }
        SqlExpr::atom(format!("lambda {params}: {body}"))
    }
}

impl Dialect for DuckDB {
    fn introspection_sql(&self) -> &'static str {
        include_str!("../../resources/introspection/duckdb.sql")
    }

    // For now, DuckDB mirrors Postgres for everything except the methods we've specifically
    // targeted for dialect differences. We delegate the shared behavior to Postgres to avoid
    // duplicating its logic. As we identify more dialect differences, these delegations can be
    // replaced with DuckDB-specific implementations.

    fn quote_identifier(&self, ident: &str) -> String {
        // Unlike Postgres, DuckDB does not support backslash escapes within quoted identifiers. It
        // uses standard SQL escaping, where an embedded double-quote is doubled and a backslash is
        // a literal character.
        format!(r#""{}""#, ident.replace('"', r#""""#))
    }

    fn quote_string(&self, string: &str) -> String {
        // Unlike Postgres, DuckDB does not support backslash escapes within string literals. It
        // uses standard SQL escaping, where an embedded single-quote is doubled and a backslash is
        // a literal character.
        format!("'{}'", string.replace('\'', "''"))
    }

    fn date(&self, date: &Date) -> String {
        Postgres().date(date)
    }

    fn duration(&self, duration: &Duration) -> String {
        // DuckDB has no equivalent to Postgres's `make_interval` function. Instead it provides a
        // family of `to_*` functions, each producing an interval for a single unit, which we add
        // together. We omit parts that are zero, falling back to `to_seconds(0)` when the entire
        // duration is zero.
        let d = NormalizedDuration::new(duration);

        let func = |name: &'static str| move |value: String| format!("{name}({value})");

        #[rustfmt::skip]
        let parts = [
            duration_part(d.years,   func("to_years")),
            duration_part(d.months,  func("to_months")),
            duration_part(d.weeks,   func("to_weeks")),
            duration_part(d.days,    func("to_days")),
            duration_part(d.hours,   func("to_hours")),
            duration_part(d.minutes, func("to_minutes")),
            duration_part(d.seconds, func("to_seconds")),
        ].into_iter().flatten().collect::<Vec<String>>();

        match parts.len() {
            0 => "to_seconds(0)".to_string(),
            1 => parts.into_iter().next().unwrap(),
            // Parenthesize a multi-part sum so that the duration behaves as a single atomic value
            // in surrounding expressions. Without this, `NOW() - to_years(1) + to_months(1)` would
            // wrongly parse as `(NOW() - to_years(1)) + to_months(1)`.
            _ => format!("({})", parts.join(" + ")),
        }
    }

    fn regex_newline_options(&self, flags: &RegexFlags) -> &'static str {
        // DuckDB's RE2 engine shares Python's defaults (`.` doesn't match a newline; `^` and `$`
        // match only at the ends of the text) and its `m` and `s` inline flags, which we write
        // into the pattern. (DuckDB's own `'m'` and `'s'` options mean something else.)
        match (flags.multiline, flags.dot_all) {
            (false, false) => "",
            (false, true) => "(?s)",
            (true, false) => "(?m)",
            (true, true) => "(?ms)",
        }
    }

    fn match_regex(&self, a: SqlExpr, pattern: RegexPattern, flags: &RegexFlags) -> SqlExpr {
        // DuckDB doesn't support Postgres's `~*` family of regex operators. Instead it provides
        // the `regexp_matches` function, with an optional `'i'` argument for case-insensitivity.
        let b = self.prefixed_regex_pattern(self.regex_newline_options(flags), pattern);
        sql_func("regexp_matches", regex_args([a, b], flags))
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
        // `regexp_extract` gives the empty string when nothing matches, so it's guarded by a
        // match, to give NULL instead.
        let matches = self.match_regex(a.clone(), pattern.clone(), flags);
        let pattern = self.prefixed_regex_pattern(self.regex_newline_options(flags), pattern);
        let group = SqlExpr::atom(group.to_string());
        let extract = sql_func("regexp_extract", regex_args([a, pattern, group], flags));
        case(vec![(matches, extract)], null())
    }

    fn round(&self, a: SqlExpr, digits: Option<MaybeLiteral<i64>>) -> SqlExpr {
        // DuckDB rounds a half away from zero. But to a negative number of digits, it rounds NaN
        // and infinity to 0, so they're kept apart unless the digits are known not to be negative.
        // And it rounds a `DECIMAL` only to a constant number of digits, so to any other, the
        // number is rounded as a `DOUBLE`.
        match digits {
            None => sql_func("round", [a]),
            Some(MaybeLiteral::Literal(n)) if n >= 0 => {
                sql_func("round", [a, SqlExpr::atom(n.to_string())])
            }
            Some(digits) => {
                let number = match digits {
                    MaybeLiteral::Literal(_) => a.clone(),
                    MaybeLiteral::Expr(_) => func::cast(a.clone(), "DOUBLE"),
                };
                case(
                    vec![(
                        sql_func("isfinite", [a.clone()]),
                        sql_func("round", [number, digits.into_sql()]),
                    )],
                    a,
                )
            }
        }
    }

    fn is_nan(&self, a: SqlExpr) -> SqlExpr {
        sql_func("isnan", [a])
    }

    fn seconds_to_duration(&self, a: SqlExpr) -> SqlExpr {
        sql_func("to_seconds", [a])
    }

    fn array(&self, values: Vec<SqlExpr>) -> SqlExpr {
        SqlExpr::atom(format!("[{}]", values.iter().join(", ")))
    }

    fn array_at(&self, a: SqlExpr, index: ArrayIndex) -> SqlExpr {
        // DuckDB counts a list's elements from 1, and from the end with a negative index (so -1 is
        // the last). It gives NULL for an index outside the list, as for 0.
        let index = match index {
            ArrayIndex::Literal(i) if i >= 0 => SqlExpr::atom(i.saturating_add(1).to_string()),
            ArrayIndex::Literal(i) => SqlExpr::atom(i.to_string()),
            ArrayIndex::Expr(i) => case(vec![(lt(i.clone(), zero()), i.clone())], add(i, one())),
        };
        sql_func("list_extract", [a, index])
    }

    fn array_length(&self, a: SqlExpr) -> SqlExpr {
        sql_func("len", [a])
    }

    fn split(&self, a: SqlExpr, separator: Separator) -> SqlExpr {
        // `string_split` gives one empty part for empty text, rather than none, and doesn't split
        // text at a NULL separator. Both are handled apart. (At an empty separator, it splits text
        // into its characters.)
        let mut variants = Vec::new();
        let separator = match separator {
            Separator::Literal(s) => SqlExpr::atom(self.quote_string(&s)),
            Separator::Expr(e) => {
                variants.push((is_null(e.clone()), null()));
                e
            }
        };
        variants.push((
            eq(a.clone(), SqlExpr::atom("''".to_string())),
            SqlExpr::atom("[]".to_string()),
        ));
        case(variants, sql_func("string_split", [a, separator]))
    }

    fn join(&self, a: SqlExpr, separator: Separator) -> SqlExpr {
        // `array_to_string` skips NULL elements, but gives NULL rather than the empty string when
        // there are only NULLs, so they're filtered out first.
        let elements = format!("list_filter({a}, lambda e: e IS NOT NULL)");
        match separator {
            Separator::Literal(s) => sql_func(
                "array_to_string",
                [
                    SqlExpr::atom(elements),
                    SqlExpr::atom(self.quote_string(&s)),
                ],
            ),
            // `array_to_string` needs a constant separator, so any other is written between the
            // elements by `list_reduce`, which starts from the empty string, and so puts the
            // separator before the first element too, where it's cut off. (A NULL separator would
            // make the result NULL, but `list_reduce` doesn't call the function on an empty list.)
            Separator::Expr(e) => {
                let e = e.for_precedence(SqlExprPrecedence::Atom);
                let reduced = format!(
                    "list_reduce(list_transform({elements}, lambda e: CAST(e AS VARCHAR)), \
                     lambda acc, e: acc || {e} || e, '')"
                );
                case(
                    vec![(
                        is_not_null(e.clone()),
                        SqlExpr::atom(format!("substr({reduced}, length({e}) + 1)")),
                    )],
                    null(),
                )
            }
        }
    }

    fn reserved_lambda_names(&self) -> &'static [&'static str] {
        // The variables of `join`'s lambdas.
        &["e", "acc"]
    }

    fn allows_subquery_in_function_arg(&self) -> bool {
        // The function becomes a lambda, and DuckDB doesn't allow a subquery in one.
        false
    }

    fn array_map(&self, a: SqlExpr, names: &ElementNames, body: SqlExpr) -> SqlExpr {
        sql_func("list_transform", [a, self.element_lambda(names, body)])
    }

    fn array_filter(&self, a: SqlExpr, names: &ElementNames, body: SqlExpr) -> SqlExpr {
        sql_func("list_filter", [a, self.element_lambda(names, body)])
    }

    fn array_some(&self, a: SqlExpr, names: &ElementNames, body: SqlExpr) -> SqlExpr {
        // `list_filter` keeps the elements for which the body is true, and not those for which
        // it's NULL. The length of a NULL list is NULL.
        let passing = sql_func("list_filter", [a, self.element_lambda(names, body)]);
        gt(sql_func("len", [passing]), zero())
    }

    fn array_every(&self, a: SqlExpr, names: &ElementNames, body: SqlExpr) -> SqlExpr {
        let failing = sql_func(
            "list_filter",
            [a, self.element_lambda(names, not_true(body))],
        );
        eq(sql_func("len", [failing]), zero())
    }

    fn reduce_params(&self, _a: &SqlExpr, names: &ReduceNames) -> (SqlExpr, SqlExpr) {
        let acc = SqlExpr::atom(self.quote_identifier(&names.acc));
        let element = SqlExpr::atom(self.quote_identifier(&names.element));
        (acc, element)
    }

    fn array_reduce(&self, a: SqlExpr, names: &ReduceNames, body: SqlExpr) -> SqlExpr {
        // `list_reduce` fails on an empty list, so that's made NULL, which it reduces to NULL.
        let nonempty = case(vec![(eq(sql_func("len", [a.clone()]), zero()), null())], a);
        let acc = self.quote_identifier(&names.acc);
        let element = self.quote_identifier(&names.element);
        let lambda = SqlExpr::atom(format!("lambda {acc}, {element}: {body}"));
        sql_func("list_reduce", [nonempty, lambda])
    }

    fn text_contains(&self, haystack: SqlExpr, needle: SqlExpr) -> SqlExpr {
        // `contains` returns a boolean for substring presence. `lower(strip_accents(...))` makes the
        // comparison case- and accent-insensitive. `contains` returns NULL if either argument is
        // NULL, so we coalesce to FALSE.
        SqlExpr::atom(format!(
            "COALESCE(contains(lower(strip_accents({haystack})), lower(strip_accents({needle}))), FALSE)",
            haystack = haystack.content,
            needle = needle.content,
        )).never_null()
    }

    fn text_eq(&self, a: SqlExpr, b: SqlExpr) -> SqlExpr {
        // The case- and accent-insensitive counterpart to `=`, mirroring the
        // `lower(strip_accents(...))` normalization used by `text_contains`. Standard NULL
        // semantics are preserved.
        let normalize = |e: SqlExpr| sql_func("lower", [sql_func("strip_accents", [e])]);
        comparison(normalize(a), "=", normalize(b))
    }

    fn aggregate_product(&self, arg: SqlExpr) -> SqlExpr {
        // Unlike Postgres, DuckDB provides a native `product` aggregate.
        sql_func("product", [arg])
    }

    fn unit_hash(&self, arg: SqlExpr) -> SqlExpr {
        // `hash` returns an unsigned 64-bit integer (UBIGINT); dividing by its maximum value maps
        // it onto a uniform double in [0, 1].
        SqlExpr {
            content: format!(
                "CAST(hash({a}) AS DOUBLE) / 18446744073709551615.0",
                a = arg.content
            ),
            // A division, so that a surrounding operation parenthesizes it where needed.
            precedence: SqlExprPrecedence::Multiplication,
            never_null: false,
        }
    }

    fn year_month(&self, arg: SqlExpr) -> SqlExpr {
        sql_func("strftime", [arg, SqlExpr::atom("'%Y-%m'".to_string())])
    }

    fn coerce_temporal_to_naive(&self, e: SqlExpr) -> SqlExpr {
        // DuckDB built without the ICU extension can't perform arithmetic or comparison involving a
        // `timestamptz` (e.g. `@now`) unless the timezone data the ICU extension would supply is
        // present — this applies even to `timestamptz - interval`. Casting the zoned value down to
        // a naive `timestamp` sidesteps the issue entirely (the zone is simply dropped, yielding
        // the UTC wall-clock value). We use `CAST(... AS TIMESTAMP)` rather than `...::TIMESTAMP` so
        // the result is atomic regardless of the inner expression's precedence.
        SqlExpr::atom(format!("CAST({} AS TIMESTAMP)", e.content))
    }

    fn coerces_timestamptz(&self) -> bool {
        true
    }
}
