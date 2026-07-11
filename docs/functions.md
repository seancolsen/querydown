# Querydown functions

Querydown provides a set of built-in functions in two flavors:

- **Scalar** functions operate within a single row and are applied with the `|` (pipe) operator.
- **Aggregate** functions combine values across many related rows and are applied with the `%` operator.

See **[Function piping](./language.md#function-piping)** for more on the syntax. In short:

- The value to the **left** of the pipe becomes the function's **first argument**.
- Any **additional** arguments are supplied in parentheses, separated by **spaces** (not commas).
- When a function takes only one argument, the parentheses may be omitted.

```qd
#issues $status|concat(": ")|if_null("")|concat(title)
```

These examples use the [issue-tracker schema](./language.md#example-schema).

## General functions

### `if_null`

Returns the first non-null value among its arguments, compiling to SQL `COALESCE`. Accepts any number of arguments: the piped value, followed by any fallbacks in parentheses.

```qd
#issues $status|if_null("unknown")
```

## Boolean functions

### `not`

Negates a boolean value, compiling to SQL `NOT`.

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

Returns the greatest of all its arguments (the piped value plus any in parentheses), compiling to SQL `GREATEST`. This is a row-wise scalar function; for the aggregate version see [`max`](#max-1) below.

```qd
#issues $title $due_date|countdown|days|max(0)
```

### `min`

Returns the least of all its arguments (the piped value plus any in parentheses), compiling to SQL `LEAST`. This is a row-wise scalar function; for the aggregate version see [`min`](#min-1) below.

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

## Date & time functions

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

Aggregate functions are applied with `%` and combine values across many related rows. They may be applied only to a **column path** (optionally reaching across a to-many relationship), not to an arbitrary computed expression. See [Specifying an aggregate function](./language.md#specifying-an-aggregate-function) for more.

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

### `max`

Returns the largest value of a column, compiling to SQL `max`. This is the aggregate version; for the row-wise scalar version see [`max`](#max) above.

```qd
#projects $name $#issues.created_at%max // date of the most recent issue
```

### `min`

Returns the smallest value of a column, compiling to SQL `min`. This is the aggregate version; for the row-wise scalar version see [`min`](#min) above.

```qd
#projects $name $#issues.created_at%min // date of the oldest issue
```

### `list`

Collects the distinct values of a column into an array, compiling to SQL `array_agg(DISTINCT ...)`. For the non-distinct version, see [`list_all`](#list_all).

```qd
#projects $name $#issues.title%list
```

If `list` is sorted (see [Sorting within an aggregate function](./language.md#sorting-within-an-aggregate-function)) by something other than the collected column itself, it compiles to a non-distinct `array_agg` instead — SQL only allows a `DISTINCT` aggregate's `ORDER BY` to sort by the value being aggregated.

### `list_all`

Collects the values of a column into an array without removing duplicates, compiling to SQL `array_agg`.

```qd
#projects $name $#issues.title%list_all
```

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

Window functions compute a value across a set of rows related to the current row, without collapsing
them. They are applied over a window definition with the `%%( … )%func` syntax — see [Window
functions](./language.md#window-functions) in the language guide for the syntax, including how
partitioning, ordering, and value arguments are written.

The set below is restricted to functions that both Postgres and DuckDB support. Value arguments (the
column to operate on, plus any extras) are written in parentheses after the function name.

### `row_number`

Assigns a unique sequential integer (starting at 1) to each row within its partition, in the window's
order. Takes no value argument. Compiles to SQL `row_number`.

```qd
#issues $id $%%(project\p created_at\s)%row_number -> rn
```

### `rank`

Ranks rows within the partition, leaving gaps after ties. Takes no value argument. Compiles to SQL
`rank`.

### `dense_rank`

Like `rank`, but without gaps after ties. Takes no value argument. Compiles to SQL `dense_rank`.

### `percent_rank`

The relative rank of a row as a value between 0 and 1. Takes no value argument. Compiles to SQL
`percent_rank`.

### `cume_dist`

The cumulative distribution of a row within its partition. Takes no value argument. Compiles to SQL
`cume_dist`.

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

### `first_value`

Returns the value of the given column from the first row of the window frame. Compiles to SQL
`first_value`.

### `last_value`

Returns the value of the given column from the last row of the window frame. Compiles to SQL
`last_value`. Note that the default frame ends at the current row.

### `nth_value`

Returns the value of the given column from the nth row of the window frame. Arguments: the column and
the position `n`. Compiles to SQL `nth_value`.

### `count`, `sum`, `avg`, `min`, `max`

The standard aggregates can also be applied as window functions, producing a running or
partition-wide value rather than collapsing rows. Each takes the column to aggregate; `count` may be
used on its own (`%count`) to mean `count(*)`.

```qd
#issues $id $%%(project\p created_at\s)%sum(id) -> running_total
```
