# More built-in functions

**Status:** planned. Nothing here is implemented.

This spec adds 22 functions to Querydown and widens one. Each is here because something is
impossible without it. None is here for convenience.

## Design rules

1. **A function must make something possible.** If a query can already be written with the
   functions, operators, and syntax that exist, it gets no function, however long that query is.
   Anyone who wants a shorter name can [define a function](../../docs/language.md#user-defined-functions).
2. **One general function beats several special ones.** `at(0)` and `at(-1)` replace `first` and
   `last`. `percentile(0.5)` replaces `median`.
3. **A function does one kind of thing.** `at` indexes arrays by position. Reading a path into JSON
   is a different thing, so it has its own function.
4. **Functions compose.** A few of them should cover many needs. [What composes](#what-composes)
   shows how.

## Conventions

- **Arguments:** the piped value is the first argument, and the rest go in parentheses. A `?` marks
  an optional argument, and `…` one that can repeat.
- **NULL:** a NULL argument gives NULL.
- **Positions count from 0**, in arrays and in JSON arrays. A negative position counts from the end,
  so `-1` is the last element.
- **Dialects:** a function gives the same result in PostgreSQL and DuckDB.

The PostgreSQL SQL below was run on PostgreSQL 15. The DuckDB SQL wasn't run. It follows the
DuckDB 1.4 documentation, so treat it as a starting point that the `db-tests` corpus will confirm.

## Text

| Function | Description | Compiles to |
| -- | -- | -- |
| `regex_replace(pattern replacement flags?)` | Replaces every match of a regular expression. In the replacement, `\1` is the first capture group. | `regexp_replace(x, p, r, 'g')` |
| `regex_extract(pattern group? flags?)` | The first match of a regular expression, or NULL if there is none. With `group`, the text of that capture group, where 1 is the first. | PostgreSQL: `(regexp_match(x, '(' \|\| p \|\| ')'))[group + 1]`. DuckDB: `regexp_extract(x, p, group)`, guarded by `regexp_matches` to give NULL |
| `split(separator)` | The parts of a string split at `separator`, as an array of text. | `string_to_array` |

The regex functions match as `:~` does: ignoring case, with the same `c`, `m`, and `s`
[flags](../../docs/language.md#regex-flags). The flags are a string, as in
`title|regex_replace('^fix' "Fix" "c")`. `flags` and `group` must be literals. Each dialect writes
the flags into the SQL as its `match_regex` does.

## Math

| Function | Description | Compiles to |
| -- | -- | -- |
| `ln` | The natural logarithm. With `exp` and `pow` already there, it completes logarithms to any base. | `LN` |

## Dates and durations

| Function | Description | Compiles to |
| -- | -- | -- |
| `start_of(unit)` | The start of the `"year"`, `"quarter"`, `"month"`, `"week"`, `"day"`, `"hour"`, or `"minute"` that a date or timestamp falls in. Weeks begin on Monday. The result has the type of the input. | `date_trunc('…', x)`, cast back to `DATE` when the input is a date |
| `seconds_to_duration` | Converts a number of seconds to a duration. The opposite of `seconds`. | PostgreSQL: `make_interval(secs => x)`. DuckDB: `to_seconds(x)` |

`unit` must be a literal, so the compiler can reject an unknown one. `start_of` reads a zoned
timestamp the way `year` does, through `temporal_arg`.

`start_of` is the one new calendar function. Grouping by week, quarter, or hour needs it, and every
calendar part that Querydown lacks follows from it by subtraction (see
[What composes](#what-composes)).

## Conversions

| Function | Description | Compiles to |
| -- | -- | -- |
| `to_number` | Converts text to a number. | `CAST(x AS DOUBLE PRECISION)` |
| `to_date` | Converts ISO 8601 text to a date, or a timestamp to its date. | `CAST(x AS DATE)`, through `temporal_arg` |
| `to_timestamp` | Converts ISO 8601 text to a timestamp without a time zone. | `CAST(x AS TIMESTAMP)` |
| `to_array` | Converts a JSON array to an array of text. An element that is an object or array becomes its JSON text. | PostgreSQL: `ARRAY(SELECT jsonb_array_elements_text(CAST(x AS jsonb)))`. DuckDB: `from_json(x, '["VARCHAR"]')` |
| `to_json` | Reads text as JSON. | PostgreSQL: `CAST(x AS jsonb)`. DuckDB: `CAST(x AS JSON)` |

Text that can't be converted is an error from the database (see [Open questions](#open-questions)).
There is no `to_text`, because a formatted string already converts: `^f"{id}"`.

## Arrays

These work on array columns, on the arrays that `list`, `list_all`, and `top` produce, and on the
results of `split` and `to_array`.

| Function | Description | Compiles to |
| -- | -- | -- |
| `at(index)` | The element at a position, or NULL if there is none. | PostgreSQL: `x[index + 1]`, or `x[cardinality(x) + index + 1]` for a negative index. DuckDB: `x[index + 1]`, or `x[index]` for a negative one |
| `join(separator)` | Joins the elements into one string, skipping NULLs. The opposite of `split`. | `array_to_string` |
| `length` (widened) | Already counts the characters of text. Given an array, it counts the elements. | PostgreSQL: `cardinality`. DuckDB: `len` |

### Functions that take a function

Each of these takes one function, written like an
[anonymous function](../../docs/language.md#anonymous-functions) without its outer parentheses, and
applies it to the elements of an array.

| Function | Description | PostgreSQL | DuckDB |
| -- | -- | -- | -- |
| `map(@e => …)` | An array of the function's result for each element. | `ARRAY(SELECT body FROM unnest(x) WITH ORDINALITY AS u(e, i) ORDER BY i)` | `list_transform(x, lambda e: body)` |
| `filter(@e => …)` | The elements for which the function is true. | `ARRAY(SELECT e FROM unnest(x) WITH ORDINALITY AS u(e, i) WHERE body ORDER BY i)` | `list_filter(x, lambda e: body)` |
| `some(@e => …)` | Whether the function is true for at least one element. False for an empty array. | `EXISTS (SELECT 1 FROM unnest(x) AS u(e) WHERE body)` | `len(list_filter(x, lambda e: body)) > 0` |
| `every(@e => …)` | Whether the function is true for every element. True for an empty array. | `NOT EXISTS (SELECT 1 FROM unnest(x) AS u(e) WHERE (body) IS NOT TRUE)` | `len(list_filter(x, lambda e: (body) IS NOT TRUE)) = 0` |
| `reduce(@acc @e => …)` | Combines the elements into one value, from the left. `@acc` starts as the first element. NULL for an empty array. | A recursive query (below) | `list_reduce(x, lambda acc, e: body)`, guarded for an empty array |

- **Position.** The function given to `map`, `filter`, `some`, or `every` may take a second
  parameter, the element's position: `filter(@e @i => @i:<3)` keeps the first three. Both databases
  count it from 1, so the compiler subtracts 1.
- **Scope.** The body sees the row's columns and the variables around it, as an anonymous function
  does. So `tags|some(@t => @t:==status)` compares each tag with the row's own `status`.
- **Limits.** The body can't use an aggregate of the rows being queried, or a window function.
  DuckDB doesn't allow a subquery in a lambda, so a body that compiles to one is an error there.
- **`reduce` in PostgreSQL** has no direct equivalent, so it compiles to a recursive query that runs
  for each row:

    ```sql
    (WITH RECURSIVE r(i, acc) AS (
       SELECT 1, x[1]
       UNION ALL
       SELECT i + 1, body FROM r WHERE i < cardinality(x)  -- body uses acc and x[i + 1]
     ) SELECT acc FROM r ORDER BY i DESC LIMIT 1)
    ```

    It works, but it's slow on long arrays.

## JSON

| Function | Description | Compiles to |
| -- | -- | -- |
| `json_path(step…)` | The value at a path into JSON. Each step is an object's key or an array's position. A text or number comes back as text without quotes, and an object or array as JSON text. NULL when the path leads nowhere, or to a JSON null. | PostgreSQL: `CAST(x AS jsonb) #>> ARRAY['items', '-1', 'sku']`. DuckDB: `json_extract_string(x, '$."items"[#-1]."sku"')` |
| `json_object(key value …)` | Builds a JSON object from pairs of keys and values. | PostgreSQL: `jsonb_build_object`. DuckDB: `json_object` |
| `json_array(value…)` | Builds a JSON array from values. | PostgreSQL: `jsonb_build_array`. DuckDB: `json_array` |

- **`json_path` reads JSON columns and text that holds JSON**, including what an earlier `json_path`
  returned.
- **A step needn't be a literal.** `translations|json_path(locale)` reads the key named by a column.
  In DuckDB, such a step is concatenated into the path at run time.
- **Values in the builders** become JSON by their type: text a string, a number a number, a boolean
  `true` or `false`, NULL `null`, a date or timestamp an ISO 8601 string, an array a JSON array, and
  JSON itself is nested as it is. Text that holds JSON is nested as a string unless it's piped
  through `to_json` first.
- **`json_object` needs an even number of arguments**, or the compiler reports an error.

```
// users over 30, by a number inside a JSON column
#users profile|json_path("age")|to_number:>30

// events tagged "urgent"
#events payload|json_path("tags")|to_array|some(@t => @t:=="urgent")

// the SKUs of each order's items that cost more than 10
#orders $id $details|json_path("items")|to_array
  |filter(@i => @i|json_path("price")|to_number:>10)
  |map(@i => @i|json_path("sku"))

// each issue with its comments as objects, oldest first
#comments.summary = @@json_object("by" user.username "body" body)
#issues $title $#comments.summary%list_all(\\created_at)
```

(`events` and `orders` aren't in the sample schema, which has no JSON column yet. Add one for the
docs and the corpus.)

### Why the path isn't a richer language

No one path syntax is used everywhere. JSONPath (now RFC 9535) is common in programming languages.
SQL has its own, the SQL/JSON path language, which looks similar but writes filters differently. jq
is a third language, and no database runs it. What the databases support differs a lot:

| | Keys and positions | From the end | Wildcards | Filters |
| -- | -- | -- | -- | -- |
| PostgreSQL | yes | `[last]` | yes | yes |
| MySQL | yes | `[last]` | yes | no |
| DuckDB | yes | `[#-1]` | `[*]` only | no |
| SQLite | yes | `[#-1]` | no | no |

Only keys and positions work everywhere, and even "from the end" is spelled two ways. A richer
path would either work in PostgreSQL alone, or need Querydown to translate it into subqueries for
the other three.

So `json_path` takes the common core, as steps, not as a string in another language. The compiler
writes each database's own path from them. The rest of a path language is already in Querydown: a
wildcard is `map`, a filter is `filter`, and an existence test is `some`. These take any Querydown
expression, which no path language can.

## Aggregates

| Function | Description | Compiles to |
| -- | -- | -- |
| `percentile(fraction)` | The value below which that fraction of a numeric column falls, interpolated. `%percentile(0.5)` is the median. | `percentile_cont(fraction) WITHIN GROUP (ORDER BY x)` |

`percentile` takes no sorting entries, and it isn't a window function, since PostgreSQL can't use
`percentile_cont` over a window.

## What composes

None of these needs a function of its own.

| To get | Write |
| -- | -- |
| The first or last element | `x\|at(0)`, `x\|at(-1)` |
| One part of split text | `email\|split("@")\|at(1)` |
| Whether an array has a value | `tags\|some(@t => @t:=="urgent")` |
| The first three elements | `x\|filter(@e @i => @i:<3)` |
| The sum of an array's numbers | `x\|reduce(@a @b => @a + @b)` |
| The largest element | `x\|reduce(@a @b => @a\|max(@b))` |
| Two arrays, multiplied element by element | `xs\|map(@x @i => @x\|times(ys\|at(@i)))` |
| An array from separate values | `@@json_array(home_phone work_phone)\|to_array` |
| `string_agg` | `#labels.name%list(\s)\|join(", ")` |
| A value from the latest related record | `#comments.body%list_all(\\created_at \d)\|at(0)` |
| Trim one end | `x\|regex_replace('^\s+' "")`, `x\|regex_replace('\s+$' "")` |
| Replace text | `x\|regex_replace('colour' "color")` |
| Starts or ends with | `x:~'^Fix'`, `x:~'\.md$'` |
| The position of text | `x\|regex_extract('^(.*?)needle' 1)\|length` |
| The first or last 3 characters | `x\|regex_extract('^.{0,3}')`, `x\|regex_extract('.{0,3}$')` |
| Pad with zeros to 5 digits | `^f"0000{id}"\|regex_extract('.{5}$')` |
| Weeks or months in a duration | `x\|days\|divide(7)`, `x\|days\|divide(30.44)` |
| Hour of the day | `t\|minus(t\|start_of("day"))\|hours\|floor` |
| Day of the week (0 is Monday) | `t\|minus(t\|start_of("week"))\|days\|floor` |
| Day of the year | `t\|minus(t\|start_of("year"))\|days\|floor\|plus(1)` |
| Quarter | `t\|month\|plus(2)\|divide(3)\|floor` |
| Seconds since 1970 | `t\|minus(@1970-01-01)\|seconds` |
| A timestamp from seconds since 1970 | `@1970-01-01\|plus(n\|seconds_to_duration)` |
| A duration from days | `n\|times(86400)\|seconds_to_duration` |
| Round | `x\|plus(0.5)\|floor` |
| Logarithm to base `b` | `x\|ln\|divide(b\|ln)` |
| `null_if` | `? x:==v ~ @null ~~ x` |
| Convert to text | `^f"{x}"` |
| Median | `x%percentile(0.5)` |
| Standard deviation | `x\|pow(2)%avg\|minus(x%avg\|pow(2))\|sqrt` |
| Count or share of true values | `flag\|(@b => ? @b ~ 1 ~~ 0)%sum`, or `%avg` |
| The most common value | `#comments.user%top(\\%count \d)` |

The calendar rows are written for timestamps. Between two plain dates, `minus` already gives a
number of days.

## Compiler work

Some of these are only a row in `get_standard_scalar_functions`
(`compiler/src/compiler/functions.rs`). The rest need:

- **Functions as arguments.** The parser accepts `@params => body` as the argument of a call, and
  the syntax tree gets a node for it. Only the five functions above accept one.
- **Parameters bound to SQL.** A parameter is bound to a syntax tree today, and inlined. An
  element parameter is bound to a piece of SQL instead: the subquery's column in PostgreSQL, the
  lambda's variable in DuckDB. Type inference gives it the array's element type.
- **Optional and literal arguments.** A helper for functions with optional arguments, and one that
  reads an argument as a literal (`unit`, `flags`, `group`) and reports an error at it otherwise.
- **Choosing SQL by type.** `length` compiles differently for an array, and `start_of` for a date.
  They call `infer_type` on the argument, as `arg_zone` already does for time zones.
- **New `TypeRule`s.** `at` and `reduce` return the element type of an array. `map` returns an
  array of its body's type.
- **`Dialect` methods** for `regex_replace`, `regex_extract`, `seconds_to_duration`, `at`, array
  `length`, the five functions that take a function, and the five JSON functions (`json_path`,
  `json_object`, `json_array`, `to_array`, `to_json`).
- **Arguments to aggregate functions.** The parentheses after `%name` hold only sorting entries
  today (`aggregate_order_by` in `parser/src/parser/expr/pipe.rs`). `percentile` needs the parser to
  accept value arguments there, and `Call.args` to carry them.
- **DuckDB's JSON extension.** The `db-tests` build uses the `duckdb` crate with only its `bundled`
  feature. JSON needs its `json` feature too, which means rebuilding DuckDB. Applications need the
  extension loaded, which official DuckDB builds do by themselves.

For each function: add it to the registry and to `compiler/src/compiler/completion/catalog.rs`, add
corpus cases that pass `cargo test --features db-tests`, document it in `docs/functions.md`, and add
it to the release notes.

## Order of work

1. **Same SQL in both dialects:** `ln`, `split`, `join`, `to_number`, `to_date`, `to_timestamp`,
   `start_of`.
2. **Dialect-specific or chosen by type:** `regex_replace`, `regex_extract`,
   `seconds_to_duration`, `at`, `length` for arrays.
3. **Functions that take a function:** `map`, `filter`, `some`, `every`, then `reduce`.
4. **JSON:** `json_path`, `to_array`, `to_json`, `json_object`, `json_array`, and a JSON column in
   the sample schema.
5. **`percentile`**, after the parser change.

## Other databases later

MySQL and SQLite aren't supported today. This design ports to them, with one doubt. (Their side of
this wasn't run. It's from their documentation as remembered.)

- **`json_path`:** both read a path of keys and positions, and both can count from the end, so the
  steps translate directly. A richer path wouldn't: SQLite has no wildcards, and neither has filters.
- **`json_object`, `json_array`, `to_json`:** both have equivalents.
- **Arrays:** neither has an array type. There, an array would be a JSON array, including what
  `list` produces, and `to_array` would change nothing.
- **`map`, `filter`, `some`, `every`:** neither has lambdas, and neither does PostgreSQL. They'd
  compile as in PostgreSQL, to a subquery over the elements (`JSON_TABLE` in MySQL, `json_each` in
  SQLite).
- **`reduce`** is the doubt. It needs a recursive query that refers to the outer row, which those
  databases may not allow.

## Left out

Nothing that exists can do these. They wait until someone needs them:

- **Trigonometry.** `sin`, `cos`, and `atan2` would cover all of it.
- **`random`.** It takes no arguments, and nothing calls a function without arguments yet.
  `unit_hash` already gives a stable shuffle or sample.
- **Time zone conversion.** DuckDB can't do it without the ICU extension.
- **Formatting a date as text** in an arbitrary pattern. PostgreSQL's `to_char` and DuckDB's
  `strftime` take different patterns, so Querydown would need a pattern language of its own.
- **More of JSON:** an object's keys, and an object built from many rows.
- **Passing a named function** where a function is expected, as in `map(@@uppercase)`.

## Open questions

1. **Should `round` be an exception to rule 1?** It's left out because `floor` can do it. But the
   workaround is clumsy for decimal places (`x|times(100)|plus(0.5)|floor|divide(100)`), and
   rounding is among the most common needs.
2. **Should `json_path` take steps, or one path string handed to the database?** A string, as in
   `json_path('$.items[0].sku')`, looks familiar and would give PostgreSQL users filters and
   wildcards. But anything past keys and positions would behave differently in each database, as
   regexes already do. Recommended: steps.
3. **Should `join` stay?** `reduce` can do it: `x|reduce(@a @b => ^f"{@a}, {@b}")`. Recommended:
   keep it. It's the opposite of `split`, and `reduce` is slow in PostgreSQL.
4. **Should `length` count an array's elements, or should arrays get `size`?** Recommended:
   `length`. It's the same question asked of text and of arrays.
5. **What should a conversion do with text it can't convert?** DuckDB can give NULL (`TRY_CAST`),
   but PostgreSQL 15 can only raise an error. Recommended: an error in both, so the dialects agree.
