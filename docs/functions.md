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

Returns the number of characters in a string, compiling to SQL `char_length`. Given an [array](#array-functions), it returns the number of elements instead.

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

### `regex_replace`

Replaces every match of a [regular expression](./language.md#regular-expression-matching) with a replacement, compiling to SQL `regexp_replace`. In the replacement, `\1` stands for the text of the first capture group, `\2` for the second, and so on.

> Show each user's email address with its "@" spelled out

```qd
#users $email|regex_replace('@' " at ")
```

> Show each issue's title with "fix" at its start capitalized, matching case-sensitively

```qd
#issues $title|regex_replace('^fix' "Fix" "c")
```

- It takes the pattern, then the replacement, then optional flags.
- The pattern is matched as by [`:~`](./language.md#regular-expression-matching): case-insensitively, unless the flags say otherwise.
- The flags are a string of the letters that `:~` takes after a backslash, with the same meanings: `c`, `m`, and `s` (see [regex flags](./language.md#regex-flags)). They must be a literal string, as in `"cm"`, or a constant set to one.
- Write the pattern and the replacement as [raw strings](./language.md#string-literals), as in `'\1'`, so that a backslash reaches the database intact.

### `regex_extract`

Returns the first match of a [regular expression](./language.md#regular-expression-matching), or NULL if there's none.

> Show the first number in each issue's title

```qd
#issues $title|regex_extract('\d+')
```

Given a group number, it returns the text of that capture group instead, where 1 is the first (and 0 is the whole match).

> Show the domain of each user's email address

```qd
#users $email|regex_extract('@(.+)$' 1) -> domain
```

- It takes the pattern, then an optional group number, then optional flags, which work as for [`regex_replace`](#regex_replace). To give flags without a group, give the group 0, as in `regex_extract('^fix' 0 "c")`.
- The group number must be a literal whole number, or a constant set to one.
- A group that takes no part in the match gives NULL in Postgres, but the empty string in DuckDB. In `'(a)|(b)'`, matched against `"a"`, group 2 is such a group.
- Postgres compiles it to `regexp_match`, with the pattern wrapped in a group of its own, so a backreference such as `\1` in the pattern counts that group too. DuckDB compiles it to `regexp_extract`.

### `split`

Splits a string at each occurrence of a separator, giving an [array](#array-functions) of strings. Compiles to SQL `string_to_array` (Postgres) or `string_split` (DuckDB).

> Show the domain of each user's email address

```qd
#users $email|split("@")|at(1) -> domain
```

- The separator is plain text, not a regular expression.
- Empty text has no parts, so it gives an empty array.
- An empty separator splits a string into its characters.
- [`join`](#join) does the opposite.

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

### `round`

Rounds a number to the nearest integer, or to a number of decimal places. A half rounds away from zero, so 2.5 rounds to 3, and -2.5 to -3.

> Show how many days old each issue is, to one decimal place

```qd
#issues $title $created_at|age|days|round(1) -> days_old
```

- A negative number of decimal places rounds to tens, hundreds, and so on: `round(-2)` rounds 1250 to 1300.
- NaN and infinity stay as they are.
- Postgres compiles it to `ROUND` of the number cast to `numeric`, since it rounds a half to even on `double precision`. DuckDB compiles it to `round`.

### `ln`

Returns the natural logarithm of a number, compiling to SQL `LN`. With [`exp`](#exp) and [`pow`](#pow), it gives logarithms to any base: `x|ln|divide(b|ln)` is the logarithm of `x` to base `b`.

```qd
#issues $id|ln
```

### `gcd`

Returns the greatest common divisor of two integers, compiling to SQL `gcd`.

```qd
#issues $id|gcd(12)
```

### `lcm`

Returns the least common multiple of two integers, compiling to SQL `lcm`.

```qd
#issues $id|lcm(12)
```

### `is_nan`

Returns whether a number is NaN, the floating-point value "not a number". Compiles to `isnan` (DuckDB), or to a comparison with NaN (Postgres, which unlike most languages treats NaN as equal to itself).

> Find invoices whose amount is NaN

```qd
#/billing/invoices amount|is_nan
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

### `seconds_to_duration`

Converts a number of seconds into an interval, the opposite of [`seconds`](#seconds). Compiles to `make_interval(secs => value)` (Postgres) or `to_seconds(value)` (DuckDB).

> Show an hour after each issue was created

```qd
#issues $title $created_at|plus(3600|seconds_to_duration)
```

Multiply first to convert from other units, as in `n|times(86400)|seconds_to_duration` for `n` days.

## Conversion functions

### `to_number`

Converts text to a number, compiling to `CAST(value AS DOUBLE PRECISION)`. Text that isn't a number is an error from the database.

> Show the first number in each issue's title, as a number

```qd
#issues $title|regex_extract('\d+')|to_number
```

### `to_date`

Converts [ISO 8601](https://en.wikipedia.org/wiki/ISO_8601) text, such as `"2024-03-01"`, to a date, or a timestamp to its date. Compiles to `CAST(value AS DATE)`. Text that isn't a date is an error from the database.

> Show the number of issues created on each day

```qd
#issues $created_at|to_date \g $%count
```

A zoned timestamp (`timestamptz`) gives its date in the session time zone on Postgres. On DuckDB, which can't operate on zoned timestamps without the ICU extension, it gives its date in UTC.

### `to_timestamp`

Converts ISO 8601 text, such as `"2024-03-01T09:30:00"`, to a timestamp without a time zone. Compiles to `CAST(value AS TIMESTAMP)`. Text that isn't a timestamp is an error from the database.

> Find issues created after 9:30 on 1 March 2024

```qd
#issues created_at:>"2024-03-01T09:30:00"|to_timestamp
```

To convert a number or a date to text, use a [formatted string](./language.md#formatted-strings), as in `^f"{id}"`.

## Array functions

An array is a list of values in one column. You get one from [`array`](#array), [`split`](#split), or the aggregate functions [`list`](#list), [`list_all`](#list_all), and [`top`](#top), or from a column whose type is an array.

Five of these, [`map`](#map), [`filter`](#filter), [`some`](#some), [`every`](#every), and [`reduce`](#reduce), apply a [function that you give them](./language.md#functions-as-arguments) to each element.

### `array`

Returns an array of the given values. It's called [without a pipe](./language.md#function-calling).

> Show each user's username and email address, as an array

```qd
#users $@@array(username email) -> names
```

- It needs at least one value.
- The values must share a type, or the database reports an error.
- A NULL value is kept as an element.
- Compiles to `ARRAY[a, b]` (Postgres) or `[a, b]` (DuckDB).

### `at`

Returns the element of an array at a position, or NULL if there's none. Positions count from 0, and a negative position counts from the end, so `at(-1)` is the last element.

> Show the body of each issue's latest comment

```qd
#issues $title $#comments.body%list_all(\\created_at)|at(-1) -> latest_comment
```

- The position can be any number, as in `at(id|mod(3))`.
- Compiles to a subscript, `array[position]`, in Postgres, and to `list_extract` in DuckDB. Postgres has no negative subscripts, so a position from the end takes the array's length, with `cardinality`.

### `join`

Joins the elements of an array into one string, with a separator between them. Elements that are NULL are skipped. The opposite of [`split`](#split).

> Show the names of each issue's labels, in one string

```qd
#issues $title $#labels.name%list(\s)|join(", ") -> labels
```

- An empty array, or one of only NULLs, gives the empty string.
- Compiles to SQL `array_to_string`. In DuckDB, whose `array_to_string` needs a constant separator, a separator that isn't a literal string is written between the elements with `list_reduce`.

### `length` of an array

[`length`](#length) returns the number of elements of an array, compiling to `cardinality` (Postgres) or `len` (DuckDB).

> Show the number of different users who commented on each issue

```qd
#issues $title $#comments.user%list|length -> commenters
```

### `map`

Applies a [function](./language.md#functions-as-arguments) to each element of an array, and returns an array of the results, in the same order.

> Show the words of each issue's title, in lower case

```qd
#issues $title|split(" ")|map(@w => @w|lowercase) -> words
```

- The function may take a second parameter, the element's position, counting from 0, as in `map(@w @i => ^f"{@i}: {@w}")`.
- A NULL array gives NULL.
- Compiles to a subquery over the elements, `ARRAY(SELECT … FROM unnest(…))`, in Postgres, and to `list_transform` in DuckDB.

### `filter`

Returns the elements of an array for which a [function](./language.md#functions-as-arguments) is true, in the same order.

> Show the words of each issue's title that are longer than three letters

```qd
#issues $title|split(" ")|filter(@w => @w|length:>3) -> long_words
```

The function may take a second parameter, the element's position, counting from 0.

> Show the first three words of each issue's title

```qd
#issues $title|split(" ")|filter(@w @i => @i:<3)|join(" ") -> start
```

- An element for which the function gives NULL is left out.
- A NULL array gives NULL.
- Compiles to a subquery over the elements, `ARRAY(SELECT … FROM unnest(…) WHERE …)`, in Postgres, and to `list_filter` in DuckDB.

### `some`

Returns whether a [function](./language.md#functions-as-arguments) is true for at least one element of an array.

> Find issues that a comment quotes the title of

```qd
#issues #comments.body%list|some(@b => @b:`title`) $title
```

- The function sees the columns of the row, like `title` above. On the right of a comparison, a column needs [backticks](./language.md#referencing-a-column-on-the-right-hand-side), as there.
- An empty array gives false, and a NULL array gives NULL.
- An element for which the function gives NULL counts as one for which it's false.
- The function may take a second parameter, the element's position, counting from 0.
- Compiles to `EXISTS` with a subquery over the elements in Postgres, and to `list_filter` in DuckDB.

### `every`

Returns whether a [function](./language.md#functions-as-arguments) is true for every element of an array.

> Find issues whose title has no word longer than ten letters

```qd
#issues title|split(" ")|every(@w => @w|length:<=10) $title
```

- An empty array gives true, and a NULL array gives NULL.
- An element for which the function gives NULL counts as one for which it's false.
- The function may take a second parameter, the element's position, counting from 0.
- Compiles to `NOT EXISTS` with a subquery over the elements in Postgres, and to `list_filter` in DuckDB.

### `reduce`

Combines the elements of an array into one value, from the left, with a [function](./language.md#functions-as-arguments) of two parameters. The first parameter starts as the first element. For each element after it, the function is given the value so far and the element, and its result becomes the value so far.

> Show the words of each issue's title, in reverse order

```qd
#issues $title|split(" ")|reduce(@a @b => ^f"{@b} {@a}") -> reversed
```

> Show the length of the longest word of each issue's title

```qd
#issues $title|split(" ")|map(@w => @w|length)|reduce(@a @b => @a|max(@b)) -> longest_word
```

- An array of one element gives that element, and an empty or NULL array gives NULL.
- The function's result should have the type of the elements, since it's given back to the function as the value so far. Otherwise, the database may report an error.
- Postgres has no direct equivalent, so `reduce` compiles to a recursive query there, which runs for each row. It works, but it's slow on long arrays. To join text, [`join`](#join) is quicker. In DuckDB, `reduce` compiles to `list_reduce`.

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
