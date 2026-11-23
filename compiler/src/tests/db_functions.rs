//! Runs built-in functions against rows of real Postgres and DuckDB databases, asserting that each
//! dialect gives the same values. This checks the _meaning_ of the SQL that a function compiles to,
//! which the corpus (whose DB test only plans queries) can't: where the databases' own functions
//! differ (in rounding a half, counting positions, matching nothing, splitting empty text…), the
//! compiler must make up the difference.
//!
//! Gated behind the `db-tests` cargo feature; see [`super::db_introspection`] for the environments.

use super::db_introspection::{DuckEnv, PgEnv};
use crate::{Compiler, Dialect, DuckDB, IdentifierResolution, Options, Postgres};

/// One table of assorted values, in rows that each function's cases read.
const DDL: &str = "\
CREATE TABLE vals (id integer PRIMARY KEY, x double precision, n integer, s text, p text);
INSERT INTO vals VALUES
  (1, 2.5, 90, 'Fix: one fix', 'f(i)x'),
  (2, -2.5, -1, 'a,b,c', ','),
  (3, 1250, NULL, '', NULL),
  (4, CAST('NaN' AS double precision), 0, NULL, ''),
  (5, NULL, NULL, 'x' || chr(10) || 'y', NULL);
";

/// How a NULL value is shown in the expected values.
const NULL: &str = "∅";

/// Each expression, and the values it should give for the rows of `vals`, in order of their ids,
/// separated by `|`. A NULL value is shown as `∅`, and a number as Rust formats an `f64`.
const CASES: &[(&str, &str)] = &[
    // A half rounds away from zero, and a negative number of digits rounds to tens, hundreds…
    (r"x|round", "3|-3|1250|NaN|∅"),
    (r"x|round(1)", "2.5|-2.5|1250|NaN|∅"),
    (r"x|round(-2)", "0|0|1300|NaN|∅"),
    (r"n|round(-1)", "90|0|∅|0|∅"),
    (r"n|times(-5)|round(-1)", "-450|10|∅|0|∅"),
    (r"x|round(n)", "2.5|0|∅|NaN|∅"),
    (r"x|round(n|minus(1))", "2.5|0|∅|NaN|∅"),
    (r"x|is_nan", "false|false|false|true|∅"),
    (r"n|gcd(12)", "6|1|∅|12|∅"),
    (r"n|lcm(4)", "180|4|∅|0|∅"),
    (r"n|seconds_to_duration|seconds", "90|-1|∅|0|∅"),
    (
        r#"@2000-01-01|plus(n|times(3600)|seconds_to_duration)|to_date"#,
        "2000-01-04|1999-12-31|∅|2000-01-01|∅",
    ),
    // Conversions from text.
    (r#""12.5"|to_number|times(n)"#, "1125|-12.5|∅|0|∅"),
    (
        r#""2024-01-02T10:30:00"|to_timestamp|minus(@2024-01-02)|hours"#,
        "10.5|10.5|10.5|10.5|10.5",
    ),
    (r#""2024-01-02"|to_date|minus(@2024-01-01)"#, "1|1|1|1|1"),
    // Positions count from 0, or from the end when negative. Outside the array, there's nothing.
    (r#"s|split(",")|at(0)"#, "Fix: one fix|a|∅|∅|x\ny"),
    (r#"s|split(",")|at(1)"#, "∅|b|∅|∅|∅"),
    (r#"s|split(",")|at(-1)"#, "Fix: one fix|c|∅|∅|x\ny"),
    (r#"s|split(",")|at(-3)"#, "∅|a|∅|∅|∅"),
    (r#"s|split(",")|at(-4)"#, "∅|∅|∅|∅|∅"),
    (r#"s|split(",")|at(3)"#, "∅|∅|∅|∅|∅"),
    (r#"s|split(",")|at(n)"#, "∅|c|∅|∅|∅"),
    (r#"s|split(",")|at(n|plus(1))"#, "∅|a|∅|∅|∅"),
    // Empty text has no parts, and an empty separator splits text into its characters.
    (r#"s|split(",")|length"#, "1|3|0|∅|1"),
    (r#"s|split("")|length"#, "12|5|0|∅|3"),
    (r#"s|split(p)|length"#, "1|3|∅|∅|∅"),
    (r#""a,b"|split(p)|length"#, "1|2|∅|3|∅"),
    (r#"s|length"#, "12|5|0|∅|3"),
    (r#"@@array(x n)|length"#, "2|2|2|2|2"),
    // `join` skips NULLs, and `split` undoes it.
    (
        r#"@@array(s @null "z")|join(",")"#,
        "Fix: one fix,z|a,b,c,z|,z|z|x\ny,z",
    ),
    (
        r#"@@array(s @null "z")|join(",")|split(",")|join("/")"#,
        "Fix: one fix/z|a/b/c/z|/z|z|x\ny/z",
    ),
    (r#"@@array(@null s)|join(",")"#, "Fix: one fix|a,b,c|||x\ny"),
    (r#"@@array("a" "b")|join(p)"#, "af(i)xb|a,b|∅|ab|∅"),
    // A match, or a group of it, or NULL when nothing matches.
    (r"s|regex_extract('f(i)x')", "Fix|∅|∅|∅|∅"),
    (r"s|regex_extract('f(i)x' 1)", "i|∅|∅|∅|∅"),
    (r#"s|regex_extract('f(i)x' 0 "c")"#, "fix|∅|∅|∅|∅"),
    (r"s|regex_extract('[a-z]+$')", "fix|c|∅|∅|y"),
    (r#"s|regex_extract('^[a-z]+$' 0 "m")"#, "∅|∅|∅|∅|x"),
    (r"s|regex_extract('x*')", "|||∅|x"),
    (r"s|regex_extract(p)", "Fix|,|∅|∅|∅"),
    // Every match is replaced, with `\1` standing for the first group.
    (
        r"s|regex_replace('f(i)x' '<\1>')",
        "<i>: one <i>|a,b,c||∅|x\ny",
    ),
    (
        r#"s|regex_replace('f(i)x' '<\1>' "c")"#,
        "Fix: one <i>|a,b,c||∅|x\ny",
    ),
    (r#"s|regex_replace('^y' "Y")"#, "Fix: one fix|a,b,c||∅|x\ny"),
    (
        r#"s|regex_replace('^y' "Y" "m")"#,
        "Fix: one fix|a,b,c||∅|x\nY",
    ),
    (
        r#"s|regex_replace('x.y' "-")"#,
        "Fix: one fix|a,b,c||∅|x\ny",
    ),
    (
        r#"s|regex_replace('x.y' "-" "s")"#,
        "Fix: one fix|a,b,c||∅|-",
    ),
    (r#"s|regex_replace(p "-")"#, "-: one -|a-b-c|∅|∅|∅"),
    // `map` keeps the elements' order, and a NULL array stays NULL (rather than becoming empty).
    (
        r#"s|split(",")|map(@e => @e|uppercase)|join("/")"#,
        "FIX: ONE FIX|A/B/C||∅|X\nY",
    ),
    (r#"s|split(",")|map(@e => @e)|length"#, "1|3|0|∅|1"),
    // Positions count from 0.
    (
        r#"s|split(",")|map(@e @i => ^f"{@i}{@e}")|join(" ")"#,
        "0Fix: one fix|0a 1b 2c||∅|0x\ny",
    ),
    (
        r#"s|split(",")|filter(@e @i => @i:<2)|join("")"#,
        "Fix: one fix|ab||∅|x\ny",
    ),
    (
        r#"s|split(",")|some(@e @i => @i:=2)"#,
        "false|true|false|∅|false",
    ),
    // The body sees the row's columns, and the variables around it.
    (r"@@array(1 2 3)|filter(@e => @e:<`n`)|length", "3|0|0|0|0"),
    (r"@@array(1 2)|map(@e => @e|times(n))|at(1)", "180|-2|∅|0|∅"),
    (
        r"n|(@m => @@array(1 2)|map(@e => @e|times(@m))|at(1))",
        "180|-2|∅|0|∅",
    ),
    // A parameter shadows a variable of the same name, and may be named like an SQL keyword.
    (
        r"n|(@e => @@array(1 2)|map(@e => @e + 1)|at(1))",
        "3|3|3|3|3",
    ),
    (r"@@array(1 2)|map(@order => @order + 1)|at(0)", "2|2|2|2|2"),
    // Functions within functions: the characters of each part, at that part's position.
    (
        r#"s|split(",")|map(@e @i => @e|split("")|filter(@c @j => @j:=@i)|join(""))|join("")"#,
        "F|a||∅|x",
    ),
    // `some` is false for an empty array, and `every` true. A NULL body counts as false.
    (
        r#"s|split(",")|some(@e => @e:=="b")"#,
        "false|true|false|∅|false",
    ),
    (
        r#"s|split(",")|every(@e => @e|length:=1)"#,
        "false|true|true|∅|false",
    ),
    (
        r"@@array(n)|some(@e => @e:>0)",
        "true|false|false|false|false",
    ),
    (
        r"@@array(n)|every(@e => @e:>=0)",
        "true|false|false|true|false",
    ),
    (
        r"@@array(n 1)|every(@e => @e:>0)",
        "true|false|false|false|false",
    ),
    // `reduce` starts from the first element: NULL for an empty array, and the element itself for
    // an array of one.
    (
        r#"s|split(",")|reduce(@a @b => ^f"{@a}+{@b}")"#,
        "Fix: one fix|a+b+c|∅|∅|x\ny",
    ),
    (r"@@array(n 2 3)|reduce(@a @b => @a + @b)", "95|4|∅|5|∅"),
    (r"@@array(n 2 3)|reduce(@a @b => @a|max(@b))", "90|3|3|3|3"),
    (
        r"@@array(1 2 3)|reduce(@a @b => @a * n + @b)",
        "8283|2|∅|3|∅",
    ),
    (
        r"@@array(1 2)|reduce(@a @b => @@array(@a @b n)|reduce(@x @y => @x + @y))",
        "93|2|∅|3|∅",
    ),
];

#[test]
fn test_functions_postgres() {
    let env = PgEnv::setup("functions", DDL);
    check(Box::new(Postgres()), |sql| env.run_query(sql));
}

#[test]
fn test_functions_duckdb() {
    let env = DuckEnv::setup(DDL);
    check(Box::new(DuckDB()), |sql| env.run_query(sql));
}

/// Compiles each case for `dialect` and runs it with `run`, which returns a query's lone text value.
fn check(dialect: Box<dyn Dialect>, run: impl Fn(&str) -> String) {
    let schema = run(dialect.introspection_sql());
    let options = Options {
        dialect,
        identifier_resolution: IdentifierResolution::Flexible,
        limit: None,
        offset: None,
    };
    let compiler = Compiler::new(&schema, options).expect("introspected schema should compile");
    let failures: Vec<String> = CASES
        .iter()
        .filter_map(|(expr, expected)| {
            let query = format!("#vals $id $({expr}) -> v");
            let sql = compiler
                .compile(query.clone())
                .unwrap_or_else(|e| panic!("{query} failed to compile: {}", e.render(&query)))
                .sql;
            // Collapse the values into one, in order of the rows' ids.
            let values = run(&format!(
                "SELECT string_agg(COALESCE(CAST(v AS text), '{NULL}'), '|' ORDER BY id) \
                 FROM ({}) AS t",
                sql.trim_end().trim_end_matches(';')
            ));
            let values = normalize(&values);
            (values != *expected)
                .then(|| format!("{query}\n  expected [{expected}], found [{values}]\n{sql}"))
        })
        .collect();
    assert!(failures.is_empty(), "\n{}", failures.join("\n\n"));
}

/// Writes each number among `values` as Rust formats an `f64`, since the databases format them
/// differently (`3` and `3.0`, `NaN` and `nan`, `0` and `-0.0`).
fn normalize(values: &str) -> String {
    values
        .split('|')
        .map(|value| match value.parse::<f64>() {
            // Without a sign on zero, which rounding a negative number can leave in DuckDB.
            Ok(number) if number == 0.0 => "0".to_string(),
            Ok(number) => number.to_string(),
            Err(_) => value.to_string(),
        })
        .collect::<Vec<_>>()
        .join("|")
}
