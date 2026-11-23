# Language

This page describes the Querydown language, one feature at a time. For a quicker reminder of the syntax, see the [cheat sheet](./cheat-sheet.md). The built-in functions have [a page of their own](./functions.md).

Every example is written against the [sample issue-tracker schema](./sample-schema.md), and you can open any of them in the [playground](/playground) to see the SQL it compiles to.

## Query anatomy

A query has the following sections:

```qd
// Definitions (optional)
   @min_comments = 5
   #issues.label:@x = ++#labels{name:@x}

// The base table (REQUIRED)
   #issues

// Conditions (optional)
   #comments:>=@min_comments
   label:bug

// Sorting (optional)
   \\due_date \d

// Result columns (optional)
   $id
   $title
   $author.*
```

- [Definitions](#variables), such as constants and functions, go first.
- Every query has exactly one base table.
- [Conditions](#comparisons-and-conditions) can follow the base table, separated by spaces. If you omit them, the query returns all the rows of the table.
- Sorting can be specified in [the sorting section](#sorting-outside-of-result-columns) (as above), or via [column control flags](#basic-sorting) on result columns.
- [Result columns](#result-columns) are expressions that each follow a dollar sign `$`. If you omit them, the query returns all the columns of the base table.
- Most white space doesn't matter.
- Comments are written with `//` for a single line, or `/* */` for several lines.

- [Pipeline](#pipeline-of-multiple-queries) and [union](#union-of-multiple-queries) can combine multiple queries into one.

The results never have more rows than the base table does. Data from related tables is [always aggregated](#mandatory-aggregation), so you never write a join.


## Values

### Identifiers

Identifiers are the names of tables and columns. A table name follows a `#` sigil, and a column name is written as it is.

```qd
#issues $title $due_date
```

- An unquoted identifier must begin with a letter or an underscore, and contain only letters, numbers, and underscores.
- To use any other name, quote it with backticks, e.g. `` `Due Date` ``. Like [strings](#string-literals), quoted identifiers are raw, so a backslash is literal and an identifier can't contain a backtick.
- Unlike in SQL, column names like `group` and `year` don't need quotes. Querydown has no keywords, and the syntax around a name always says whether it's a function.
- Note that a bare (unquoted) word on the [right-hand-side of a comparison](#bare-text-on-the-right-hand-side-of-a-comparison) is parsed as a string literal, not a column reference. Use backticks there if you need to reference a column.
- Likewise, an unquoted word made up of only letters and digits that stands alone as a condition is a [default text search](#default-text-search) term, not a column reference.

### Flexible identifiers

Table and column names are resolved "flexibly", which saves you from quoting most names that have spaces or capital letters.

If a table or column isn't found exactly as you wrote it, the compiler looks for a _unique_ match, comparing only the lowercased ASCII letters and numbers of each name. So `foo_bar` resolves to a column named `Foo Bar`, but only if it doesn't also match another column, like `foobar`.

The sample schema's names are all plain, so this matters most with a schema of your own. An application can turn it off by [compiling](./setup/rust.md#options) with strict identifier resolution.

### Tables in other schemas

A database can keep its tables in several schemas. A table in the current schema is written by its name alone, and a table in another schema is written with the name of its schema first, each name after a `/`.

> Unpaid invoices, oldest first, with the names of their clients

```qd
#/billing/invoices is_paid:@false \\issued_on $client.name $amount
```

Here `#/billing/invoices` is the table `invoices` in the schema `billing`, and `client` leads to a record of `clients`, which is in the current schema. In the [sample schema](./sample-schema.md), `invoices` is the only table in another schema.

A table in another schema can go anywhere a table can, such as among [many related records](#referencing-multiple-related-records):

> Each client, with their total of unpaid invoices

```qd
#clients $name $#/billing/invoices{is_paid:@false}.amount%sum -> unpaid
```

- Which schema is the current one, and which tables are in it, is up to the [schema JSON](./setup/introspection.md#schemas). Introspection puts the tables of your database's current schema there (usually `public` in PostgreSQL, and `main` in DuckDB).
- A schema can be inside another, so a table can have more than one name before its own. In DuckDB, a table of another attached database has the database's name, then the schema's, as in `#/archive/main/invoices`.
- Tables in different schemas can have the same name. `#invoices` is a table in the current schema, not `#/billing/invoices`. If there's no `#invoices`, the error suggests the tables of that name in other schemas.
- Relationships work across schemas, in both directions, just as they do within one.
- Each name can be [quoted](#identifiers) with backticks, as in `` #/`Billing Dept`/invoices ``, and is resolved [flexibly](#flexible-identifiers).
- A [user-defined table](#user-defined-tables) has no schema.
- After the name of a table in another schema, a `/` followed by a name continues the table's name, so put a space after a `/` that divides: `#/billing/invoices / x` is a division, but `#/billing/invoices/x` is the table `x` in the schema `invoices`, inside `billing`. A `/` after a table in the current schema is always a division.

### Built-in constants

Constants are written with a `@` sigil. These ones are built in:

| Querydown | SQL |
| -- | -- |
| `@now`      | `now()`    |
| `@infinity` | `CAST('infinity' AS double precision)` |
| `@true`     | `TRUE`     |
| `@false`    | `FALSE`    |
| `@null`     | `NULL`     |

> Issues that are past their due date and have no description

```qd
#issues due_date:<@now description:@null
```

You can [define constants](#user-defined-constants) of your own too.

### String literals

Text is written within quotes.

| Example | Explanation |
| -- | -- |
| `"foo"` | With double quotes |
| `'foo'` | With single quotes |

- You can quote a string with single quotes or with double quotes.
- Strings are raw: there are no escape sequences. A backslash is a literal character, so `'\n'` is a backslash followed by `n` rather than a newline, and a regular expression like `title:~'v\d+'` needs no doubled backslashes.
- A string can't contain its own quote character, so quote it with the other one instead, e.g. `"can't"` or `'say "hi"'`. For a string containing both kinds of quote, use a [flagged string](#flagged-strings).
- Strings may span multiple lines.

### Flagged strings

A string prefixed with `^` is a flagged string. Flags after the `^` change how the string is read, and you get a wider choice of quote characters.

| Example | Explanation |
| -- | -- |
| `^f"Hello {username}!"`   | Using the `f` flag to interpolate values |
| `^e'Don\'t say "never"'` | Using the `e` flag to interpret escape sequences |
| `^^Don't say "never"^`   | Using a custom quote character to avoid escape sequences |

| Flag | Meaning |
| --   | -- |
| `f`  | **format**: interpolate the expressions written within `{ }` |
| `e`  | interpret **escape** sequences (strings are raw by default) |

- The flags follow `^` directly, in any order. Each may be given at most once, and they're case-sensitive.
- The flags can also be omitted, so `^` alone can prefix a string. This is still a "flagged string", and is useful for its choice of quotes.
- A flagged string may be quoted with any of the following characters: `" ' ^ # / |`. Choose one the string doesn't contain, e.g. `^/v\d+/` or `^|say "can't"|`.
- Like other strings, flagged strings may span multiple lines.

### Formatted strings

The `f` flag makes a string a template: each expression within `{ }` is replaced by its value.

> Show each user's name and email address, formatted together as one value.

```qd
#users $^f"{username} <{email}>" -> contact
```

- Any expression can be interpolated, e.g. `^f"{title|uppercase} ({#comments} comments)"`.
- Within `{ }`, a bare word is always a column reference, even on the [right-hand-side of a comparison](#bare-text-on-the-right-hand-side-of-a-comparison). For example, `description:^f"see {title}"` finds issues whose description contains the text "see " followed by the issue's own title.
- The string ends at the first quote character _outside_ of the `{ }`, so an expression within the braces can contain the quote character, e.g. `^f"{status|concat("!")}"`. It may even contain another formatted string.
- To write a literal brace, double it: `{{` or `}}`.
- An interpolated value that isn't text is converted to text, and a `NULL` value becomes empty text. (The string compiles to SQL `concat`.)
- There are no format specifiers like Python's `{x:.2f}`. Within the braces, `:` is a [comparison](#comparisons-and-conditions) as usual, so `{a:b}` interpolates a boolean. To format a value, pipe it to a [function](./functions.md) instead.
- Like any string, a formatted string standing alone as a condition is a [default text search](#default-text-search).

### Escape sequences

Strings are raw unless you give them the `e` flag, which interprets these escape sequences:

| Escape | Meaning |
| -- | -- |
| `\\` | a backslash |
| `\n` | a newline |
| `\t` | a tab |
| `\r` | a carriage return |
| `\u{e9}` | the Unicode character with the given hexadecimal code point (one to six digits), here `é` |
| `\"` | the string's quote character, whichever it is, e.g. `\'` in `^e'it\'s'` |
| `\{` and `\}` | a literal brace, with the `f` flag |

> Find comments that contain a tab character

```qd
#comments body:^e"\t"
```

Any other character after a backslash is an error.

### Date literals

A date is written in [ISO 8601](https://en.wikipedia.org/wiki/ISO_8601) format after a `@` sigil.

> Issues created on or after the first day of 2024

```qd
#issues created_at:>=@2024-01-01
```

There is no literal syntax for a time of day yet, so a timestamp like `@2000-01-01T08:30:00` is a syntax error. A date literal compared against a timestamp column means midnight at the start of that date.

### Duration literals

A duration is a length of time, written as a number directly followed by a unit. Several of these can be strung together.

> Comments made within the past day and a half

```qd
#comments created_at:<1d12h
```

The units are case-insensitive:

| Unit  | Meaning |
| --    | --      |
| `y`   | years   |
| `m`   | months  |
| `w`   | weeks   |
| `d`   | days    |
| `h`   | hours   |
| `min` | minutes |
| `s`   | seconds |

Note that `m` always means months and `min` always means minutes, so the two are never ambiguous. A duration compared against a date [measures from now](#comparing-dates-with-durations), as in the example above.

| Example         | Meaning |
| --              | -- |
| `2y`            | 2 years |
| `2.5y`          | 2.5 years |
| `1y2d`          | 1 year and 2 days |
| `9m`            | 9 months |
| `9min`          | 9 minutes |
| `1h`            | 1 hour |
| `2y6m3d8h9min34.89s` | 2 years, 6 months, 3 days, 8 hours, 9 minutes, and 34.89 seconds |
| `0y`            | (empty) |


## Computations

### Arithmetic

The four basic arithmetic operators work on numbers, dates, and durations, with their usual precedence.

> For each issue, show how many days it was given from its creation until its due date

```qd
#issues $title $(due_date - created_at)|days
```

| Operator | Meaning |
| -- | -- |
|  `+` | Addition |
|  `-` | Subtraction |
|  `*` | Multiplication |
|  `/` | Division |

There are no other math operators. Everything else is a [function](./functions.md#math-functions), applied by name.

### Function piping

The pipe `|` applies a function to the value on its left. Pipes chain, so each function receives the result of the one before it.

> Show issues, along with the number of days until the due date.

```qd
#issues $* $due_date|countdown|days
```

- The piped value is the function's first argument. Any more arguments go in parentheses after the function's name, separated by spaces, as in `status|if_null("unknown")`.
- See the list of [all functions](./functions.md).
- The pipe has the highest [operator precedence](./cheat-sheet.md#operator-precedence), so it's evaluated before multiplication and division.
- [_Aggregate_ functions](#specifying-an-aggregate-function) are applied with `%` instead of `|`.

### Case expressions

A case expression chooses between values, like an if/else chain. It's written with `?`, `~`, and `~~`.

> Categorize each issue into being either "overdue", "due soon", or "due later".

```qd
#issues
$title
$ ?
  due_date|countdown|days:<0  ~ "overdue"
  due_date|countdown|days:<30 ~ "due soon"
  ~~ "due later"
```

- `?` begins a case expression.
- `~` denotes a case variant by separating its test expression from its corresponding value expression.
- `~~` prefixes the fallback value and indicates the end of the case expression.

A case expression requires at least one variant and a fallback. It compiles to a SQL `CASE` expression, evaluating the variants in order and yielding the value of the first variant whose test expression is true, or the fallback if none are.

### Anonymous functions

An anonymous function names a value so that you can use it several times within one expression. Write it in parentheses as `(@parameter => body)`, and pipe a value into it.

> Categorize each issue into being either "overdue", "due soon", or "due later".

```qd
#issues
$title
$due_date|countdown|days|(@d => ? @d:<0~"overdue" @d:<30~"due soon" ~~"due later")
```

In the query above:

1. `due_date|countdown|days` computes the number of days until the issue's due date.
1. That value becomes the `@d` parameter of the anonymous function:

    ```qd fragment
    (@d => ? @d:<0~"overdue" @d:<30~"due soon" ~~"due later")
    ```

1. The body of the function refers to `@d` twice, without repeating the computation.

To reuse a function across a query, give it a name with a [user-defined function](#user-defined-functions).

An anonymous function can take more than one parameter. The piped-in value binds to the first parameter, and the remaining arguments are supplied in parentheses (separated by spaces) directly after the function:

```qd
#issues
$title
$due_date|countdown|days|(@due @age => ? @due:<0 ~ "overdue" @age:>365 ~ "stale" ~~ "on track")(created_at|age|days)
```

Here `@due` is the number of days until the due date (piped in), and `@age` is the number of days since the issue was created (from the trailing `(created_at|age|days)`).

### Functions as arguments

A few functions take a function as an argument, and apply it to each element of an [array](./functions.md#array-functions). Write the function like an anonymous function, but without its parentheses, as in `map(@w => @w|lowercase)`.

> Show the words of each issue's title that are longer than three letters, in upper case

```qd
#issues $title|split(" ")|filter(@w => @w|length:>3)|map(@w => @w|uppercase)
```

Here `split(" ")` turns the title into an array of words. `filter` keeps the words for which its function is true, and `map` applies its function to each of those.

- Only [`map`](./functions.md#map), [`filter`](./functions.md#filter), [`some`](./functions.md#some), [`every`](./functions.md#every), and [`reduce`](./functions.md#reduce) take a function. Giving one to any other function is an error.
- The body sees the columns of the row and the variables around it, as an anonymous function's does. So in ``#comments.body%list|some(@b => @b:`title`)``, each comment is compared with the issue's own `title`.
- A parameter can have the name of a variable around it, which it hides within the body.
- Functions can be given within functions, as in `title|split(" ")|some(@w => @w|split("")|every(@c => @c:~'[0-9]'))`, which finds titles with a word made only of digits.
- The body can't use an aggregate of the rows being queried, like `%count`, or a window function. It can use an aggregate of related records, like `#comments%count`, but a parameter can't be used within a filter of related records, like `#comments{body:@w}`, which is found by a query of its own.
- In DuckDB, the function becomes a lambda, which can't contain a subquery, so it can't use a [constant defined by a query](#defining-a-constant-using-the-result-of-a-query).

### Function calling

The `@@` sigil calls a function by name, with all of its arguments in parentheses, instead of piping the first one in.

> For each issue, show the number of days it is overdue, with zero in place of negative numbers

```qd
#issues $title $@@max(due_date|age|days 0)
```

The arguments are separated by spaces, not commas. The query above is the same as this one, which pipes the first argument in:

```qd
#issues $title $due_date|age|days|max(0)
```

Use whichever reads better.

## Comparisons and conditions

A condition is anything that's true or false for each row, and most conditions are comparisons. Every comparison operator begins with a colon. The conditions of a query sit between its base table and its result columns, and a row must satisfy all of them.

### Match comparisons

The match operator `:` is the everyday comparison. It finds text that _contains_ a value, and compares anything else for equality.

> Find issues where the title contains "performance"

```qd
#issues title:"performance"
```

This searches for "performance" anywhere in the title, ignoring case.

- What `:` does depends on the type of the value on its left. Text matches when it contains the value on the right. Anything else (a number, a date, a boolean, and so on) must equal it.
- As a special case, matching text against an **empty string** tests for equality, because every text contains the empty string. So `#issues description:""` finds issues with an empty description, the same as `#issues description:==""`.
- Matching against `@null` finds missing values: `#issues description:@null`.

### Equality comparisons

Use `:=` to compare text for equality while ignoring case, and `:==` to compare it exactly.

> Find issues where the status equals "do", ignoring case

```qd
#issues status:="do"
```

This finds issues whose status is "do" or "Do", but _not_ "done".

> Find issues where the status is exactly "do"

```qd
#issues status:=="do"
```

Case doesn't apply to values that aren't text (numbers, dates, and so on), so for them `:=` and `:==` both test for plain equality.

### Numeric comparisons

Numbers, dates, and durations are compared with `:` for equality, and with `:<` `:<=` `:>` `:>=` for order.

> Find the issue with id 123

```qd
#issues id:123
```

> Find issues with more than 5 comments

```qd
#issues #comments:>5
```

Because `id` isn't text, the match operator tests for equality here: `id:123` is the same as `id:=123`.

### Comparing dates with durations

When you compare a date or datetime against a [duration](#duration-literals), the comparison is about how far the date lies from _now_, in either direction. So `date:<duration` matches when the date falls within `duration` of the present moment, whether it's in the past or the future.

> Find issues created within the past 6 months:

```qd
#issues created_at:<6m
```

> Find issues due within 6 months:

```qd
#issues due_date:<6m
```

Both queries use `:<`, even though one date lies in the past and the other in the future. Under the hood the comparison compiles to `ABS(NOW() - date) < duration`, so the sign of the difference doesn't matter.

The date must be on the **left** and the duration on the **right**. The reversed order is an error, just as comparing a datetime against a duration directly would be.

#### Computed dates and durations

Neither side needs to be written literally. Querydown infers the type of each expression, propagating it through functions, arithmetic, and [variables](#variables), so any expression that works out to a date or datetime can be compared against any expression that works out to a duration. For example, a datetime run through an aggregate function is still a datetime:

> Find issues whose most recent comment was made within the past week:

```qd
#issues #comments.created_at%max:<1w
```

And a sum of durations is still a duration, as is a [constant](#user-defined-constants) that holds one:

> Find issues created within the past 8 days:

```qd
#issues created_at:<1w + 1d
```

The _difference_ between two dates is a duration too, not a date. So comparing it against a duration is an ordinary comparison of two durations, with nothing measured from now:

> Find issues that were given more than a week to complete:

```qd
#issues due_date - created_at:>1w
```

The types that this relies on come from the schema, so the comparison works on any column of a table. It also works on the columns of an earlier [pipeline](#pipeline-of-multiple-queries) stage or a [user-defined table](#user-defined-tables), which keep their date, datetime, and duration types.

#### Comparing in one direction only

To take the direction of the difference into account, compare against a datetime instead of a duration.

> Find issues that are more than 1 week overdue

```qd
#issues due_date:<@now|minus(1w)
```

### Regular expression matching

Use `:~` to match text against a regular expression.

> Find issues with a description matching the regex `front[ -]?end`

```qd
#issues description:~'front[ -]?end'
```

Regex matching is case-insensitive by default. Patterns are [raw strings](#string-literals), so a backslash reaches the regex engine intact (e.g. `'v\d+'`). The regex syntax itself is your database's: [POSIX-style "advanced" regular expressions](https://www.postgresql.org/docs/current/functions-matching.html#POSIX-SYNTAX-DETAILS) in Postgres, and [RE2](https://github.com/google/re2/wiki/Syntax) in DuckDB.

#### Regex flags

To change how a regex matches, write flags between the `:` and the `~` of the operator, after a backslash:

> Find issues whose title starts with "Fix", matching case-sensitively

```qd
#issues title:\c~'^Fix'
```

| Flag | Effect |
| -- | -- |
| `c` | Match case-sensitively. |
| `m` | Multi-line: `^` and `$` match at the start and end of each line, rather than only at the start and end of the whole text. |
| `s` | Dot-all: `.` matches a newline too. |

The flags may be combined in any order, e.g. `\ms`.

> Find issues whose description has a line consisting only of "TODO"

```qd
#issues description:\cm~'^TODO$'
```

Without any flags, a regex matches the same way in every database: case-insensitively, with `.` matching any character _except_ a newline, and with `^` and `$` matching only at the start and end of the whole text. The `m` and `s` flags mean what they do in Python (and JavaScript, Perl, etc.). There's no `i` flag, since case-insensitivity is the default. Python's `x`, `a`, `u`, and `L` flags aren't supported, because the databases don't support them.

A few behaviors differ between databases, because Querydown hands the pattern to the database's own regex engine:

- In Postgres, a negated bracket expression such as `[^a]` doesn't match a newline unless the `s` flag is given. In DuckDB (and Python) it always does.
- Postgres lets an embedded options group like `(?x)` appear only at the very start of a pattern, but Querydown already puts its own options there. Use Querydown's flags instead.

To replace what a regex matches, or to extract it, use the functions [`regex_replace`](./functions.md#regex_replace) and [`regex_extract`](./functions.md#regex_extract). They take the same flags, as a string.

### All comparison operators

| Operator | Meaning |
| -- | -- |
| `:` | [Matches](#match-comparisons): text contains the value, and anything else equals it |
| `:=` | [Equals](#equality-comparisons), ignoring the case of text |
| `:==` | [Equals](#equality-comparisons) exactly |
| `:<` `:<=` | [Less than](#numeric-comparisons), less than or equal to |
| `:>` `:>=` | [Greater than](#numeric-comparisons), greater than or equal to |
| `:~` | [Matches a regular expression](#regular-expression-matching) |
| `:\c~` | Matches a regular expression with [flags](#regex-flags) (any of `c`, `m`, `s`) |

Prefix any comparison with `!` to [negate](#negation) it.

### Bare text on the right-hand-side of a comparison

A bare (unquoted) word on the **right-hand-side** of a comparison is a **string literal**, not a column reference. That's how search works in tools like GitHub, and it saves you the quotes.

> Find issues where the title contains the word "performance"

```qd
#issues title:performance
```

The query above is equivalent to `#issues title:"performance"`.

Here, "word" means any unquoted identifier, so it may contain underscores: `#issues status:in_progress` compares against the text "in_progress". (That's unlike a [default text search](#default-text-search) term, which is made up of letters and digits only.)

#### The rule applies at any depth

This is not limited to a bare word standing alone immediately after the operator. **Every** bare word _anywhere_ within the right-hand-side expression — at any depth of nesting — is a string literal. For example, the bare words inside the following right-hand-sides are all string literals:

- an [expansion](#comparison-expansion): `#issues title:~[color colour]` searches for "color" or "colour"
- a [function pipeline](#function-piping): `#issues title:foo|concat(bar)` builds the string from the literals "foo" and "bar"
- a [range](#ranges): `#issues title:a..m` finds the titles that sort between "a" and "m"
- parentheses, arithmetic, and so on — the same rule reaches into all of them

The motivation is to make the most common case — searching for literal text — both the shortest to type and the most intuitive to read.

#### Everything _outside_ the right-hand-side is a column reference

The string-literal rule is confined to the right-hand-side. Anywhere else, a bare word is still a column reference. Contrast the two pipelines below, which are written identically except for their position:

```qd
#issues title:foo|concat(bar)
```

Here `foo` and `bar` are **string literals**, because they sit on the right-hand-side of `title:`.

```qd
#issues $status|concat(title)
```

Here `status` and `title` are **column references**, because this is a [result column](#result-columns), not the right-hand-side of a comparison.

This boundary even resets within a nested comparison. In `#projects is_active:(name:foo)`, the inner `name` is on the _left_ of its own comparison, so it is a column reference, while `foo` (on the inner right-hand-side) remains a string literal.

It also resets within the `{ }` of a [formatted string](#formatted-strings). In `#issues description:^f"see {title}"`, `title` is a column reference.

#### Referencing a column on the right-hand-side

If you need to reference a column on the right-hand-side, you can either:

- quote the identifier with backticks, e.g. ``#issues description:`title` ``, or
- write it as a multi-part path, e.g. `#issues description:foo.bar`.

### Default text search

A bare word that stands alone as a condition searches all the text columns of the table at once.

> Find issues where the title, description, or status contains "accessibility", and where one of them also contains "feature"

```qd
#issues accessibility feature
```

This gives you a low-friction search across many columns. The example searches `title`, `description`, and `status` because those are the text columns of the `issues` table, though you can [configure which columns are searched](#configuring-the-default-text-search).

A word is a search term when it stands among the conditions without a comparison operator.

#### What counts as a search term

Only a **bare word** is a search term: an unquoted word that begins with a letter and contains only letters and digits. A bare word is _always_ a search term, even when it happens to match a column name — so `#issues title` searches for the literal text "title" rather than referencing the `title` column.

Anything else that looks like a name is a column reference, as it is elsewhere in a query. In particular, a name containing an **underscore** isn't a bare word:

> Find active projects (`is_active` is a boolean column)

```qd
#projects is_active
```

So `#issues in_progress` doesn't search for "in_progress": it refers to a column named `in_progress`, and fails because there isn't one. To search for such text, quote it (`#issues "in_progress"`).

Going the other way, to use a column whose name _is_ a bare word as a condition, quote it with backticks, just as you would on the [right-hand side of a comparison](#bare-text-on-the-right-hand-side-of-a-comparison). If `projects` had a boolean column named `archived`, then `#projects archived` would search for the text "archived", while ``#projects `archived` `` would find the archived projects.

To search for text that isn't a bare word, quote it:

```qd
#issues "localhost:3000"
```

A [constant](#user-defined-constants) that holds a string is a search term too, since a constant stands for its value:

```qd
@term = "localhost:3000"
#issues @term
```

#### Combining and negating search terms

Because the comma is shorthand for an [OR condition set](#or-condition-sets), bare terms can be combined with it to search for either term:

```qd
#issues foo,bar
```

Each side of a comma is a whole condition, so the same is true after a comparison: `#issues status:open,closed` finds issues whose status contains "open", _or_ that mention "closed" in any text column. See [OR condition sets](#or-condition-sets) for how to compare one column with several values.

You can _negate_ a search term by prefixing it with `!`, to find records that do **not** contain the term in any of their text columns:

```qd
#issues !backend
```

Negation works on individual comma operands too, so you can mix positive and negative terms. This finds issues that mention "foo" or do not mention "bar":

```qd
#issues foo,!bar
```

Just as a bare word is always a search term (never a column reference), a `!`-prefixed bare word is always a _negated_ search term. A name that isn't a bare word is a column, negated or not, so `#projects !is_active` finds the projects that aren't active. To negate a column whose name is a bare word, quote it with backticks, e.g. ``#projects !`archived` ``.

#### Configuring the default text search

To choose what the default text search looks at, define a [custom comparison](#custom-comparisons) named `__querydown_default_text_search` on the table.

> Search issues by title, by description, and by the text of their comments, but not by status

```qd
#issues.__querydown_default_text_search:@x = [
  title:@x
  description:@x
  ++#comments{body:@x}
]

#issues accessibility feature
```

An application that embeds Querydown would typically put a definition like this before each query, so that its users get a search suited to each table.

### AND condition sets

Curly brackets `{ }` enclose a set of conditions that must _all_ be true.

> Issues that are open **and** created after 2023-03-04

```qd
#issues {status:"open" created_at:>@2023-03-04}
```

The conditions of a query are an AND set already, so at the top level you can leave the brackets out. The query above is the same as:

```qd
#issues status:"open" created_at:>@2023-03-04
```

### OR condition sets

Square brackets `[ ]` enclose a set of conditions of which _at least one_ must be true.

> Issues that are open **or** created after 2023-03-04

```qd
#issues [status:"open" created_at:>@2023-03-04]
```

The comma `,` is a shorthand for the same thing:

```qd
#issues status:"open",created_at:>@2023-03-04
```

The comma has the lowest [precedence](./cheat-sheet.md#operator-precedence) of any operator, so it always separates whole conditions. It never separates the values of one comparison:

- `status:open,closed` is the comparison `status:open`, _or_ a [default text search](#default-text-search) for "closed" in every text column.
- `id:1,2` is an error, because the `2` stands alone, and a number isn't a condition.

To compare one column with several values, use [comparison expansion](#comparison-expansion) instead: `status:[open closed]` or `id:[1 2]`.

### Nested condition sets

Condition sets can hold other condition sets, which is how you mix AND with OR.

> Issues that are open and created after 2023-03-04 _or_ reopened and created after 2022-11-22:

```qd
#issues [
  {status:"open" created_at:>@2023-03-04}
  {status:"reopened" created_at:>@2022-11-22}
]
```

### Negation

Prefix a comparison, a condition set, or any other condition with `!` to negate it. `!X` finds exactly the records that `X` doesn't. Only a condition can be negated: applying `!` to a literal value, such as a string or a number, is an error.

> Issues that aren't in the "Website" project

```qd
#issues !project.name:="Website"
```

That includes issues with no project at all. A comparison whose operand is NULL (here an issue with no project has no project name) doesn't hold, so its negation does. The same goes for a nullable column (`!status:="open"` includes issues with no status) and for an aggregate over no related records:

> Issues with no comment in the past week, including issues with no comments

```qd
#issues !#comments.created_at%max:<1w
```

So you never need to add a separate `:@null` check to a negated condition. Negating a condition set works the same way: `![project.id:3 status:="closed"]` finds issues that are neither in project 3 nor closed, including open issues with no project.

In SQL terms, `!X` compiles to `X IS NOT TRUE` (or to a plain `NOT X` where `X` can't be NULL), not to a bare `NOT X`, which would be NULL and so drop the record whenever `X` is. The rule holds everywhere `!` appears, including in a result column: `#projects $!is_active` is `true`, not NULL, for a project whose `is_active` is NULL. The [`not`](./functions.md#not) function behaves the same way.

### Comparison expansion

Whenever a condition set (`[ ... ]` for "or", `{ ... }` for "and") appears on either side of a comparison, the comparison is automatically "expanded" across the entries of the set. The comparison is applied to each entry in turn, and the results are combined using the set's conjunction.

Because an expansion on the right-hand-side is part of the right-hand-side, its bare words are [string literals](#bare-text-on-the-right-hand-side-of-a-comparison). An expansion on the left-hand-side holds column references, as everywhere else on the left.

> Issues whose status is either "open" or "reopened":

```qd
#issues status:[open reopened]
```

> Issues that are missing a title and description (the left-hand expansion references columns):

```qd
#issues {title description}:@null
```

> Issues where the title or description contains "foo":

```qd
#issues [title description]:~foo
```

> Projects that are neither active nor pro bono

```qd
#projects {is_pro_bono is_active}:@false
```

The entries of an expansion are the things being compared, not conditions, so an entry can't be negated with `!`. Writing `status:[open !closed]` is an error, since `!` would apply to the value "closed" rather than to a comparison. Either [negate](#negation) the whole comparison, or write the comparisons out:

> Issues whose status is neither "closed" nor "done"

```qd
#issues !status:[closed done]
```

> Issues whose status contains "open", or doesn't contain "closed"

```qd
#issues [status:open !status:closed]
```

A condition set expands a comparison only when it's written directly in the comparison. A set that arrives some other way isn't an expansion: one held by a [constant](#user-defined-constants), passed as an argument to a function, or wrapped in parentheses is a single (boolean) condition, and comparing a value with it is an error.

#### Without comparison expansion

Because a condition set on either side of a comparison is _always_ expanded, you can't use one to form a boolean value and then compare that. To do boolean logic on values _before_ they're compared, use [functions](./functions.md#boolean-functions) instead:

```qd
#projects is_pro_bono|and(is_active):@false
```

Beware that this query gives different results from the one with comparison expansion. If `is_pro_bono` is true and `is_active` is false, then `is_pro_bono|and(is_active)` is false, which matches the condition and includes the row.

### Dual expansion

A comparison can be expanded on both sides at once. The set on the left is the outer one. Notice that the same syntax holds column references on the left (`title`, `description`) but string literals on the right (`foo`, `bar`):

> Issues where the title and description both contain "foo" or contain "bar":

```qd
#issues {title description}:~[foo bar]
```

### Ranges

A range compares a value against a lower and an upper bound at once. Write the two bounds with `..` between them.

> Issues created in the 2010s

```qd
#issues created_at|year:2010..2019
```

The range `2010..2019` **includes** both 2010 and 2019. You can use `<` on either side of the `..` to make the range exclude either of the bounds. For example:

- `2010<..2019` means _"greater than 2010 and less or equal to 2019"._
- `2010..<2019` means _"greater or equal to 2010 and less than 2019"._
- `2010<..<2019` means _"greater than 2010 and less than 2019"._

A few details:

- A range is compared with `:`. The ordering and pattern operators (`:<`, `:~`, and so on) can't be used with one.
- Each bound is a **single value**: a literal (`2010`, `@2024-01-01`, `1w`), a [constant](#user-defined-constants) or built-in like `@now`, a column, or an expression in parentheses. The `..` binds more tightly than arithmetic and function pipes, so a computed bound needs parentheses: write `id:(1 + 1)..(2 * 3)` and `created_at:(@now|minus(1w))..@now`, not `id:1 + 1..2 * 3`.
- As everywhere on the [right-hand-side of a comparison](#bare-text-on-the-right-hand-side-of-a-comparison), a bare word is a string literal: `title:a..m` finds titles from "a" to "m". Use backticks for a column: ``created_at:`due_date`..@now``.
- A range is written directly on one side of a comparison. It can't be an entry of an [expansion](#comparison-expansion) (`id:[1 3..5]`), nor be wrapped in parentheses (`id:(1..5)`); both are syntax errors. To combine a range with other values, write the comparisons out: `[id:1 id:3..5]`.
- The other side of the comparison may be an expansion: `[created_at due_date]:@2024-01-01..@now`.


## Result columns

Result columns choose what a query outputs. Each one is an expression that follows a `$`, and they come last in the query, after any conditions.

> Show the id and title of each issue, along with its author's username

```qd
#issues $id $title $author.username
```

A result column can be any expression: a column, a [computation](#computations), a [count of related records](#aggregate-counts), and so on. A query without result columns outputs every column of its base table.

### Aliasing result columns

Use `->` after a result column to name it.

```qd
#issues $id->Identifier $title->Subject
```

The alias becomes the name of the column in the results. It's also the name by which a later [pipeline](#pipeline-of-multiple-queries) stage refers to the column.

### Basic sorting

To sort the results by a result column, add the `\s` flag after it. The `s` stands for "sort", and the sorting is ascending.

> Issues sorted by their creation date

```qd
#issues $title $created_at \s
```

To sort by a value without showing it, use a [sorting expression](#sorting-outside-of-result-columns) instead.

### Descending sorting

Add a `d` after the `s` to sort in descending order.

> Issues sorted by their creation date, most recent first

```qd
#issues $title $created_at \sd
```

### Multiple sorting

To sort by several columns, number the flags to say which column is sorted by first.

> Issues sorted by their creation date, most recent first, and then by title

```qd
#issues $title \s2 $created_at \sd1
```

Columns sorted without a number come after all the numbered ones, in the order they appear.

### Sorting NULL values

`NULL` values are sorted last, unless you add the `n` flag, which stands for "nulls first".

> Issues sorted by their creation date, most recent first, starting with the issues that have no creation date

```qd
#issues $title $created_at \sdn
```

### Grouping and aggregation

To group the results by a result column, add the `\g` flag after it. The other result columns then summarize each group with an aggregate function, applied with `%`.

> For each issue status, show the number of issues and the date of the most recently created issue

```qd
#issues $status \g $%count $created_at%max
```

- Every ungrouped column must contain an aggregate function. Otherwise the compiler reports an error.
- `%count` can stand on its own, which counts the rows of each group (SQL's `count(*)`).
- Aggregate functions like `%max`, `%sum`, and `%avg` may be applied to columns on the base table (e.g. `created_at%max`) or on a related table reachable via a single record (e.g. `author.created_at%max`), and to [values computed from such a column](#aggregating-a-computed-value) (e.g. `title|length%max`). A foreign key column is aggregated like any other column, e.g. `author%distinct` for the number of distinct authors.
- To group by several columns, give each the `\g` flag. As with [sorting](#multiple-sorting), you can number them: `\g1` and `\g2`. Columns without an explicit ordinal are grouped in the order they appear, after all columns that do specify an ordinal.
- Grouping and sorting can be combined, e.g. `#issues $status \g $%count \sd` to sort the groups by their counts.
- A [standalone sorting expression](#sorting-outside-of-result-columns) in a grouped query follows the same rule as a result column: it must either match a grouping column (`\\status` above) or contain an aggregate function (`\\created_at%max`). Otherwise the compiler reports an error.

- See the list of [all aggregate functions](./functions.md#aggregate-functions).

#### Grouping with related records

A count or other aggregate of [related records](#referencing-multiple-related-records), such as `#comments` or `#comments.created_at%max`, is computed separately for each record of the base table. So within a grouped query it behaves like a plain column, not like an aggregate: it has a value for each issue, not one for each group of issues. Using one as an ungrouped result column is an error:

```qd error
#issues $status \g $#comments
```

There are two things you can do with it instead. You can group by it:

> The number of issues having each number of comments

```qd
#issues $#comments \g $%count
```

Or you can aggregate it across each group, by first giving it a name with a [computed column](#computed-columns):

> For each status, the total number of comments on the issues with that status

```qd
#issues.comment_count = #comments
#issues $status \g $comment_count%sum
```

#### Filtering on an aggregate

Conditions choose which records get grouped and aggregated, so a condition can't itself use an aggregate of those records. Writing `#issues %count:>5 $status \g` is an error. To filter on an aggregate, compute it in one stage of a [pipeline](#pipeline-of-multiple-queries), and filter on it in the next:

> Statuses that have more than 5 issues

```qd
#issues $status \g $%count -> issue_count
~~~
issue_count:>5
```

This restriction only concerns aggregates of the records being queried. An aggregate of _related_ records is fine in a condition, e.g. `#issues #comments.created_at%max:<1w`.

### Column globs

`$*` stands for all the columns of the base table, which lets you add a column of your own alongside them.

> Show all the columns of each issue, and also the number of days until its due date

```qd
#issues $* $due_date|countdown|days
```

### Column glob on related table

A glob at the end of a path stands for all the columns of a related record.

> Show all the columns of each issue, and also all the columns of the issue's author

```qd
#issues $* $author.*
```

### Hiding columns within a glob

Parentheses after the `*` adjust particular columns of the glob. Within them, name a column and give it flags. The `\h` flag hides the column.

> Issues with all columns except description:

```qd
#issues $*(description \h)
```

The `\h` flag only has an effect within the parentheses of a glob. On an ordinary result column it does nothing, so `#issues $id \h $title` still shows the `id` column. To leave such a column out, simply don't list it. (And to sort by a value without showing it, use a [standalone sorting expression](#sorting-outside-of-result-columns).)

### Sorting columns within a glob

Within a glob's parentheses, the `\s` flag (with `d`, `n`, or a number, as [usual](#basic-sorting)) sorts by a column while leaving it in its place.

> Issues with all columns, most recently created first

```qd
#issues $*(created_at \sd)
```

### Nesting result columns

When several result columns share the same leading path, you can factor the path out: write it once, followed by `.(`, then the columns, then `)`.

> Comments, showing details of each comment's issue

```qd
#comments
$id @{hide:yes} // the comment id
$created_at
$issue.(
  $id @{hide:yes} // the issue id
  $title @{width:400}
  $#labels.name%list(\\name)
  $project.name
)
```

The path before `.( )` is applied to the start of every result column inside the parentheses, so the query above is the same as:

```qd
#comments
$id @{hide:yes} // the comment id
$created_at
$issue.id @{hide:yes} // the issue id
$issue.title @{width:400}
$issue.#labels.name%list(\\name)
$issue.project.name
```

Everything else about the nested columns — aliases, control flags like `\s` and `\h`, and annotations — works exactly as it does outside of nesting.

- Each nested column must begin with a column reference, so that the outer path has somewhere to attach. The reference may sit behind a pipe (`$title|upper` becomes `issue.title|upper`), arithmetic (`$id * 2`), a comparison (`$status:closed`), a `!` prefix, or a `++`/`--` prefix (`$++#labels` becomes `++issue.#labels`). A column that starts with none of these — e.g. a literal like `$8`, or a standalone `$%count` — cannot be nested.
- The path is applied **only** to that leading column reference. Any other column reference within the same nested column is left as written, relative to the base table: `#comments $issue.( $id + id )` is `issue.id + id`, which adds the comment's own `id` to the issue's. Write the path out where you need it again (`$id + issue.id`). This is unlike a [scoped comparison](#scoped-comparisons), in which everything inside the braces is relative to the related record.
- Groups can be nested within groups, and the paths compose: `$issue.( $project.( $name ) )` is the same as `$issue.project.name`.
- [Column globs](#column-globs) work inside a group: `$issue.( $project.* )` is the same as `$issue.project.*`.
- The path before `.( )` may traverse to-many relationships: `$#comments.( $created_at%max )` is the same as `$#comments.created_at%max`.


## Sorting outside of result columns

The [sorting flags](#basic-sorting) sort by a result column. To sort by a value without showing it, use a standalone **sorting expression**, written with the `\\` prefix.

> Issues sorted by their creation date, most recent first, without showing the creation date

```qd
#issues
\\created_at \d
$title
```

- Sorting expressions go between the conditions and the result columns.
- A query can have several. The first one listed is sorted by first.
- The `\\` prefix already means "sort", so there's no `s` flag here. After the expression you can add the `\d` (descending) and `\n` (nulls first) flags, or both as `\dn`. Without flags, sorting is ascending with NULL values last.
- Whitespace is allowed between the `\\` and the expression.

> Issues sorted by status, and then by creation date, most recent first, with issues lacking a creation date first of all

```qd
#issues
\\status
\\created_at \dn
$title
```

A sorting expression produces the same `ORDER BY` as a sorting flag does, without outputting a column. When a query has both, the sorting expressions are sorted by first.

### Scoped sorting expressions

When several sorting expressions share the same leading path, you can factor the path out: write it once, followed by `.(`, then the sorting expressions, then `)`.

> Show issues sorted by project status, project name, and issue due date

```qd
#issues
\\project.(
  \\is_active
  \\product.name
)
\\due_date
```

The path before `.( )` is applied to the start of every sorting expression inside the parentheses, so the query above is the same as:

```qd
#issues
\\project.is_active
\\project.product.name
\\due_date
```

Everything else about the nested expressions — the `\d` and `\n` flags — works exactly as it does outside of scoping.

- Each nested sorting expression must begin with a column reference, so that the outer path has somewhere to attach. The reference may sit behind a pipe (`\\name|upper` becomes `\\project.name|upper`), arithmetic (`\\id * 2`), a comparison (`\\status:closed`), a `!` prefix, or a `++`/`--` prefix (`\\++#labels` becomes `\\++project.#labels`). An expression that starts with none of these — e.g. a literal like `\\8`, or a standalone `\\%count` — cannot be scoped.
- As with [nested result columns](#nesting-result-columns), the path is applied only to that leading column reference. Any other column reference in the expression stays relative to the base table.
- Groups can be nested within groups, and the paths compose: `\\project.( \\product.( \\name ) )` is the same as `\\project.product.name`.
- The path before `.( )` may traverse to-many relationships: `\\#comments.( \\created_at%max )` is the same as `\\#comments.created_at%max`.


## Referencing _single_ related records

When a record refers to one record of another table through a foreign key, you can use that record's columns as if they were its own. Querydown writes the join for you.

### Single related records via column name chains

When a column links to another table, write a `.` after it to reach the columns of the related record. These chain, so you can follow one link after another.

> Issues created by members of the backend team, displaying the issue title and author's username

```qd
#issues author.team.name:"Backend" $id $title $author.username
```

### Scoped comparisons

When several conditions all reference the same related record, you can factor the path out: write it once, immediately followed by a condition set. This is the condition-side counterpart to [nesting result columns](#nesting-result-columns).

> Comments whose issue's title contains "dashboard"

```qd
#comments issue{title:dashboard}
```

The path before the condition set is applied to the start of every entry inside it, so the query above is the same as:

```qd
#comments issue.title:dashboard
```

The condition set's conjunction carries through: `{ }` combines its entries with `AND`, and `[ ]` combines them with `OR`.

> Comments whose issue's title **or** description contains "dashboard" — the same as `#comments [issue.title:dashboard issue.description:dashboard]`

```qd
#comments issue[title:dashboard description:dashboard]
```

Because the scope is evaluated against the related table, **anything you could write as a top-level condition on that table works inside the braces** — you can copy a condition written against `#issues` straight into `issue{ ... }`. In particular, a bare [default text search](#default-text-search) term searches the _related_ table's text columns:

> Comments whose issue matches "dashboard" in any of its text columns

```qd
#comments issue{dashboard}
```

This has no equivalent written with a path prefix (a bare search term has no column to prefix onto), which is why the scoping is resolved by the compiler rather than rewritten away.

A few details:

- There must be **no space** between the path and the condition set. With a space, `#comments issue {id:7}` is instead a [default text search](#default-text-search) for "issue" (on `#comments`) alongside a separate condition set, exactly as before.
- Scopes may be nested, and the paths compose: `issue{project{name:foo}}` is the same as `issue.project.name:foo`.
- The path steps through _single_ related records (a column-name chain). A path whose last step is a to-many table like `#comments{ ... }` is instead a [filter on those related records](#conditions-to-filter-aggregate-data), which is an existing, separate feature.
- **Everything** inside the braces is relative to the related record — column references on the right-hand side of a comparison included — and there is no syntax for referring back to the record outside. In ``#comments issue{author.username:=`user`.username}``, `user` is looked up on the issue (which has no such column), not on the comment. To compare the two records with each other, write the comparison without a scope:

    > Comments made by the author of their issue

    ```qd
    #comments issue.author.username:=user.username
    ```

## Referencing _multiple_ related records

When many records of another table refer to a record, you reach them by the name of their table, with its `#` sigil. Because there are many of them, you always get a summary of them: a count, or the result of some other aggregate function.

### Mandatory aggregation

In Querydown, unlike in SQL, all data from related records is aggregated with respect to the base table. So the results never have more rows than the base table does.

This makes queries simpler to write and to read, but it's also a limit: some SQL queries can't be written in Querydown. That's a deliberate trade. Querydown doesn't try to make _every_ query possible, only to make _most_ queries easier.

### Aggregate counts

A `#table` on its own counts the related records of that table.

> Show the number of issues associated with each project

```qd
#projects $name $#issues
```

In the [sample schema](./sample-schema.md), each project has many issues. When the base table is `#projects`, the expression `#issues` means: _the number of issues related to each project_.

A project with no issues shows `0`, not NULL.

### Specifying an aggregate function

To summarize related records some other way, name one of their columns and apply an aggregate function to it with `%`. This works like a [pipe](#function-piping).

> For each project, show the date of the most recently created issue

```qd
#projects $name $#issues.created_at%max
```

See the list of [all aggregate functions](./functions.md#aggregate-functions).

An aggregate function is applied to a column (reached by a path), or to a
[value computed from one](#aggregating-a-computed-value). It can't be applied to any other kind of
expression, such as `(#issues.id * 2)%sum`.

#### Aggregating a computed value

Scalar functions piped between the column and the aggregate function compute a value from each
record's column, and those values are what get aggregated.

> For each user, the average length of their comments

```qd
#users $username $#comments.body|length%avg
```

The `%` marks where the per-record computation ends: the functions before it apply to each
comment's `body`, and any functions after it apply to the aggregated result. So in
`#issues.title|length%sum|plus(1)`, each title's length is summed, and then 1 is added to the sum.

The value is computed exactly as if the pipeline were the definition of a
[computed column](#computed-columns) on the table that holds the column — here, as if `comments` had
a computed column `body|length`. That decides what the functions' arguments refer to: a column
named in an argument is a column of that same record.

> For each project, the most days any of its issues was given before it's due

```qd
#projects $name $#issues.due_date|minus(created_at)|days%max
```

Here `created_at` is each issue's, not the project's. Likewise, after a to-one link, as in
`#issues.author.username|concat(email)%list`, an argument is a column of the record the link leads
to (here, the author). To reach another table, use a path from there. Unlike a computed column's
definition, though, the pipeline sees the [variables](#user-defined-functions) around it, which
keep the meaning they had where they were bound.

Aggregating a computed value works everywhere aggregating a column does: across a to-many path
(including a [filtered one](#conditions-to-filter-aggregate-data), and one that continues through a
to-one link), in a condition (`#projects #issues.created_at|year%distinct:>2`), and in a
[grouped query](#grouping-and-aggregation) (`#issues $status \g $title|length%max`). A few more
things to know:

- [Sorting by the aggregated value](#sorting-by-the-aggregated-value) with `\s` sorts by the
  computed value. A sorting expression that names the column itself, like `\\body`, sorts by the
  column instead, so it doesn't count as a sort by the aggregated value.
- `%count` counts the records whose computed value isn't NULL.
- The computation can't use an aggregate of the records being aggregated, such as `title%max` in
  `title|concat(title%max)%list`. An aggregate of records related to each one, like
  `#issues.title|concat(#comments)%list`, is fine.
- Only functions piped onto the column can compute the value. Arithmetic operators can't, but the
  equivalent functions can: write `#issues.id|times(2)%sum` rather than
  `(#issues.id * 2)%sum`. For anything else, define a [computed column](#computed-columns) and
  aggregate that.

#### Arguments to an aggregate function

One aggregate function, [`percentile`](./functions.md#percentile), takes an argument, which goes in
parentheses after its name, like a scalar function's. Its argument is the fraction of the values
that the result is above: `0.5` for the median.

> For each issue, the median length of its comments

```qd
#issues $title $#comments.body|length%percentile(0.5)
```

The same parentheses hold an aggregate function's [sorting entries](#sorting-within-an-aggregate-function),
which begin with `\`. Giving an argument to any other aggregate function is an error, and so is
giving one to an aggregate function that's used on its own, without a value before it, like
`%count`.

### Sorting within an aggregate function

Some aggregate functions (notably [`list`](./functions.md#list) and
[`list_all`](./functions.md#list_all)) produce a result whose order is meaningful. You can control
that order by attaching one or more sorting entries inside the function's parentheses. Each entry
is either a [standalone sorting expression](#sorting-outside-of-result-columns) (written with the
`\\` prefix) or the `\s` [column flags](#basic-sorting). (The parentheses can hold an
[argument](#arguments-to-an-aggregate-function) too, which is told apart by not beginning with `\`.)

> For each issue, list its label names in alphabetical order

```qd
#issues $title $#labels.name%list(\\name)
```

As with standalone sorting expressions, `\\` entries may be followed by `\d` (descending) and `\n`
(nulls first) flags, and when several entries are given, the first takes precedence.

#### Sorting by the aggregated value

To sort by the very value being aggregated, use the `\s` column flag, just as you would to sort a
result column. Combine it with `d` and `n` as usual: `\sd`, `\sn`, `\sdn`.

> For each issue, list its label names in reverse alphabetical order

```qd
#issues $title $#labels.name%list(\sd)
```

This is the same as writing out a sorting expression that reaches the aggregated column — here
`\\name \d` — but it saves you from repeating the path.

#### The context of a sorting expression

A sorting expression attached to an aggregate is evaluated **from the context of the first `#` table
in the aggregated path**, not from the path's final column. This lets you sort by a column of an
intermediate join table that the path passes through.

> For each issue, list all its label names, ordered by the join table's own `id` column

```qd
#issues $title $#issue_labels.label.name%list_all(\\id)
```

Here the path begins with `#issue_labels`, so `\\id` refers to `issue_labels.id`. To sort by a column
that is reached by following a path out of that first `#` table, write the full path in the sorting
expression — for example `\\label.name` sorts by the related label's name. (Since `label.name` is the
column being aggregated, reached the same way, that's a sort by the aggregated value, equivalent to
`\s`.)

#### Sorting distinct values

[`list`](./functions.md#list) removes duplicate values, which affects how it can be sorted. Take this
query, which lists the users who have commented on each issue:

```qd
#issues $title $#comments.user.username%list
```

If a user has commented on an issue three times, their username appears just once in the list. But
those three comments each have their own `created_at`, so it's not meaningful to sort the list by
`\\created_at` — which of the three dates would decide where the username goes? For that reason,
every sorting entry within `list` must be one of the following:

- **A sort by the listed value itself**, written as `\s` (or as a path to it, like `\\user.username`
  above).

  > For each issue, list the users who have commented on it, alphabetically

  ```qd
  #issues $title $#comments.user.username%list(\s)
  ```

- **An aggregated sorting expression**, which summarizes the records that share each value. It's
  written like any other [aggregate](#specifying-an-aggregate-function) and is computed
  separately for each distinct value, from the records (here, the comments) behind that value.

  > For each issue, list the users who have commented on it, in the order in which they first
  > commented

  ```qd
  #issues $title $#comments.user.username%list(\\created_at%min)
  ```

  > ... with the most recently active commenter first

  ```qd
  #issues $title $#comments.user.username%list(\\created_at%max \d)
  ```

  > ... with the most frequent commenter first, breaking ties alphabetically

  ```qd
  #issues $title $#comments.user.username%list(\\%count \d \s)
  ```

Sorting `list` by anything else (such as `\\created_at`) is an error. If you want to sort by each
individual record and you don't mind duplicates, use [`list_all`](./functions.md#list_all) instead,
which accepts any sorting expression.

Aggregated sorting expressions have a couple of limitations:

- They can only be used within `list` and [`top`](#keeping-only-the-top-values). Other aggregate
  functions don't remove duplicates, so they have no use for them.
- They're only supported when listing values from related records via a `#` path, as above, and not
  when listing a column of [grouped](#grouping-and-aggregation) records (e.g.
  `#issues $status \g $title%list(\\id%min)`). Sorting grouped records by the listed value
  (`$title%list(\s)`) works fine.

#### Keeping only the top values

The [`top`](./functions.md#top) function is like `list`, except that it keeps only the values that
tie for first place under its sorting entries.

> For each issue, show the users who made the most comments on it

```qd
#issues $title $#comments.user.username%top(\\%count \d)
```

If Alice made 5 comments on an issue, Bob made 5, and Charlie made 4, then this shows `[Alice,Bob]`
for that issue. Compare this with `%list(\\%count \d)`, which would show all three users, most
frequent first.

`top` accepts the same sorting entries as `list`: sorts by the value itself (`\s`) and aggregated
sorting expressions, in any combination. At least one is required. Values tie only if they tie on
_every_ sorting entry, so each extra entry acts as a tiebreaker:

> For each issue, show the users who made the most comments on it, keeping only those among them
> who commented most recently

```qd
#issues $title $#comments.user.username%top(\\%count \d \\created_at%max \d)
```

Because they all tie, the values are collected in ascending order. An issue with no comments shows
`null`, just as it would with `list`.

`top` has the same limitation as aggregated sorting expressions: it's only supported for values from
related records via a `#` path, and not for a column of [grouped](#grouping-and-aggregation) records.

### "Has some" and "has none" conditions

`++` before a `#table` is a condition that's true when there's at least one related record, and `--` when there are none.

> Projects that have at least one issue

```qd
#projects ++#issues
```

This is short for `#projects #issues:>0`.

> Projects that have no issues

```qd
#projects --#issues
```

This is short for `#projects #issues:0`.

### Conditions to filter aggregate data

A condition set directly after a `#table` filters the related records before they're counted or aggregated.

> Projects that have no open issues

```qd
#projects --#issues{status:"open"}
```

Inside the block, everything is relative to the related table (here `issues`), just as it would be in a query whose base table is `#issues`. A few details:

- The block belongs to the `#table` that it **directly** follows. A block written after a column is not a filter on the related records: `#issues ++#comments.user{id:5}` is the condition `++#comments.user` followed by a separate condition set, `{id:5}`, which applies to the issue. To filter the comments by a column of their user, put the path inside the comments' block instead:

    > Issues that have a comment by user 5

    ```qd
    #issues ++#comments{user.id:5}
    ```

- The block can't refer to the base record by its own column names. But each related record links back to it through a foreign key column, which you can follow like any other:

    > For each issue, count the comments made more than a week after the issue was created

    ```qd
    #issues $id $#comments{created_at:>issue.created_at|plus(1w)}
    ```

- The block can't use an aggregate of the very records it filters (e.g. `#issues{%count:>5}`), since it decides which records those are. An aggregate of records related to _them_ is fine, e.g. `#projects ++#issues{#comments:>5}`.

### Transitive relationships

A `#table` can name a table that's related only through other tables. Querydown finds the way there.

> Show the number of issues associated with each client

```qd
#clients $name $#issues
```

The `issues` table isn't directly related to the `clients` table, but that's okay. The query above is short for this one:

```qd
#clients $name $#products.#projects.#issues
```

The shorthand works because there's only one way to get from `clients` to `issues`. **Querydown chooses the shortest path to many records of the table, as long as there's only one such path.**

### Related tables with single vs multiple records

A `#table` always means _many_ related records. Where the schema allows only one related record, it's an error.

> Attempt to show the number of projects associated with each issue

```qd error
#issues $title $#projects
```

Each issue has just one project, so there's nothing to count. To reach the _single_ related project, use a [column name chain](#single-related-records-via-column-name-chains) instead:

> Show the project name for each issue

```qd
#issues $title $project.name
```

To summarize:

- `#projects` refers to a related `projects` table with **many** records.
- `project` (the column that links to the `projects` table) refers to the **one** related `projects` record.

_(A `>>projects` syntax, for referencing a single related record by its table name instead of by the linking column, is reserved but not yet supported. Using it is an error.)_

### Ambiguous paths

If more than one path to the related table ties for being the shortest, the `#table` is ambiguous, and the compiler reports an error.

> Attempt to show the number of users associated with each issue

```qd error
#issues $id $title $#users
```

From `issues`, there are two ways to reach _many_ `users` records: through the `assignments` table, and through the `comments` table. Each takes one table in between, so neither is shorter, and the compiler can't choose for you. (The `author` column leads from an issue to a user too, but only to [one](#related-tables-with-single-vs-multiple-records).)

Name the table in between to say which you mean.

> The number of users _who are assigned_ to each issue

```qd
#issues $id $title $#assignments.#users
```

> The number of distinct users _who have commented_ on each issue

```qd
#issues $id $title $#comments.#users.id%distinct
```

### Intersecting paths

A path can pass through the same table more than once, and can even come back to the table it started from. Each `#` part of the path finds its own route, so the path as a whole may loop back on itself.

> Issues, showing the most recent creation date among all the issues that share a label with each issue

```qd
#issues $* $#labels.#issues.created_at%max
```

Here, `#labels` goes from `issues` through `issue_labels` to `labels`. Then `#issues` goes back through `issue_labels` to `issues`. An issue shares its labels with itself, so its own creation date counts too. An issue with no labels shows `null`.

Column name chains can loop back too:

> Issues, showing the most recent creation date among all the issues that each issue's author has commented on

```qd
#issues $* $author.#comments.issue.created_at%max
```

This path goes from `issues` to `users`, then to `comments`, and then back to `issues`.

When Querydown finds the route for a single `#` part, it never visits the same table twice, with one exception: the route may end by following a foreign key that refers to its own table. In our schema, `issues.duplicate_of` refers to `issues`, so a `#issues` part can lead from an issue to the issues marked as duplicates of it:

> Issues, showing the most recent creation date among the issues marked as duplicates of each issue

```qd
#issues $* $#issues.created_at%max
```

`duplicate_of` is the only foreign key from `issues` to itself, so it's the shortest unambiguous route and you don't need to name it. You can still name it, as in `#issues(duplicate_of)` (see [Specifying the linking column](#specifying-the-linking-column)).

A table without such a foreign key can't lead back to itself this way:

> Attempt to show, for each user, the number of "related" users

```qd error
#users $* $#users
```

There are many ways to get from one user to other users: the same team, the same issues, and so on. Name a table in between to say which one you mean:

> Users, showing the number of users on each user's team

```qd
#users $* $team.#users
```

### Specifying the linking column

If a table refers to another table through more than one foreign key, then a `#` part leading to it is ambiguous. Put the foreign key column in parentheses to say which one to follow.

> Issues that are not blocking any other issues

```qd
#issues --#blocks(blocker)
```

> Issues that are not blocked by any other issues

```qd
#issues --#blocks(blocking)
```

The linking column applies to the last link of the route. Querydown still finds the shortest route, but only among routes that end by following that foreign key:

> Issues, listing the titles of issues marked as duplicates of issues that share a label with each issue

```qd
#issues $* $#labels.#issues(duplicate_of).title%list
```

Without `(duplicate_of)`, `#labels.#issues` would end at the issues that share a label, reached via `issue_labels.issue`. Naming `duplicate_of` adds one more hop, from those issues to the issues marked as their duplicates.

## Pipeline of multiple queries

A query can have several **stages**, separated by `~~~`. Each stage reads the result of the stage before it, instead of a table from the schema. This lets you do what a single query can't, such as filtering or aggregating the result of an earlier aggregation.

> For each project, count the number of months in which at least 10 issues were created

```qd
#issues $project \g $created_at|year_month \g $%count -> issue_count
~~~
issue_count:>=10 $project \g $%count
```

Each stage is compiled to an SQL [common table expression](https://www.postgresql.org/docs/current/queries-with.html) (CTE), and the next stage selects from that CTE as its base "table". The columns available to a stage are exactly the result columns produced by the previous stage:

- A result column with an alias (e.g. `-> issue_count`) is referenced in the next stage by that alias.
- A result column that is a plain column reference (e.g. `$project`) keeps its column name.

The query above compiles to:

```sql output
WITH
  "pipe0" AS (
    SELECT
      "issues"."project" AS "project",
      to_char("issues"."created_at", 'YYYY-MM') AS "column_2",
      count(*) AS "issue_count"
    FROM "issues"
    GROUP BY "issues"."project", to_char("issues"."created_at", 'YYYY-MM')
  )
SELECT
  "pipe0"."project",
  count(*)
FROM "pipe0"
WHERE
  "pipe0"."issue_count" >= 10
GROUP BY "pipe0"."project";
```

A few things to know:

- A stage's output columns become plain columns of the next stage's base table. They have no relationships, so you can reference them directly but cannot traverse to related records from them. Using a path like `#comments` in a later stage is an error: compute what you need from related records in the first stage, and output it as a column.
- The intermediate columns are mostly untyped. As a result, the match operator (`:`) compares them using exact equality rather than type-aware matching (e.g. case-insensitive "contains" for text).
- The exception is dates, datetimes, and durations, which keep their types from one stage to the next. So [comparing a date with a duration](#comparing-dates-with-durations) means the same thing in a later stage as in the first: `created_at:<1w` finds the rows whose `created_at` lies within a week of now.
- Only the [column annotations](#column-level-annotations) of the last stage appear in the output. An annotation written in an earlier stage is dropped, even when its column passes through the later stages unchanged.


## Union of multiple queries

The `+++` operator appends the results of one query to the results of another, like SQL's `UNION ALL`.

> List all the issues related to issue 1234, along with the way in which they are related

```qd
#issues duplicate_of:1234
$id
$title
$created_at
$"Duplicate"
+++
#blocks blocker:1234
$blocking.id
$blocking.title
$blocking.created_at
$"Dependent"
+++
#blocks blocking:1234
$blocker.id         -> id
$blocker.title      -> title
$blocker.created_at -> created_at
$"Dependency"       -> relationship
~~~
$*(created_at \s)
```

Each query in a union has its own base table, conditions, and result columns. The definitions written before the first query (constants, functions, user-defined tables, and so on) are available to all of them.

A few things to know:

- Each query in a union must have the same number of result columns, and the types of the columns must be compatible across the queries.
- Column names are taken from the last query in a union. Where the last query leaves a column unnamed (a computed column without an alias, e.g. `$title|length`), the first query's name for that column is used instead.
- Duplicate rows are kept, as with SQL's `UNION ALL`. To remove them, group by every column in a [pipeline](#pipeline-of-multiple-queries) stage, e.g. `~~~ $id \g $title \g`.
- The queries in a union can't be sorted, because a union doesn't keep the order of its queries' rows. To sort the result of a union, sort it in a [pipeline](#pipeline-of-multiple-queries) stage, as in the example above.
- Union has higher precedence than pipeline (the union will be performed before the pipeline). [User-defined tables](#user-defined-tables) can be used if you need a pipeline within a union.
- [Column annotations](#column-level-annotations) may be written in any query of a union. If more than one query annotates the same column, the last annotation wins.


## Window functions

A **window function** computes a value across a set of rows related to the current row, without
collapsing those rows the way a [grouped aggregate](#grouping-and-aggregation) does.

A window is defined via `%%( )`. Inside the parentheses, you use the same syntax as with [column
globs](#column-globs), with two relevant flags:

- `\p` marks a **partition** expression (`PARTITION BY`). An entry with no flag is also treated as a
  partition expression.
- `\s` marks an **ordering** expression (`ORDER BY`), and accepts the usual `\d` (descending), `\n`
  (nulls first), and ordinal modifiers described under [sorting](#multiple-sorting).

After the window definition, `%func` applies the window function. Any **value arguments** are
supplied in parentheses after the function name, space-separated (the same convention as [function
piping](#function-piping)). Ranking functions like `row_number` take no value argument.

> Number each issue within its project, ordered by creation date:

```qd
#issues $id $title $%%(project\p created_at\s)%row_number -> rn
```

> A running count of issues within each project, in creation-date order:

```qd
#issues $id $%%(project\p created_at\s)%count -> running_count
```

> The previous issue's status within each project (offset `1`, defaulting to `"none"`):

```qd
#issues $id $%%(project\p created_at\s)%lag(status 1 "none") -> previous_status
```

_(See a list of [all window functions](./functions.md#window-functions).)_

### Filtering on a window function

SQL does not permit window functions in a `WHERE` clause. When you use one in a condition, Querydown
automatically computes the window value in a subquery and applies your filter to the result.

> The most recent issue in each project (the classic "top row per group"):

```qd
#issues %%(project\p created_at\sd)%row_number:1 $id $title $project
```

This compiles to a query that computes `row_number() OVER (…)` in a subquery (a CTE) and then filters
`= 1` in the outer query, which reads from the subquery in place of the base table. The window
function is computed over the rows that pass the stage's ordinary (non-window) conditions, which are
applied within the subquery. Everything else about the stage then works just as it would without the
window condition, including [grouping](#grouping-and-aggregation): the window condition filters the
rows before they're grouped.

> For each project, count the issues that were the most recent issue assigned to some user:

```qd
#assignments %%(user\p issue.created_at\sd)%row_number:1 $issue.project \g $%count
```

A window function must be written directly among the stage's conditions. One reached any other way,
such as through a [computed column](#computed-columns), a [custom comparison](#custom-comparisons),
or a [scoped comparison](#scoped-comparisons), is an error.

Equivalently, you can compute the window value as a column in one [pipeline](#pipeline-of-multiple-queries)
stage and filter it as a plain column in the next stage:

```qd
#issues $id $title $project $%%(project\p created_at\sd)%row_number -> rn
~~~
rn:1 $id $title $project
```

### Window function limitations

- There is no syntax for an explicit **frame clause** (`ROWS`/`RANGE BETWEEN …`); windows use each
  function's SQL default frame. Note in particular that `last_value` and `nth_value` use the default
  frame ending at the current row.
- A single stage cannot combine `\g` [grouping](#grouping-and-aggregation) with a window function in
  its result columns or sorting. Use a [pipeline](#pipeline-of-multiple-queries) to group the output
  of a window stage. (A window function in the stage's _conditions_ is fine, as described
  [above](#filtering-on-a-window-function).)
- Window functions can't be used within an aggregate function, within another window function, or
  with [related records](#referencing-multiple-related-records) (e.g. in a filter like
  `#comments{…}`, or in a computed column that's aggregated across a to-many relationship). In each of
  these cases, compute the window value as a result column in one stage of a
  [pipeline](#pipeline-of-multiple-queries), and use it in the next stage.

## Variables

Definitions give names to values, computations, conditions, and whole queries. They all go at the start, before the base table, and the query after them can use what they define. An application that embeds Querydown can also supply definitions of its own ahead of each query.

### User-defined constants

A constant names a value, which you can then use as `@name` anywhere in the query.

> Show the issues created by user 1234

```qd
@user_id = 1234
#issues author:@user_id
```

A constant is defined with `@name = expr`. Its value is **inlined** into the generated SQL wherever
you refer to it. A constant's definition can refer to constants defined before it.

A constant stands for its value, so a value with a special meaning keeps that meaning when it
arrives through a constant:

```qd
@recent = 1w
@none = @null
#issues created_at:<@recent description:@none
```

Here `created_at:<@recent` is a [comparison of a date with a duration](#comparing-dates-with-durations),
and `description:@none` finds the issues with no description, exactly as if `1w` and `@null` had
been written in place. Likewise, a constant holding a string is a
[default text search](#default-text-search) term when it stands alone as a condition.

A constant holds a **single** value (or condition). It can't hold a set of values with which to
[expand a comparison](#comparison-expansion): only a set written directly in a comparison expands
it. Given `@statuses = [open reopened]`, the comparison `status:@statuses` is an error. (Standing
alone, `@statuses` is an ordinary [OR condition set](#or-condition-sets), in which the bare words are
search terms: `#issues @statuses` finds the issues that mention "open" or "reopened" anywhere.)

### Defining a constant using the result of a query

A constant can hold the result of a query that produces a single value. Write the query within
`#( )`.

> Find issues created after the most recent comment was created

```qd
@date_of_latest_comment = #( #comments $created_at%max )
#issues created_at:>@date_of_latest_comment
```

The query is inlined as a scalar subquery (`(SELECT … )`) wherever you refer to the constant.

### Computed columns

A computed column is a named expression that belongs to a table. Once you define it, with
`#table.name = expr`, you can use it by name like a real column of that table: in result columns,
in conditions, and in the definitions of later computed columns.

> Show each user, along with whether they are old enough to buy alcohol

```qd
#users.age = birth_date|age|years|floor
#users.can_purchase_alcohol = age:>=21
#users $* $can_purchase_alcohol
```

Here `can_purchase_alcohol` references the earlier `age` computed column, which in turn references
the real `birth_date` column. A computed column hosted on a related table can be reached across a
to-one relationship, e.g. `#issues $author.can_purchase_alcohol`.

A computed column can also be [aggregated](#specifying-an-aggregate-function) like a real column,
including across a to-many relationship. Its definition is evaluated once for each related record,
and the results are aggregated:

> Show each project alongside the furthest stage reached by any of its issues

```qd
#issues.stage = ? #assignments:>0~2 !description:@null~1 ~~0
#projects $name $#issues.stage%max
```

The definition may itself reach other tables, as `stage` does to count each issue's assignments.
Without an aggregate function, `$#issues.stage` is an error, just as `$#issues.title` would be.

For a computation needed only once, you can also write it inline, piping the column through scalar
functions before the aggregate function: see
[aggregating a computed value](#aggregating-a-computed-value).

### User-defined functions

A function names a computation, which you can then apply to different values.

> Given a fiscal year that begins on February 1st, find issues that were opened in fiscal year 2020 and are due in fiscal year 2021

```qd
@@fiscal_year = @date => (@date - 1m)|year
#issues created_at|fiscal_year:2020 due_date|fiscal_year:2021
```

A function is defined with `@@name = @param1 @param2 => body`. It behaves like a built-in scalar
function: you can apply it with a pipe (`value|name`), possibly with more arguments
(`value|name(extra)`), where the piped-in value is the first argument. When applied, the
arguments are bound to the parameters and the body is **inlined** into the generated SQL. (Built-in
functions take precedence over a user-defined function of the same name.)

Although the body is inlined, each piece of it keeps the meaning it had where it was written:

- The function's **body** sees its own parameters and the [constants](#user-defined-constants), but
  not the variables of whichever expression calls it. Column names within the body refer to the
  table that the function is called on.
- An **argument** is evaluated where the function is called, not where the body uses the parameter.
  That matters when the body uses a parameter within a
  [filter on related records](#conditions-to-filter-aggregate-data):

    > Find issues that their own author has commented on

    ```qd
    @@has_comment_by = @u => ++#comments{user:@u}
    #issues author|has_comment_by
    ```

    Here `author` is a column of the issue, even though `@u` is used inside `#comments{ }`, where a
    column name written directly would be looked up on the comment.

One limitation: an argument that refers to its record like this (to its columns, or to records
related to it) can be used within a filter on that record's related records, but no deeper. It can't
be used within a filter that is nested inside another filter, nor within a filter on the related
records of some other record (as in `author.#comments{ }`). The compiler reports an error if it is.
An argument that doesn't refer to its record, such as a literal, can be used anywhere.

### Function with multiple parameters

A function can take several parameters, separated by spaces.

> Show each issue's title along with the number of days until it's due, clamped between 0 and 30

```qd
@@clamp = @value @low @high => @value|max(@low)|min(@high)
#issues $title $due_date|countdown|days|clamp(0 30)
```

- Arguments are separated by spaces too.
- The piped-in value (`due_date|countdown|days`) is bound to the first parameter (`@value`), and the parenthesized arguments are bound to the rest, in order (`@low` = `0`, `@high` = `30`).
- The function can also be [called](#function-calling) without a pipe, passing all arguments in parentheses: `$@@clamp(due_date|countdown|days 0 30)`.
- The number of arguments must match the number of parameters, otherwise the compiler reports an error.

### Function containing an assignment

A function's body can begin with local assignments (`@name = expr`), which name intermediate
values for the final expression to use. Each one can refer to the function's parameters and to the
assignments before it.

> Count the users of each generation

```qd
@@generation = @birth_date =>
  @birth_year = @birth_date|year
  ? @birth_year:>=2010 ~ "Alpha"
    @birth_year:>=1997 ~ "Z"
    @birth_year:>=1981 ~ "Millennial"
    @birth_year:>=1965 ~ "X"
    @birth_year:>=1946 ~ "Boomer"
    @birth_year:>=1928 ~ "Silent"
    @birth_year:>=1901 ~ "Greatest"
    @birth_year:>=1883 ~ "Lost"
    ~~ @null

#users $birth_date|generation \g $%count
```

### Custom comparisons

A custom comparison gives a meaning to comparing a made-up name against a value. You use the name like a column on the left of a comparison, and it expands to conditions of your choosing.

> Find issues that have a comment containing the word "workaround"

```qd
#issues.comment:@x = ++#comments{body:@x}

#issues comment:workaround
```

A custom comparison is defined with `#table.name:@parameter = condition`. An application that embeds Querydown might define some ahead of each query, as a convenience for its users.

If the definition uses the match operator (`:`), and every comparison within it does too, then you can use the custom comparison with any other operator, which takes the place of each `:`.

> Find issues that have a comment matching a regular expression

```qd
#issues.comment:@x = ++#comments{body:@x}

#issues comment:~"work[ -]?around"
```

> Find issues that have a comment exactly equal to "+1"

```qd
#issues.comment:@x = ++#comments{body:@x}

#issues comment:="+1"
```

If the definition uses any other operator, then the custom comparison must be used exactly as it was defined.

> Find issues that the user with the exact username "david" has commented on, authored, or been assigned to

```qd
#issues.participant:@x = [
  ++#comments{user.username:=@x}
  ++#assignments{user.username:=@x}
  author.username:=@x
]

#issues participant:david
```

With this definition, `participant:~"david"` is an error.

A few more things to know:

- A custom comparison is used in place of a column on the left-hand side of a comparison, which
  includes the entries of a left-hand [expansion](#comparison-expansion): `[comment title]:workaround`
  finds issues with "workaround" in a comment or in the title.
- The right-hand side may be an expansion too (`comment:[workaround hack]`), but not a
  [range](#ranges).
- The value on the right-hand side is evaluated where the comparison is used, in the same way as an
  [argument to a function](#user-defined-functions). So ``comment:`title` `` finds issues having a
  comment that mentions the issue's own title: `title` is a column of the issue, even though the
  definition uses `@x` within `#comments{ }`.

### Linked record comparisons

A linked record comparison defines what it means to compare a link to a table's records against a
value, so that you don't have to name a column of the linked record.

Normally, to find issues authored by "alice", you reach through the `author` link to the `username`
column:

```qd
#issues author.username:alice
```

To make `author:alice` mean the same thing, define a [custom comparison](#custom-comparisons) named
`__querydown_linked_record_comparison` on the linked table.

```qd
#users.__querydown_linked_record_comparison:@x = username:@x

#issues author:alice
```

With this definition, `author:alice` means exactly the same thing as
`author.username:alice`. It works for any to-one link that lands on the `users` table, including
multi-hop chains such as `duplicate_of.author:alice`.

Because it is an ordinary custom comparison, everything that applies to custom comparisons applies
here too. If the body uses only the match operator (`:`), the comparison can be [called with a
switched operator](#custom-comparisons), so `author:=alice` and `author:~"^al"` both work. The body
can also be any expression — for instance, matching on either of two columns:

```qd
#users.__querydown_linked_record_comparison:@x = [username:@x email:@x]

#issues author:alice
```

An application that embeds Querydown might supply these definitions ahead of each query, so that
comparing against a linked record just works, without its users needing to know which column to
match.

Once a linked record comparison is defined for a table, it applies to **every** comparison made
directly against a link to that table. That includes comparisons that would otherwise have compared
the foreign key itself: with the definition above, `author:5` no longer finds the issues whose
`author` column is 5, because it now compares the author's username. To compare the key, name the
column it refers to:

```qd
#users.__querydown_linked_record_comparison:@x = username:@x

#issues author.id:5
```

`author:@null` goes through the comparison as well, becoming `author.username:@null`. That still finds
the issues with no author (which have no username to speak of), along with any whose author has no
username.

### User-defined tables

A user-defined table names a query, whose results you can then query as if they were a table. It's
defined with `#name = #( query )`.

> For each project, count the number of months in which at least 10 issues were created

```qd
#project_months = #(
  #issues $project \g $created_at|year_month \g $%count -> issue_count
)
#project_months issue_count:>=10 $project \g $%count
```

- The name can be the base table of the query, or of a later user-defined table.
- The result columns of the inner query are the columns of the table.
- As with the result of a [pipeline](#pipeline-of-multiple-queries) stage, those columns have no
  relationships, and only the dates, datetimes, and durations among them keep their types.
- A user-defined table compiles to a common table expression (CTE).

## Annotations

Annotations attach data of your own to a query. Querydown doesn't interpret them. It passes them through to the application, alongside the SQL, for the application to use as it sees fit: for column widths, number formats, a choice of chart, and so on.

### Column-level annotations

To annotate a result column, write an object prefixed with a `@` sigil **last** in the column, after any flags and alias.

> Show several columns of each issue, with a display annotation on each

```qd
#issues
$title @{width:100}
$created_at @{formatter:timeElapsed textColor:light}
$due_date @{format:'YYYY-MM-DD' datePicker:@true}
$#comments @{formattingConditions:[
  {gte:10 bg:'#fbc9ff'}
  {gte:5 bg:'#d5d2ff'}
]}
```

Along with the SQL, compiling this query produces these annotations:

```json output
{
  "columnAnnotations": [
    { "width": 100 },
    { "formatter": "timeElapsed", "textColor": "light" },
    { "format": "YYYY-MM-DD", "datePicker": true },
    {
      "formattingConditions": [
        { "gte": 10, "bg": "#fbc9ff" },
        { "gte": 5, "bg": "#d5d2ff" }
      ]
    }
  ]
}
```

The structure of an annotation is entirely up to you (or to the application you're writing queries for).

#### The annotation output

The `columnAnnotations` array is **positional**: it has one entry per output column, in the same order as the columns in the result set. A column with no annotation gets `null` in its slot. This means a column glob like `$*` contributes one `null` (or its adjustment annotation) per expanded column, and columns [hidden from a glob](#hiding-columns-within-a-glob) (`\h`) contribute nothing.

The annotations describe the columns of the final result, so in a [pipeline](#pipeline-of-multiple-queries) only those written in the **last** stage are output. An annotation on a column of an earlier stage is dropped, even when a later stage passes the column through unchanged with `$*`. Annotate the column in the last stage instead, which a glob lets you do without listing the other columns:

```qd
#issues $id $title $#comments -> comments
~~~
comments:>5 $*(title @{width:400})
```

(In a [union](#union-of-multiple-queries), by contrast, an annotation may be written in any of the queries, since they all describe the same result columns.)

#### Querydown's JSON variant

Annotations are written in a JSON-like syntax with a few differences from standard JSON:

- Object entries and array elements are delimited with **whitespace** instead of commas.
- Strings can be left **unquoted** if they are identifiers (e.g. `timeElapsed`). They can also be quoted with single or double quotes.
- The values `null`, `true`, and `false` are written with a `@` sigil: `@null`, `@true`, `@false`.
- Only the top-level annotation object takes the `@` sigil. Nested objects do not.

A few things to watch out for:

- Bare `true`, `false`, and `null` (without the `@` sigil) are parsed as the **strings** `"true"`, `"false"`, and `"null"`.
- Inside an annotation, the `@` sigil accepts **only** `true`/`false`/`null` — not dates (`@2000-01-01`) or other constants.
- A value that begins with a digit is parsed as a number, and a value that begins with `#` (such as a color like `#fbc9ff`) must be quoted.

### Query-level annotations

To annotate the query as a whole, write an object prefixed with `\\\` at the very **end** of the query, after the result columns, and after the last stage of a [pipeline](#pipeline-of-multiple-queries).

> Show all issues, and annotate the query with `{"foo": 100}`

```qd
#issues \\\{"foo": 100}
```

Along with the SQL, compiling this query produces this annotation:

```json output
{
  "queryAnnotation": { "foo": 100 }
}
```

`queryAnnotation` is `null` for a query without one. As with column annotations, the structure is entirely up to you, and the object is written in [Querydown's JSON variant](#querydowns-json-variant) — so it can span several lines and drop the quotes and commas:

```qd
#issues status:open
$title @{width:400}
$created_at
\\\{
  view:board
  groupBy:status
  pageSize:50
}
```

A few rules:

- The `{` must directly follow the `\\\`.
- Only one query annotation is allowed, and nothing but whitespace and comments may follow it.
- A [subquery](#user-defined-tables) (`#( … )`) can't have its own query annotation.

## Limit and offset

There's no syntax for limiting the number of rows. Limits and offsets are options to the compiler instead. That leaves pagination to the _application_ rather than to the query's author: the same query can be compiled once for each page, and nobody can write a query that fetches more rows than the application allows.

| Option   | SQL      | Effect                                           |
| -------- | -------- | ------------------------------------------------ |
| `limit`  | `LIMIT`  | The maximum number of rows to return             |
| `offset` | `OFFSET` | The number of rows to skip before returning any  |

Both are optional non-negative integers, and you can use either without the other. How you pass them depends on how you run the compiler:

- [Rust](./setup/rust.md#options): the `limit` and `offset` fields of `Options`
- [JavaScript](./setup/javascript.md#compile): an options object as the last argument to `compile` or `compile_sections`, e.g. `compile(schema, "postgres", query, { limit: 50, offset: 100 })`
- [CLI](./setup/cli.md#compile-a-query): `querydown compile --limit 50 --offset 100 …`

> With a limit of `50` and an offset of `100`, the third page of issues, oldest first

```qd limit=50 offset=100
#issues \\created_at \\id $title
```

```sql output
SELECT
  "issues"."title"
FROM "issues"
ORDER BY
  "issues"."created_at" ASC NULLS LAST,
  "issues"."id" ASC NULLS LAST
LIMIT 50
OFFSET 100;
```

A few things to keep in mind:

- The limit and offset apply only to the outermost query — that is, to the rows of the final result. A [user-defined table](#user-defined-tables), an earlier stage of a [pipeline](#pipeline-of-multiple-queries), and a scalar subquery are never limited.
- Because all [joined data is aggregated](#mandatory-aggregation) with respect to the base table, the limit counts rows of the result (base table rows, or groups when [grouping](#grouping-and-aggregation)), never related records.
- Without sorting, a database is free to return rows in any order, so pages may overlap or skip rows. To paginate reliably, sort on something unique, such as the primary key (e.g. `\\id` as the last sorting expression).
