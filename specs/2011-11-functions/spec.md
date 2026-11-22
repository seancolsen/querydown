# More built-in functions

**Status:** planned. Nothing here is implemented.

This spec adds 15 functions to Querydown and widens one. Each is here because something is
impossible without it. None is here for convenience.

## Design rules

1. **A function must make something possible.** If a query can already be written with the
   functions, operators, and syntax that exist, it gets no function, however long that query is.
   Anyone who wants a shorter name can [define a function](../../docs/language.md#user-defined-functions).
2. **One general function beats several special ones.** `at(0)` and `at(-1)` replace `first` and
   `last`. `percentile(0.5)` replaces `median`.
3. **Functions compose.** Each takes and returns plain values (text, numbers, dates, durations,
   arrays), so that a few of them cover many needs. [What composes](#what-composes) shows how.

## Conventions

- **Arguments:** the piped value is the first argument, and the rest go in parentheses. A `?` marks
  an optional argument, and `…` one that can repeat.
- **NULL:** a NULL argument gives NULL.
- **Positions count from 0**, in arrays and in JSON arrays. A negative position counts from the end,
  so `-1` is the last element.
- **Dialects:** a function gives the same result in PostgreSQL and DuckDB.

The PostgreSQL SQL below was run on PostgreSQL 15. The DuckDB SQL wasn't run, so treat it as a
starting point. The `db-tests` corpus will confirm it.

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
| `to_duration` | Converts a number of seconds to a duration. The opposite of `seconds`. | PostgreSQL: `make_interval(secs => x)`. DuckDB: `to_seconds(x)` |

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

Text that can't be converted is an error from the database (see [Open questions](#open-questions)).
There is no `to_text`, because a formatted string already converts: `^f"{id}"`.

## Arrays

These work on array columns, on the arrays that `list`, `list_all`, and `top` produce, and on the
results of `split` and `to_array`.

| Function | Description | Compiles to |
| -- | -- | -- |
| `at(index)` | The element at a position, or NULL if there is none. | PostgreSQL: `x[index + 1]`, or `x[cardinality(x) + index + 1]` for a negative index. DuckDB: `x[index + 1]`, or `x[index]` for a negative one |
| `slice(start end?)` | The elements from `start` up to, but not including, `end`. Without `end`, the rest of the array. | `x[a:b]`, with the bounds converted as for `at` |
| `join(separator)` | Joins the elements into one string, skipping NULLs. The opposite of `split`. | `array_to_string` |
| `has(value)` | Whether the array contains `value`. | `value = ANY(x)` |
| `length` (widened) | Already counts the characters of text. Given an array, it counts the elements. | PostgreSQL: `cardinality`. DuckDB: `len` |

## JSON

JSON needs no functions of its own. `at` reads into it, and its values come out as text, for the
conversions above to turn into numbers, dates, and arrays.

| Function | Description | Compiles to |
| -- | -- | -- |
| `at(key…)` | The value at a path into JSON. Each argument is an object's key or an array's position. A text or number comes back as text without quotes, and an object or array as JSON text. NULL when the path leads nowhere, or to a JSON null. | PostgreSQL: `CAST(x AS jsonb) #>> ARRAY['a', '0']`. DuckDB: `json_extract_string(x, '$."a"[0]')` |

- `at` reads JSON unless its input is an array, as inferred by the compiler. So it works on `json`
  and `jsonb` columns, and on text that holds JSON, including what an earlier `at` returned.
- A key needn't be a literal: `translations|at(locale)` reads the key named by a column. In DuckDB,
  such a key is concatenated into the path at run time.

```
// users over 30, by a number inside a JSON column
#users profile|at("age")|to_number:>30

// events tagged "urgent"
#events payload|at("tags")|to_array|has("urgent")

// the SKU of each order's last item
#orders $id $details|at("items" -1 "sku")
```

(These tables aren't in the sample schema, which has no JSON column yet. Add one for the docs and the
corpus.)

## Aggregates

| Function | Description | Compiles to |
| -- | -- | -- |
| `percentile(fraction)` | The value below which that fraction of a numeric column falls, interpolated. `%percentile(0.5)` is the median. | `percentile_cont(fraction) WITHIN GROUP (ORDER BY x)` |

`percentile` takes no sorting entries, and it isn't a window function, since PostgreSQL can't use
`percentile_cont` over a window.

## What composes

Everything below was in the earlier draft of this spec as a function of its own, or is a common
request. None needs one.

| To get | Write |
| -- | -- |
| The first or last element | `x\|at(0)`, `x\|at(-1)` |
| One part of split text | `email\|split("@")\|at(1)` |
| `string_agg` | `#labels.name%list(\s)\|join(", ")` |
| A value from the latest related record | `#comments.body%list_all(\\created_at \d)\|at(0)` |
| The latest three | `#comments.body%list_all(\\created_at \d)\|slice(0 3)` |
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
| A timestamp from seconds since 1970 | `@1970-01-01\|plus(n\|to_duration)` |
| A duration from days | `n\|times(86400)\|to_duration` |
| Round | `x\|plus(0.5)\|floor` |
| Logarithm to base `b` | `x\|ln\|divide(b\|ln)` |
| `null_if` | `? x:==v ~ @null ~~ x` |
| Convert to text | `^f"{x}"` |
| Median | `x%percentile(0.5)` |
| Standard deviation | `x\|pow(2)%avg\|minus(x%avg\|pow(2))\|sqrt` |
| Count or share of true values | `flag\|(@b => ? @b ~ 1 ~~ 0)%sum`, or `%avg` |
| The most common value | `#comments.user%top(\\%count \d)` |
| The number of elements | `x\|length` |

The calendar rows are written for timestamps. Between two plain dates, `minus` already gives a
number of days.

## Compiler work

Some of these are only a row in `get_standard_scalar_functions`
(`compiler/src/compiler/functions.rs`). The rest need:

- **Optional and literal arguments.** A helper for functions with optional arguments, and one that
  reads an argument as a literal (`unit`, `flags`, `group`) and reports an error at it otherwise.
- **Choosing SQL by type.** `at` and `length` compile differently for an array, and `start_of` for
  a date. They call `infer_type` on the argument, as `arg_zone` already does for time zones.
- **New `TypeRule`s.** `at` returns the element type of an array, and text otherwise.
- **`Dialect` methods** for `regex_replace`, `regex_extract`, `to_duration`, `to_array`, array `at`,
  `slice`, array `length`, and JSON `at`.
- **Arguments to aggregate functions.** The parentheses after `%name` hold only sorting entries
  today (`aggregate_order_by` in `parser/src/parser/expr/pipe.rs`). `percentile` needs the parser to
  accept value arguments there, and `Call.args` to carry them. This is the only parser change.
- **DuckDB's JSON extension.** The `db-tests` build uses the `duckdb` crate with only its `bundled`
  feature. JSON needs its `json` feature too, which means rebuilding DuckDB. Applications need the
  extension loaded, which official DuckDB builds do by themselves.

For each function: add it to the registry and to `compiler/src/compiler/completion/catalog.rs`, add
corpus cases that pass `cargo test --features db-tests`, document it in `docs/functions.md`, and add
it to the release notes.

## Order of work

1. **Same SQL in both dialects:** `ln`, `split`, `join`, `has`, `to_number`, `to_date`,
   `to_timestamp`, `start_of`.
2. **Dialect-specific or chosen by type:** `regex_replace`, `regex_extract`, `to_duration`, array
   `at`, `slice`, `length` for arrays.
3. **JSON:** `at` for JSON, `to_array`, and a JSON column in the sample schema.
4. **`percentile`**, after the parser change.

## Left out

Nothing that exists can do these. They wait until someone needs them:

- **Trigonometry.** `sin`, `cos`, and `atan2` would cover all of it.
- **`random`.** It takes no arguments, and nothing calls a function without arguments yet.
  `unit_hash` already gives a stable shuffle or sample.
- **Time zone conversion.** DuckDB can't do it without the ICU extension.
- **Formatting a date as text** in an arbitrary pattern. PostgreSQL's `to_char` and DuckDB's
  `strftime` take different patterns, so Querydown would need a pattern language of its own.
- **More of JSON:** an object's keys, and reading the same key from every element of an array.

## Open questions

1. **Should `round` be an exception to rule 1?** It's left out because `floor` can do it. But the
   workaround is clumsy for decimal places (`x|times(100)|plus(0.5)|floor|divide(100)`), and
   rounding is among the most common needs.
2. **Should `at` read JSON, or should JSON get its own function?** One function for every kind of
   indexing is simpler to learn. The cost: `at` on plain text is a database error rather than a
   compile error, because text may hold JSON. Recommended: one function.
3. **Should `length` count an array's elements, or should arrays get `size`?** Recommended:
   `length`. The compiler always knows when a value is an array, which `at` relies on too.
4. **What should a conversion do with text it can't convert?** DuckDB can give NULL (`TRY_CAST`),
   but PostgreSQL 15 can only raise an error. Recommended: an error in both, so the dialects agree.
5. **Is `to_duration` the right name?** It only takes seconds, so `from_seconds` would be more
   literal. Recommended: `to_duration`, to sit with the other conversions.
