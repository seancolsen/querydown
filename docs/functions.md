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
#issues $title $due_date|away|days|ceil
```

### `floor`

Rounds a number down to the nearest integer, compiling to SQL `FLOOR`.

```qd
#issues $title $due_date|away|days|floor
```

### `plus`

Adds two numbers. Equivalent to the `+` operator, but usable in a pipeline.

```qd
#issues $title $due_date|away|days|plus(7)
```

### `minus`

Subtracts the argument from the piped value. Equivalent to the `-` operator, but usable in a pipeline.

```qd
#issues $title $due_date|away|days|minus(7)
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
#issues $title $due_date|away|days|keep_above(0) // never below 0
```

### `keep_below`

Caps the value at an upper bound, returning the lesser of the piped value and the argument. Compiles to SQL `LEAST`.

```qd
#issues $title $due_date|away|days|keep_below(30) // never above 30
```

### `max`

Returns the greatest of all its arguments (the piped value plus any in parentheses), compiling to SQL `GREATEST`. This is a row-wise scalar function; for the aggregate version see [`max`](#max-1) below.

```qd
#issues $title $due_date|away|days|max(0)
```

### `min`

Returns the least of all its arguments (the piped value plus any in parentheses), compiling to SQL `LEAST`. This is a row-wise scalar function; for the aggregate version see [`min`](#min-1) below.

```qd
#issues $title $due_date|away|days|min(0)
```

## Date & time functions

### `age`

Returns the interval elapsed _since_ a past timestamp, compiling to `NOW() - value`. Typically applied to a date column and chained with [`days`](#days), [`hours`](#hours), etc.

```qd
#issues $title $created_at|age|days
```

### `ago`

Returns the timestamp that lies a given duration _in the past_, compiling to `NOW() - value`. Typically applied to a [duration literal](./language.md#duration-literals) within a condition.

```qd
#issues created_at:>@6M|ago // created within the last 6 months
```

_(`age` and `ago` are equivalent; the two names exist for readability depending on whether you apply the function to a date or to a duration.)_

### `away`

Returns the interval from now _until_ a future timestamp, compiling to `value - NOW()`. The counterpart to [`age`](#age): for a future date it is positive, and for a past date it is negative. Typically applied to a date column and chained with [`days`](#days), [`hours`](#hours), etc.

```qd
#issues $title $due_date|away|days // days until each issue is due
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

Collects the values of a column into an array, compiling to SQL `array_agg`.

```qd
#projects $name $#issues.title%list
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
