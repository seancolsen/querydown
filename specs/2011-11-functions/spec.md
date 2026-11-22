# More built-in functions

**Status:** planned. Nothing here is implemented.

Querydown has 38 scalar functions, 12 aggregate functions, and 11 window functions
(see [docs/functions.md](../../docs/functions.md)). Common needs such as rounding a number,
replacing text, grouping by week, or joining a list into a string have no function yet. This spec
lists the functions to add, 54 in all.

## Conventions

Every function below follows these, unless its row says otherwise.

- **Names** are whole words in snake_case, like the existing `lowercase` and `if_null`.
- **Arguments:** the piped value is the first argument, and the rest go in parentheses. A `?` marks
  an optional argument.
- **NULL:** a NULL argument gives NULL.
- **Positions** in text and arrays count from 1.
- **Dialects:** the function gives the same result in PostgreSQL and DuckDB. Where one SQL
  expression can't do that, the "Compiles to" column names both, and the function gets a method on
  `Dialect`.
- **Text is compared exactly** (case-sensitively), as SQL does. The exceptions are `starts_with`,
  `ends_with`, and the regex functions, which match the way the `:` and `:~` operators do.

The PostgreSQL column was checked against PostgreSQL 15. The DuckDB SQL hasn't been run yet. The
`db-tests` corpus will confirm it during implementation.

The **Phase** column says when to build each function. See [Order of work](#order-of-work).

## Text functions

| Function | Description | Compiles to | Phase |
| -- | -- | -- | -- |
| `trim_start`, `trim_end` | Removes whitespace from one end of a string. | `ltrim`, `rtrim` | 1 |
| `replace(from to)` | Replaces every occurrence of `from` with `to`. | `replace` | 1 |
| `slice(start length?)` | The part of a string that begins at character `start`. It's `length` characters long, or runs to the end. | `substr` | 1 |
| `left(n)`, `right(n)` | The first or last `n` characters. | `left`, `right` | 1 |
| `position(text)` | Where `text` first occurs in the string, or 0 if it doesn't. | `strpos` | 1 |
| `split_part(separator n)` | The `n`th part of a string split at `separator`, as in `email\|split_part("@" 2)`. An empty string when there is no `n`th part. | `split_part` | 1 |
| `split(separator)` | The parts of a string split at `separator`, as an array. | `string_to_array` | 1 |
| `pad_start(length fill?)`, `pad_end(length fill?)` | Pads a value to `length` characters with `fill` (a space by default), as in `id\|pad_start(5 "0")`. Longer values are cut to `length`. | `lpad`, `rpad` of `CAST(x AS TEXT)`. PostgreSQL rejects a number here without the cast. | 1 |
| `starts_with(text)`, `ends_with(text)` | Whether the string begins or ends with `text`, ignoring case like `:`. | `Dialect` method that reuses the normalization of `text_eq` | 2 |
| `regex_replace(pattern replacement)` | Replaces every match of a regular expression. `\1` in the replacement is the first capture group. | `regexp_replace(x, p, r, 'g')` | 2 |
| `regex_extract(pattern)` | The first match of a regular expression, or NULL if there is none. | PostgreSQL: `(regexp_match(x, '(' \|\| p \|\| ')'))[1]`. DuckDB: `NULLIF(regexp_extract(x, p), '')` | 2 |

The regex functions ignore case by default, as `:~` does, and take no flags. They prefix the pattern
with `Dialect::prefixed_regex_pattern`, as `match_regex` does.

## Math functions

| Function | Description | Compiles to | Phase |
| -- | -- | -- | -- |
| `round(digits?)` | Rounds to the nearest integer, or to `digits` decimal places. | `ROUND(x)`, or `ROUND(CAST(x AS numeric), digits)`. PostgreSQL has no two-argument `ROUND` for floats. | 1 |
| `sign` | -1, 0, or 1, by the sign of a number. | `SIGN` | 1 |
| `ln` | The natural logarithm. | `LN` | 1 |
| `log(base?)` | The logarithm to base 10, or to `base`. | `LOG10(x)`, or `LN(x) / LN(base)` | 1 |
| `percent_of(total)` | The value as a percentage of `total`, as in `#issues{status:closed}\|percent_of(#issues)`. NULL when `total` is 0. | `x * 100.0 / NULLIF(total, 0)` | 1 |

`percent_of` exists because plain division trips people up: `/` between two integers drops the
remainder in PostgreSQL, and dividing by zero is an error.

## Date and time functions

| Function | Description | Compiles to | Phase |
| -- | -- | -- | -- |
| `hour`, `minute`, `second` | The hour (0–23), minute (0–59), or whole second (0–59) of a timestamp. | `calendar_field`, as `year` does. `second` needs `FLOOR`, because `CAST(59.9 AS INTEGER)` is 60. | 1 |
| `quarter` | The quarter of the year (1–4). | `EXTRACT(quarter …)` | 1 |
| `week` | The ISO week of the year (1–53). | `EXTRACT(week …)` | 1 |
| `weekday` | The ISO day of the week: 1 for Monday to 7 for Sunday. | `EXTRACT(isodow …)` | 1 |
| `day_of_year` | The day of the year (1–366). | `EXTRACT(doy …)` | 1 |
| `weeks`, `months` | Converts an interval into a number of weeks or months, like `days` and `years`. A month is a twelfth of a 365.25-day year. | `EXTRACT(epoch FROM x) / 604800`, `/ 2629800` | 1 |
| `to_epoch` | The number of seconds from 1970-01-01 UTC to a timestamp. | `EXTRACT(epoch FROM x)` | 1 |
| `start_of_week`, `start_of_month`, `start_of_quarter`, `start_of_year` | The first day of the period that a date or timestamp falls in, as a date. Weeks begin on Monday. | `CAST(date_trunc('…', x) AS DATE)` | 1 |
| `start_of_hour` | A timestamp with its minutes and seconds set to zero. | `date_trunc('hour', x)` | 1 |
| `from_epoch` | The timestamp that many seconds after 1970-01-01 UTC. For columns that store Unix time. | `to_timestamp(x)`, made naive where `coerces_timestamptz` | 2 |
| `weekday_name`, `month_name` | The English name of the day or month, such as "Monday" or "March". | PostgreSQL: `to_char(x, 'FMDay')`, `'FMMonth'`. DuckDB: `dayname`, `monthname` | 2 |

All of these read a zoned timestamp the way `year` does, through `temporal_arg`. The `start_of_…`
functions suit grouping better than `year_month`, since their results are dates, which sort and
compare as dates.

## Conversion functions

| Function | Description | Compiles to | Phase |
| -- | -- | -- | -- |
| `null_if(value)` | NULL when the piped value equals `value`, and the piped value otherwise. The opposite of `if_null`. | `NULLIF` | 1 |
| `to_text` | Converts any value to text. | `CAST(x AS TEXT)` | 1 |
| `to_number` | Converts text or a boolean to a number. | `CAST(x AS DOUBLE PRECISION)` | 1 |
| `to_date` | Converts a timestamp to its date, dropping the time, or converts ISO 8601 text to a date. | `CAST(x AS DATE)`, through `temporal_arg` | 1 |

Text that can't be converted is an error from the database (see [open questions](#open-questions)).

## Array functions

These are scalar functions for array columns and for the arrays that `list`, `list_all`, and `top`
produce.

| Function | Description | Compiles to | Phase |
| -- | -- | -- | -- |
| `join(separator)` | Joins the elements of an array into one string, skipping NULLs. | `array_to_string` | 1 |
| `has(value)` | Whether an array contains `value`. | `value = ANY(x)` | 1 |
| `size` | The number of elements in an array. | PostgreSQL: `cardinality`. DuckDB: `len` | 2 |
| `first`, `last` | The first or last element of an array, or NULL if it's empty. | `x[1]`. Last: `x[cardinality(x)]` in PostgreSQL, `x[-1]` in DuckDB | 2 |

They make two common requests possible without new aggregate functions:

```
// each issue's labels as one string, like SQL's string_agg
#issues $title $#labels.name%list(\s)|join(", ")

// the body of each issue's latest comment
#issues $title $#comments.body%list_all(\\created_at \d)|first
```

## Aggregate functions

| Function | Description | Compiles to | Phase |
| -- | -- | -- | -- |
| `median` | The middle value of a numeric column. | `percentile_cont(0.5) WITHIN GROUP (ORDER BY x)` | 1 |
| `stddev`, `variance` | The sample standard deviation and variance of a numeric column. | `stddev_samp`, `var_samp` | 1 |
| `mode` | The most common value of a column. When several tie, one of them. | `mode() WITHIN GROUP (ORDER BY x)` | 1 |
| `count_true` | The number of true values in a boolean column. Zero when there are no rows, like `count`. | `count(*) FILTER (WHERE x)` | 1 |
| `fraction_true` | The share of a boolean column's non-null values that are true, from 0 to 1. | `avg(CAST(x AS INTEGER))` | 1 |
| `percentile(p)` | The value below which the fraction `p` of a numeric column falls, as in `%percentile(0.9)`. | `percentile_cont(p) WITHIN GROUP (ORDER BY x)` | 3 |

`count_true` and `fraction_true` matter most in grouped queries, where a `{…}` filter isn't
available. With a computed column `#issues.overdue = due_date:<@now`, the query
`#issues $project \g $overdue%fraction_true` gives each project's share of overdue issues.

None of these take sorting entries. None become window functions in this round, since PostgreSQL
can't use `percentile_cont` or `mode` over a window.

## Compiler work beyond a registry row

Most functions need only a row in `get_standard_scalar_functions` or
`get_standard_aggregate_functions` in `compiler/src/compiler/functions.rs`. These need more:

- **Optional arguments.** `round`, `slice`, `log`, `pad_start`, and `pad_end` take one argument or
  two. The helpers `args_1`, `args_2`, and `args_v` don't cover that, so add one that does.
- **`Dialect` methods** for `starts_with`, `ends_with`, `regex_replace`, `regex_extract`,
  `weekday_name`, `month_name`, `size`, and `last`. `from_epoch` uses the existing
  `coerce_temporal_to_naive`.
- **A new `TypeRule`** for `first` and `last`: the element type of an array argument. `split`
  returns `Array(Text)`, which `Fixed` covers.
- **Arguments to aggregate functions.** The parentheses after `%name` hold only sorting entries
  today (`aggregate_order_by` in `parser/src/parser/expr/pipe.rs`). `percentile(p)` needs the
  parser to accept value arguments there as well, and `Call.args` to carry them. This is the only
  change to the parser.

## For each function

1. Add it to the registry, with its `TypeRule`.
2. Add its entry to `compiler/src/compiler/completion/catalog.rs`. Tests keep the two in step.
3. Add corpus cases to `compiler/src/tests/corpus.md`, and pass `cargo test --features db-tests`
   for both databases.
4. Document it in `docs/functions.md` with a `qd` example. Add the common ones to
   `docs/cheat-sheet.md`.
5. Add it to the release notes under "What's new".

## Order of work

- **Phase 1** (43 functions): the same SQL in both dialects, so each is a registry row plus
  the optional-arguments helper.
- **Phase 2** (10 functions): those whose SQL differs between the dialects, or that need the new
  `TypeRule`.
- **Phase 3** (1 function): `percentile`, after the parser change.

## Not planned

These already have syntax, so they get no function:

| You might look for | Use instead |
| -- | -- |
| `if`, `case` | the case expression `? a ~ b ~~ c` |
| `between` | a range, `x:1..5` |
| `in` | comparison expansion, `status:[open reopened]` |
| `contains`, `matches` | the `:` and `:~` operators |
| `is_null` | `x:@null` |
| `format` (text) | a formatted string, `^f"{a} {b}"` |
| `string_agg` | `%list` and then `join` |
| aggregate `first`, `last` | `%list_all( … )` and then `first` |

Deferred until someone asks, or until what they depend on exists:

- **`format_date(pattern)`.** PostgreSQL's `to_char` and DuckDB's `strftime` take different
  patterns, so Querydown would need a pattern language of its own.
- **Calendar differences**, such as the whole months between two dates. `months` is an
  approximation.
- **`random`.** It takes no arguments, and nothing calls a function without arguments yet.
- **Time zone conversion.** DuckDB can't do it without the ICU extension.
- **Trigonometry, bitwise operations, and JSON access.**

## Open questions

1. **Should `starts_with` and `ends_with` ignore case?** Recommended: yes, since `:` and `:=` do.
   Anyone who needs an exact match has `:\c~'^…'`.
2. **What should `to_number` and `to_date` do with text they can't convert?** DuckDB can give NULL
   (`TRY_CAST`), but PostgreSQL 15 can only raise an error. Recommended: an error in both, so the
   dialects agree.
3. **How long is a month for `months`?** A twelfth of a year (30.4375 days) makes
   `x|months` equal `x|years|times(12)`. But `EXTRACT(epoch …)` counts an interval of one month as
   30 days, so `1m|months` would be 0.99. Recommended: a twelfth of a year, since `months` is mostly
   applied to `age`, which has no month part.
4. **Should `length` count an array's elements, in place of a separate `size`?** Recommended: no.
   The compiler would have to choose by the argument's inferred type, which is sometimes unknown.
