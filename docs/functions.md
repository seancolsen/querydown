# Functions

This page lists every function built into Querydown, with an example of each.

Functions come in three kinds:

- **Scalar** functions work on the values of a single row. You apply them with the [pipe](./language.md#function-piping) `|`.
- **[Aggregate](#aggregate-functions)** functions combine values from many rows. You apply them with `%`.
- **[Window](#window-functions)** functions compute a value for each row from the rows around it. You apply them with `%` to a window, which you define with `%%( )`.

> Show each issue's status and title together, with "unknown" in place of a missing status

```qd
#issues $status|if_null("unknown")|concat(": " title)
```

- The value to the **left** of the pipe is the function's **first argument**.
- Any **more** arguments go in parentheses after the function's name, separated by **spaces** (not commas).
- A function that needs no more arguments takes no parentheses.

You can also [call a function without a pipe](./language.md#function-calling), and [define functions](./language.md#user-defined-functions) of your own. The list of built-in functions is still short, and more will be added.

The examples use the [sample issue-tracker schema](./sample-schema.md).

## General functions

### `if_null`

Returns the first non-null value among its arguments, compiling to SQL `COALESCE`. Accepts any number of arguments: the piped value, followed by any fallbacks in parentheses.

```qd
#issues $status|if_null("unknown")
```

## Boolean functions

### `not`

Negates a boolean value, just like the [`!` operator](./language.md#negation): true unless the value is true, so a NULL value gives `true`. Compiles to SQL `IS NOT TRUE`.

```qd
#projects $name $is_active|not -> inactive
```

### `and`

Logical conjunction across its arguments, compiling to SQL `AND`.

```qd
#issues $title $(status:="open")|and(due_date:<@now) -> open_and_overdue
```

### `or`

Logical disjunction across its arguments, compiling to SQL `OR`.

```qd
#issues $title $(status:="open")|or(status:="reopened") -> needs_attention
```

### `xor`

Exclusive-or across its arguments — true when an odd number of them are true. Compiles to a chain of `<>` (inequality) comparisons.

```qd
#projects $name $is_active|xor(#issues:0) -> active_or_empty_but_not_both
```

## Text functions

### `length`

Returns the number of characters in a string, compiling to SQL `char_length`.

```qd
#issues $title $description|length
```

### `lowercase`

Converts a string to lower case, compiling to SQL `lower`.

```qd
#users $username|lowercase
```

### `uppercase`

Converts a string to upper case, compiling to SQL `upper`.

```qd
#users $username|uppercase
```

### `concat`

Joins multiple strings together into one, compiling to SQL `concat`.

```qd
#issues $status|concat(": ")|concat(title) -> label
```

### `trim`

Removes leading and trailing whitespace from a string, compiling to SQL `trim`.

```qd
#issues $title|trim
```

### `md5`

Computes the MD5 hash of a string, compiling to SQL `md5`.

```qd
#users $email|md5 -> email_hash
```

## Math functions

### `abs`

Returns the absolute value of a number, compiling to SQL `ABS`.

```qd
#issues $title $due_date|age|days|abs
```

### `ceil`

Rounds a number up to the nearest integer, compiling to SQL `CEIL`.

```qd
#issues $title $due_date|countdown|days|ceil
```

### `floor`

Rounds a number down to the nearest integer, compiling to SQL `FLOOR`.

```qd
#issues $title $due_date|countdown|days|floor
```

### `plus`

Adds two numbers. Equivalent to the `+` operator, but usable in a pipeline.

```qd
#issues $title $due_date|countdown|days|plus(7)
```

### `minus`

Subtracts the argument from the piped value. Equivalent to the `-` operator, but usable in a pipeline.

```qd
#issues $title $due_date|countdown|days|minus(7)
```

### `times`

Multiplies two numbers. Equivalent to the `*` operator, but usable in a pipeline.

```qd
#issues $title $due_date|age|days|times(24) -> hours_old
```

### `divide`

Divides the piped value by the argument. Equivalent to the `/` operator, but usable in a pipeline.

```qd
#issues $title $due_date|age|days|divide(7) -> weeks_old
```

### `mod`

Returns the remainder after dividing the piped value by the argument, compiling to SQL `%`.

```qd
#issues id|mod(2):0 $title // even-numbered issues
```

### `keep_above`

Raises the value to a lower bound, returning the greater of the piped value and the argument. Compiles to SQL `GREATEST`.

```qd
#issues $title $due_date|countdown|days|keep_above(0) // never below 0
```

### `keep_below`

Caps the value at an upper bound, returning the lesser of the piped value and the argument. Compiles to SQL `LEAST`.

```qd
#issues $title $due_date|countdown|days|keep_below(30) // never above 30
```

### `max`

Returns the greatest of all its arguments (the piped value plus any in parentheses), compiling to SQL `GREATEST`. This is the scalar function, which compares values within a row. To find the greatest value across rows, use the [aggregate `max`](#max-aggregate).

```qd
#issues $title $due_date|countdown|days|max(0)
```

### `min`

Returns the least of all its arguments (the piped value plus any in parentheses), compiling to SQL `LEAST`. This is the scalar function, which compares values within a row. To find the least value across rows, use the [aggregate `min`](#min-aggregate).

```qd
#issues $title $due_date|countdown|days|min(0)
```

### `pow`

Raises the piped value to the power of the argument, compiling to SQL `POWER`.

```qd
#issues $id|pow(2)
```

### `exp`

Raises `e` to the power of the piped value, compiling to SQL `EXP`.

```qd
#issues $id|exp
```

### `sqrt`

Returns the square root of a number, compiling to SQL `SQRT`.

```qd
#issues $id|sqrt
```

### `unit_hash`

Hashes a value deterministically to a floating-point number between 0 and 1. Useful for sampling or bucketing rows in a stable, repeatable way. Compiles to a dialect-specific normalization of `hash` (DuckDB) or `hashtext` (Postgres).

```qd
#users $email|unit_hash
```

## Date and time functions

### `age`

Returns the interval elapsed _since_ a past timestamp, compiling to `NOW() - value`. Typically applied to a date column and chained with [`days`](#days), [`hours`](#hours), etc.

```qd
#issues $title $created_at|age|days
```

### `countdown`

Returns the interval from now _until_ a future timestamp, compiling to `value - NOW()`. The counterpart to [`age`](#age): for a future date it is positive, and for a past date it is negative. Typically applied to a date column and chained with [`days`](#days), [`hours`](#hours), etc.

```qd
#issues $title $due_date|countdown|days // days until each issue is due
```

### `year`

Returns the year of a date or timestamp as an integer, compiling to `CAST(EXTRACT(year FROM value) AS INTEGER)`. Not to be confused with [`years`](#years), which converts an _interval_ into a number of years.

```qd
#issues created_at|year:2023 $title // issues created in 2023
```

### `month`

Returns the month (1–12) of a date or timestamp as an integer, compiling to `CAST(EXTRACT(month FROM value) AS INTEGER)`.

```qd
#issues $title $due_date|month
```

### `day`

Returns the day of the month (1–31) of a date or timestamp as an integer, compiling to `CAST(EXTRACT(day FROM value) AS INTEGER)`. Not to be confused with [`days`](#days), which converts an _interval_ into a number of days.

```qd
#users $username $birth_date|day
```

### `year_month`

Returns the year and month of a date or timestamp as a `YYYY-MM` string (e.g. `"2023-03"`). Handy for grouping by month. Compiles to `to_char(value, 'YYYY-MM')` (Postgres) or `strftime(value, '%Y-%m')` (DuckDB).

```qd
#issues $created_at|year_month \g $%count // issues created per month
```

For the components above, a zoned timestamp (`timestamptz`) is read in the session time zone on Postgres. On DuckDB, which can't operate on zoned timestamps without the ICU extension, it is first cast to a naive timestamp, so the components reflect its UTC value.

### `years`

Converts an interval into a number of years, compiling to `EXTRACT(epoch FROM value) / 31557600`.
A year is treated as 365.25 days.

```qd
#users $username $birth_date|age|years|floor
```

### `days`

Converts an interval into a number of days, compiling to `EXTRACT(epoch FROM value) / 86400`.

```qd
#issues $title $created_at|age|days
```

### `hours`

Converts an interval into a number of hours, compiling to `EXTRACT(epoch FROM value) / 3600`.

```qd
#comments $body $created_at|age|hours
```

### `minutes`

Converts an interval into a number of minutes, compiling to `EXTRACT(epoch FROM value) / 60`.

```qd
#comments $body $created_at|age|minutes
```

### `seconds`

Converts an interval into a number of seconds, compiling to `EXTRACT(epoch FROM value)`.

```qd
#comments $body $created_at|age|seconds
```

## Aggregate functions

Aggregate functions combine values from many rows into one, and are applied with `%`. The rows are either the [related records](./language.md#specifying-an-aggregate-function) reached by a `#table` path, or the rows of each group in a [grouped query](./language.md#grouping-and-aggregation).

> For each project, show the number of issues and the date of the most recent one

```qd
#projects $name $#issues $#issues.created_at%max
```

An aggregate function is applied to a **column**, or to a [value computed from a column](./language.md#aggregating-a-computed-value) by piping it through scalar functions first, e.g. `#comments.body|length%avg`. It can't be applied to any other kind of expression, such as `(#issues.id * 2)%sum`.

Where the descriptions below say "the values of a column", that includes such computed values.

### `count`

Counts rows. Used alone as `%count`, it compiles to `count(*)`; applied to a column, it counts non-null values.

```qd
#issues $status \g $%count
```

### `distinct`

Counts the distinct values of a column, compiling to `count(DISTINCT ...)`.

```qd
#issues $id $title $#comments.user%distinct // number of distinct commenters
```

### `sum`

Adds up the values of a numeric column, compiling to SQL `sum`.

```qd
#clients $name $#products.id%sum
```

### `avg`

Averages the values of a numeric column, compiling to SQL `avg`.

```qd
#projects $name $#issues.id%avg
```

### `max` (aggregate)

Returns the largest value of a column, compiling to SQL `max`. To compare values within a row instead, use the [scalar `max`](#max).

```qd
#projects $name $#issues.created_at%max // date of the most recent issue
```

### `min` (aggregate)

Returns the smallest value of a column, compiling to SQL `min`. To compare values within a row instead, use the [scalar `min`](#min).

```qd
#projects $name $#issues.created_at%min // date of the oldest issue
```

### `list`

Collects the distinct values of a column into an array, compiling to SQL `array_agg(DISTINCT ...)`. For the non-distinct version, see [`list_all`](#list_all).

```qd
#projects $name $#issues.title%list
```

To sort the list, put [sorting entries](./language.md#sorting-within-an-aggregate-function) in parentheses. Because `list` removes duplicates, each entry must sort either by the listed value itself (`\s`, `\sd`, …) or by an [aggregated sorting expression](./language.md#sorting-distinct-values), which summarizes the records that share each value.

> For each project, list its issue titles in reverse alphabetical order

```qd
#projects $name $#issues.title%list(\sd)
```

> For each issue, list the users who commented on it, in the order of their first comments

```qd
#issues $title $#comments.user.username%list(\\created_at%min)
```

> For each issue, list the users who commented on it, most frequent commenter first

```qd
#issues $title $#comments.user.username%list(\\%count \d \s)
```

Sorting `list` by a per-record expression, such as `\\created_at`, is an error. Use [`list_all`](#list_all) for that.

### `list_all`

Collects the values of a column into an array without removing duplicates, compiling to SQL `array_agg`.

```qd
#projects $name $#issues.title%list_all
```

Unlike `list`, `list_all` may be sorted by any expression, including per-record ones, since it doesn't need to reconcile duplicates:

> For each issue, list the username of each comment's author, oldest comment first

```qd
#issues $title $#comments.user.username%list_all(\\created_at)
```

### `top`

Collects the distinct values of a column that tie for first place under its sorting entries, as an array. Sorting entries work just as they do for [`list`](#list), but at least one is required, since they decide what comes first. Ties are values that tie on _every_ sorting entry, so adding a tiebreaker (such as `\s`) leaves a single value. The tied values are collected in ascending order.

> For each issue, show the users who made the most comments on it

```qd
#issues $title $#comments.user.username%top(\\%count \d)
```

> For each user, show the issue (or issues) that they commented on most recently

```qd
#users $username $#comments.issue.title%top(\\created_at%max \d)
```

> For each project, show the issue title that comes last alphabetically

```qd
#projects $name $#issues.title%top(\sd)
```

See [Keeping only the top values](./language.md#keeping-only-the-top-values) for more. `top` is only supported for values from related records via a `#` path, not for a column of [grouped](./language.md#grouping-and-aggregation) records.

### `all_true`

Returns true when _every_ value of a boolean column is true, compiling to SQL `bool_and`.

```qd
#clients $name $#projects.is_active%all_true // all projects active?
```

### `any_true`

Returns true when _at least one_ value of a boolean column is true, compiling to SQL `bool_or`.

```qd
#clients $name $#projects.is_active%any_true // any project active?
```

### `product`

Multiplies the values of a numeric column together. DuckDB has a native `product` aggregate; for Postgres (which has none) it is reconstructed from sums of logarithms.

```qd
#projects $name $#issues.id%product
```

## Window functions

Window functions compute a value for each row from a set of rows around it, without collapsing
those rows into one. You apply them to a window, defined with `%%( )`, as in `%%( … )%func`. The
language page explains [how to define a window](./language.md#window-functions): how to partition
and order it, and how to pass arguments.

Arguments (the column to work on, and any others) go in parentheses after the function's name.
These are the window functions that both PostgreSQL and DuckDB support.

### `row_number`

Assigns a unique sequential integer (starting at 1) to each row within its partition, in the window's
order. Takes no value argument. Compiles to SQL `row_number`.

```qd
#issues $id $%%(project\p created_at\s)%row_number -> rn
```

### `rank`

Ranks rows within the partition, leaving gaps after ties. Takes no value argument. Compiles to SQL
`rank`.

```qd
#issues $id $%%(project\p created_at\s)%rank -> rank
```

### `dense_rank`

Like `rank`, but without gaps after ties. Takes no value argument. Compiles to SQL `dense_rank`.

```qd
#issues $id $%%(project\p created_at\s)%dense_rank -> rank
```

### `percent_rank`

The relative rank of a row as a value between 0 and 1. Takes no value argument. Compiles to SQL
`percent_rank`.

```qd
#issues $id $%%(project\p created_at\s)%percent_rank -> percentile
```

### `cume_dist`

The cumulative distribution of a row within its partition: the fraction of rows that come at or
before it. Takes no value argument. Compiles to SQL `cume_dist`.

```qd
#issues $id $%%(project\p created_at\s)%cume_dist -> fraction
```

### `ntile`

Divides each partition into the given number of buckets and returns the bucket number of each row.
Takes the bucket count as its argument. Compiles to SQL `ntile`.

```qd
#issues $id $%%(status\p id\s)%ntile(4) -> quartile
```

### `lag`

Returns a value from a row a given number of rows *before* the current row. Arguments: the column,
then an optional offset (default 1), then an optional default value for when no such row exists.
Compiles to SQL `lag`.

```qd
#issues $id $%%(project\p created_at\s)%lag(status 1 "none") -> previous_status
```

### `lead`

Like `lag`, but reads from a row *after* the current row. Compiles to SQL `lead`.

```qd
#issues $id $%%(project\p created_at\s)%lead(status) -> next_status
```

### `first_value`

Returns the value of the given column from the first row of the window frame. Compiles to SQL
`first_value`.

```qd
#issues $id $%%(project\p created_at\s)%first_value(title) -> first_title
```

### `last_value`

Returns the value of the given column from the last row of the window frame. Compiles to SQL
`last_value`. The default frame ends at the current row (and the rows that tie with it), so this is
rarely the last row of the partition.

```qd
#issues $id $%%(project\p created_at\s)%last_value(title) -> latest_title
```

### `nth_value`

Returns the value of the given column from the nth row of the window frame. Arguments: the column and
the position `n`. Compiles to SQL `nth_value`.

```qd
#issues $id $%%(project\p created_at\s)%nth_value(title 2) -> second_title
```

### Aggregates as window functions

The aggregate functions `count`, `sum`, `avg`, `min`, and `max` can be applied to a window too. They
then give a running or partition-wide value for each row, instead of collapsing the rows. Each takes
the column to aggregate as its argument, and `count` can be used on its own (`%count`) to count rows.

```qd
#issues $id $%%(project\p created_at\s)%sum(id) -> running_total
```
