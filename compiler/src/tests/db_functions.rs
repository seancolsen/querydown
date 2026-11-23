//! Runs built-in functions against rows of real Postgres and DuckDB databases, asserting that each
//! dialect gives the same values. This checks the _meaning_ of the SQL that a function compiles to,
//! which the corpus (whose DB test only plans queries) can't: where the databases' own functions
//! differ (in rounding a half, counting positions, matching nothing, splitting empty text…), the
//! compiler must make up the difference.
//!
//! Gated behind the `db-tests` cargo feature; see [`super::db_introspection`] for the environments.

use super::db_introspection::{DuckEnv, PgEnv};
use crate::{Compiler, Dialect, DuckDB, IdentifierResolution, Options, Postgres};

/// One table of assorted values, in rows that each function's cases read. `JSON_TYPE` stands for
/// the dialect's type of JSON (see [`ddl`]).
const DDL: &str = r#"
CREATE TABLE vals (id integer PRIMARY KEY, x double precision, n integer, s text, p text, j JSON_TYPE);
INSERT INTO vals VALUES
  (1, 2.5, 90, 'Fix: one fix', 'f(i)x', '{"items": [{"sku": "A1", "price": 12.5}, {"sku": "B2", "price": 8}], "tags": ["urgent", "ui"], "name": "x", "n": 3, "flag": true, "nothing": null, "nested": {"a": [1, 2], "b": "c"}, "k\"q": 1, "0": "zero"}'),
  (2, -2.5, -1, 'a,b,c', ',', '[10, "b", null, [1], {"c": 1}]'),
  (3, 1250, NULL, '', NULL, '{}'),
  (4, CAST('NaN' AS double precision), 0, NULL, '', NULL),
  (5, NULL, NULL, 'x' || chr(10) || 'y', NULL, '"just a string"');
CREATE TABLE groups (id integer PRIMARY KEY);
INSERT INTO groups VALUES (1), (2), (3), (4), (5);
CREATE TABLE members (id integer PRIMARY KEY, grp integer REFERENCES groups(id), v double precision, d interval);
INSERT INTO members VALUES
  (1, 1, 10, INTERVAL '10 seconds'), (2, 1, 1, INTERVAL '1 second'), (3, 1, 2, INTERVAL '2 seconds'),
  (4, 2, 3, NULL), (5, 2, 1, NULL), (6, 2, 10, NULL), (7, 2, 2, NULL),
  (8, 3, 4, INTERVAL '1 day'), (9, 3, NULL, NULL), (10, 3, 6, INTERVAL '2 days'),
  (11, 4, NULL, NULL);
"#;

/// [`DDL`] for a dialect whose type of JSON is `json_type`.
fn ddl(json_type: &str) -> String {
    DDL.replace("JSON_TYPE", json_type)
}

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
    // Timestamp literals.
    (
        r#"@2024-01-02T10:30|minus(@2024-01-02)|hours"#,
        "10.5|10.5|10.5|10.5|10.5",
    ),
    (
        r#"@2024-01-02T10:30:15.25|minus(@2024-01-02T10:30)|seconds"#,
        "15.25|15.25|15.25|15.25|15.25",
    ),
    (
        r#"@2024-01-02T10:30:00.000001|minus(@2024-01-02T10:30)|seconds"#,
        "0.000001|0.000001|0.000001|0.000001|0.000001",
    ),
    (
        r#"@2024-02-28T23:00|plus(n|times(3600)|seconds_to_duration)|to_date"#,
        "2024-03-03|2024-02-28|∅|2024-02-28|∅",
    ),
    (r#"@now:>@2024-01-02T10:30"#, "true|true|true|true|true"),
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
    // A path of keys and positions, which count from the end when negative. A key never indexes
    // an array. Nowhere, and a JSON null, are NULL.
    (r#"j|json_path("name")"#, "x|∅|∅|∅|∅"),
    (r#"j|json_path("n")|to_number|times(2)"#, "6|∅|∅|∅|∅"),
    (r#"j|json_path("flag")"#, "true|∅|∅|∅|∅"),
    (r#"j|json_path("nothing")"#, "∅|∅|∅|∅|∅"),
    (r#"j|json_path("missing")"#, "∅|∅|∅|∅|∅"),
    (r#"j|json_path("items" 0 "sku")"#, "A1|∅|∅|∅|∅"),
    (r#"j|json_path("items" (-1) "sku")"#, "B2|∅|∅|∅|∅"),
    (r#"j|json_path("items" (-3) "sku")"#, "∅|∅|∅|∅|∅"),
    (r#"j|json_path("items" 2 "sku")"#, "∅|∅|∅|∅|∅"),
    (r#"j|json_path(0)"#, "∅|10|∅|∅|∅"),
    (r#"j|json_path(-1 "c")"#, "∅|1|∅|∅|∅"),
    (r#"j|json_path(3 0)"#, "∅|1|∅|∅|∅"),
    (r#"j|json_path("0")"#, "zero|∅|∅|∅|∅"),
    (r#"j|json_path("name" "x")"#, "∅|∅|∅|∅|∅"),
    // A position never indexes a string, number, or boolean (as one does in Postgres's `jsonb`).
    (r#"j|json_path("name" 0)"#, "∅|∅|∅|∅|∅"),
    (r#"j|json_path(-1)"#, "∅|{\"c\":1.0}|∅|∅|∅"),
    (r#"j|json_path('k"q')"#, "1|∅|∅|∅|∅"),
    // A step that isn't a literal: a position when it's a number, and a key otherwise.
    (r#"j|json_path(n)"#, r#"∅|{"c":1.0}|∅|∅|∅"#),
    (r#"j|json_path(n|minus(1))"#, "∅|[1.0]|∅|∅|∅"),
    (r#"j|json_path("na"|concat("me"))"#, "x|∅|∅|∅|∅"),
    (r#"j|json_path('k'|concat('"q'))"#, "1|∅|∅|∅|∅"),
    (r#"j|json_path(@null)"#, "∅|∅|∅|∅|∅"),
    // An object or array is JSON text, which `json_path` reads again.
    (
        r#"j|json_path("nested")"#,
        r#"{"a":[1.0,2.0],"b":"c"}|∅|∅|∅|∅"#,
    ),
    (r#"j|json_path("nested")|json_path("a" 1)"#, "2|∅|∅|∅|∅"),
    (r#"j|json_path("tags")"#, r#"["urgent","ui"]|∅|∅|∅|∅"#),
    (r#"'{"a": [5]}'|to_json|json_path("a" 0)"#, "5|5|5|5|5"),
    // `to_array` gives text, as `json_path` does.
    (
        r#"j|json_path("tags")|to_array|join(",")"#,
        "urgent,ui|∅|∅|∅|∅",
    ),
    (
        r#"j|json_path("tags")|to_array|some(@t => @t:=="urgent")"#,
        "true|∅|∅|∅|∅",
    ),
    (
        r#"j|json_path("items")|to_array|map(@i => @i|json_path("sku"))|join(",")"#,
        "A1,B2|∅|∅|∅|∅",
    ),
    (
        r#"j|json_path("items")|to_array|filter(@i => @i|json_path("price")|to_number:>10)|map(@i => @i|json_path("sku"))|join(",")"#,
        "A1|∅|∅|∅|∅",
    ),
    (r#"j|json_path(3)|to_array|length"#, "∅|1|∅|∅|∅"),
    (
        r#"'[1, "a", null, [2, 3], {"b": true}, 2.50]'|to_array|length"#,
        "6|6|6|6|6",
    ),
    (
        r#"'[1, "a", null, [2, 3], {"b": true}, 2.50]'|to_array|at(0)"#,
        "1|1|1|1|1",
    ),
    (
        r#"'[1, "a", null, [2, 3], {"b": true}, 2.50]'|to_array|at(1)"#,
        "a|a|a|a|a",
    ),
    (
        r#"'[1, "a", null, [2, 3], {"b": true}, 2.50]'|to_array|at(2)"#,
        "∅|∅|∅|∅|∅",
    ),
    (
        r#"'[1, "a", null, [2, 3], {"b": true}, 2.50]'|to_array|at(3)"#,
        "[2.0,3.0]|[2.0,3.0]|[2.0,3.0]|[2.0,3.0]|[2.0,3.0]",
    ),
    (
        r#"'[1, "a", null, [2, 3], {"b": true}, 2.50]'|to_array|at(4)"#,
        r#"{"b":true}|{"b":true}|{"b":true}|{"b":true}|{"b":true}"#,
    ),
    (
        r#"'[1, "a", null, [2, 3], {"b": true}, 2.50]'|to_array|at(5)"#,
        "2.5|2.5|2.5|2.5|2.5",
    ),
    (r#"'[]'|to_array|length"#, "0|0|0|0|0"),
    (r#"'null'|to_array|length"#, "∅|∅|∅|∅|∅"),
    (
        r#"@@json_array(1 "a")|to_array|join(",")"#,
        "1,a|1,a|1,a|1,a|1,a",
    ),
    // The builders make JSON of each value by its type.
    (
        r#"@@json_object("s" s "n" n "b" n:>0 "z" @null)"#,
        r#"{"b":true,"n":90.0,"s":"Fix: one fix","z":null}|{"b":false,"n":-1.0,"s":"a,b,c","z":null}|{"b":null,"n":null,"s":"","z":null}|{"b":false,"n":0.0,"s":null,"z":null}|{"b":null,"n":null,"s":"x\ny","z":null}"#,
    ),
    (
        r#"@@json_array(@2024-01-02 "2024-01-02T10:30:00.5"|to_timestamp "2024-01-02T10:30:00"|to_timestamp)"#,
        r#"["2024-01-02","2024-01-02T10:30:00.5","2024-01-02T10:30:00"]|["2024-01-02","2024-01-02T10:30:00.5","2024-01-02T10:30:00"]|["2024-01-02","2024-01-02T10:30:00.5","2024-01-02T10:30:00"]|["2024-01-02","2024-01-02T10:30:00.5","2024-01-02T10:30:00"]|["2024-01-02","2024-01-02T10:30:00.5","2024-01-02T10:30:00"]"#,
    ),
    (
        r#"@@json_object("a" @@array(1 2) "j" @@json_array(1) "t" '{"x":1}' "tj" '{"x":1}'|to_json)"#,
        r#"{"a":[1.0,2.0],"j":[1.0],"t":"{\"x\":1}","tj":{"x":1.0}}|{"a":[1.0,2.0],"j":[1.0],"t":"{\"x\":1}","tj":{"x":1.0}}|{"a":[1.0,2.0],"j":[1.0],"t":"{\"x\":1}","tj":{"x":1.0}}|{"a":[1.0,2.0],"j":[1.0],"t":"{\"x\":1}","tj":{"x":1.0}}|{"a":[1.0,2.0],"j":[1.0],"t":"{\"x\":1}","tj":{"x":1.0}}"#,
    ),
    (
        r#"@@json_array(j|json_path("n") j|json_path("n")|to_json)"#,
        r#"["3",3.0]|[null,null]|[null,null]|[null,null]|[null,null]"#,
    ),
    // (NaN isn't JSON: Postgres writes it as a string, and DuckDB as `NaN`.)
    (
        r#"@@json_array(? x|is_nan ~ 0 ~~ x)"#,
        "[2.5]|[-2.5]|[1250.0]|[0.0]|[null]",
    ),
    // Anything but an array gives NULL.
    (r#"j|to_array|length"#, "∅|5|∅|∅|∅"),
    (
        r#"@@json_object(1 "one")"#,
        r#"{"1":"one"}|{"1":"one"}|{"1":"one"}|{"1":"one"}|{"1":"one"}"#,
    ),
    (r#"@@json_object()"#, "{}|{}|{}|{}|{}"),
    (r#"@@json_array()"#, "[]|[]|[]|[]|[]"),
];

/// Queries that aggregate, each with the values it should give, as for [`CASES`]. Each query has a
/// column `id`, by which its rows are ordered, and a column `v`, whose values are shown.
///
/// The groups are 1: an odd number of members, 2: an even number, 3: a NULL among them, 4: only
/// NULLs, and 5: no members.
const AGGREGATE_CASES: &[(&str, &str)] = &[
    // The median: the middle value, or between the two middle ones. NULLs are ignored, and there's
    // no median of nothing.
    (
        r"#groups $id $#members.v%percentile(0.5) -> v",
        "2|2.5|5|∅|∅",
    ),
    // Interpolated between the values on either side.
    (
        r"#groups $id $#members.v%percentile(0.25) -> v",
        "1.5|1.75|4.5|∅|∅",
    ),
    (r"#groups $id $#members.v%percentile(0) -> v", "1|1|4|∅|∅"),
    (r"#groups $id $#members.v%percentile(1) -> v", "10|10|6|∅|∅"),
    // Through a constant, of the values that functions compute, and of filtered records.
    (
        "@f = 0.75\n#groups $id $#members.v%percentile(@f) -> v",
        "6|4.75|5.5|∅|∅",
    ),
    (
        r"#groups $id $#members.v|times(2)%percentile(0.5) -> v",
        "4|5|10|∅|∅",
    ),
    (
        r"#groups $id $#members{v:>1}.v%percentile(0.5) -> v",
        "6|3|5|∅|∅",
    ),
    // Integers interpolate as other numbers do, and durations by their seconds.
    (
        r"#groups $id $#members.id%percentile(0.5) -> v",
        "2|5.5|9|11|∅",
    ),
    (
        r"#groups $id $#members.d|seconds%percentile(0.5)|seconds_to_duration|seconds -> v",
        "2|∅|129600|∅|∅",
    ),
    // In a condition.
    (r"#groups #members.v%percentile(0.5):>2 $id $id -> v", "2|3"),
    // In a grouped query, and in a pipeline's later stage.
    (
        r"#members $grp -> id \g $v%percentile(0.5) -> v",
        "2|2.5|5|∅",
    ),
    (
        r"#members $grp $v ~~~ $grp -> id \g $v%percentile(0.5) -> v",
        "2|2.5|5|∅",
    ),
];

#[test]
fn test_functions_postgres() {
    let env = PgEnv::setup("functions", &ddl("jsonb"));
    check(Box::new(Postgres()), |sql| env.run_query(sql));
}

#[test]
fn test_functions_duckdb() {
    let env = DuckEnv::setup(&ddl("JSON"));
    check(Box::new(DuckDB()), |sql| env.run_query(sql));
}

#[test]
fn test_aggregates_postgres() {
    let env = PgEnv::setup("aggregates", &ddl("jsonb"));
    check_aggregates(Box::new(Postgres()), |sql| env.run_query(sql));
}

#[test]
fn test_aggregates_duckdb() {
    let env = DuckEnv::setup(&ddl("JSON"));
    check_aggregates(Box::new(DuckDB()), |sql| env.run_query(sql));
}

/// Compiles each of [`CASES`] for `dialect` and runs it with `run`, which returns a query's lone
/// text value.
fn check(dialect: Box<dyn Dialect>, run: impl Fn(&str) -> String) {
    let cases = CASES
        .iter()
        .map(|(expr, expected)| (format!("#vals $id $({expr}) -> v"), *expected));
    check_queries(dialect, cases, run);
}

/// Like [`check`], for [`AGGREGATE_CASES`].
fn check_aggregates(dialect: Box<dyn Dialect>, run: impl Fn(&str) -> String) {
    let cases = AGGREGATE_CASES
        .iter()
        .map(|(query, expected)| (query.to_string(), *expected));
    check_queries(dialect, cases, run);
}

/// Compiles each query of `cases` for `dialect`, runs it with `run`, and compares the values of its
/// column `v`, in order of its column `id`, with those expected.
fn check_queries<'a>(
    dialect: Box<dyn Dialect>,
    cases: impl Iterator<Item = (String, &'a str)>,
    run: impl Fn(&str) -> String,
) {
    let schema = run(dialect.introspection_sql());
    let options = Options {
        dialect,
        identifier_resolution: IdentifierResolution::Flexible,
        limit: None,
        offset: None,
    };
    let compiler = Compiler::new(&schema, options).expect("introspected schema should compile");
    let failures: Vec<String> = cases
        .filter_map(|(query, expected)| {
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
            (values != expected)
                .then(|| format!("{query}\n  expected [{expected}], found [{values}]\n{sql}"))
        })
        .collect();
    assert!(failures.is_empty(), "\n{}", failures.join("\n\n"));
}

/// Writes each number among `values` as Rust formats an `f64`, since the databases format them
/// differently (`3` and `3.0`, `NaN` and `nan`, `0` and `-0.0`). Each JSON object or array is
/// written compactly, with its keys in order and its numbers as `f64`s, since the databases format
/// JSON differently too: Postgres's `jsonb` puts spaces after `:` and `,`, and orders keys by
/// length, and DuckDB writes a `DOUBLE` of 1250 as `1250.0`.
fn normalize(values: &str) -> String {
    values
        .split('|')
        .map(|value| match value.parse::<f64>() {
            // Without a sign on zero, which rounding a negative number can leave in DuckDB.
            Ok(number) if number == 0.0 => "0".to_string(),
            Ok(number) => number.to_string(),
            Err(_) if value.starts_with(['{', '[']) => {
                match serde_json::from_str::<serde_json::Value>(value) {
                    Ok(json) => canonical_json(json).to_string(),
                    Err(_) => value.to_string(),
                }
            }
            Err(_) => value.to_string(),
        })
        .collect::<Vec<_>>()
        .join("|")
}

/// `json` with each of its numbers as an `f64` (and its keys in order, as `serde_json` keeps them).
fn canonical_json(json: serde_json::Value) -> serde_json::Value {
    use serde_json::Value;
    match json {
        Value::Number(n) => n.as_f64().map(Value::from).unwrap_or(Value::Number(n)),
        Value::Array(elements) => Value::Array(elements.into_iter().map(canonical_json).collect()),
        Value::Object(entries) => Value::Object(
            entries
                .into_iter()
                .map(|(key, value)| (key, canonical_json(value)))
                .collect(),
        ),
        other => other,
    }
}
