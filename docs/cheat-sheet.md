# Cheat sheet

This page lists all of Querydown's syntax, with a link to where each piece is explained on the [language](./language.md) page. For the built-in functions, see [functions](./functions.md).

## A query at a glance

```qd
@cutoff = 6m                                  // definitions
#issues                                       // base table
status:open created_at:<@cutoff               // conditions
\\due_date \d                                 // sorting
$title $author.username $#comments -> replies // result columns
```

## Literals

| Code | Meaning |
| -- | -- |
| `"text"` `'text'` | [string](./language.md#string-literals), which is raw (no escape sequences) |
| `^f"Hi {name}"` | [formatted string](./language.md#formatted-strings), which interpolates the expressions within `{ }` |
| `^e'it\'s'` | string with [escape sequences](./language.md#escape-sequences) |
| `^/text/` | [flagged string](./language.md#flagged-strings) with another quote character: `"` `'` `^` `#` `/` or `\|` |
| `42` `3.14` | number |
| `@2000-01-01` | [date](./language.md#date-literals) |
| `1y` `2w3d` `90min` | [duration](./language.md#duration-literals), with the units `y` `m` `w` `d` `h` `min` `s` |
| `@true` `@false` `@null` `@now` `@infinity` | [built-in constants](./language.md#built-in-constants) |
| `` `Due Date` `` | [quoted identifier](./language.md#identifiers) |
| `//` `/* */` | comments |

## Paths to data

| Code | Meaning |
| -- | -- |
| `#issues` | [table](./language.md#identifiers). The first one in a query is its [base table](./language.md#query-anatomy). |
| `#/billing/invoices` | [table in another schema](./language.md#tables-in-other-schemas) |
| `title` | column |
| `author.username` | column of a [single related record](./language.md#single-related-records-via-column-name-chains) |
| `#comments` | count of [many related records](./language.md#aggregate-counts) |
| `#comments.created_at%max` | [aggregate](./language.md#specifying-an-aggregate-function) of a column of many related records |
| `#comments{user:5}` | related records, [filtered](./language.md#conditions-to-filter-aggregate-data) |
| `#blocks(blocker)` | related records, reached via a [particular linking column](./language.md#specifying-the-linking-column) |
| `#products.#projects.#issues` | related records, reached via [particular tables](./language.md#transitive-relationships) |

## Conditions

| Code | Meaning |
| -- | -- |
| `a:1 b:2` | conditions that must all be true |
| `{a:1 b:2}` | [set of `AND` conditions](./language.md#and-condition-sets) |
| `[a:1 b:2]` | [set of `OR` conditions](./language.md#or-condition-sets) |
| `a:1,b:2` | shorthand for an [`OR` set](./language.md#or-condition-sets) |
| `!a:1` | [negation](./language.md#negation) of any condition |
| `a:[1 2]` `[a b]:1` | [comparison expansion](./language.md#comparison-expansion): compare with each entry of the set |
| `a:1..5` | [range](./language.md#ranges). Use `<..` `..<` `<..<` to exclude a bound. |
| `++#comments` | [has at least one](./language.md#has-some-and-has-none-conditions) related record |
| `--#comments` | [has no](./language.md#has-some-and-has-none-conditions) related records |
| `author{team:5 username:al}` | [scoped comparison](./language.md#scoped-comparisons): conditions on a related record (no space before the set) |
| `performance` | [default text search](./language.md#default-text-search) for a bare word, in all text columns |

## Comparison operators

| Code | Meaning |
| -- | -- |
| `:` | [matches](./language.md#match-comparisons): text contains the value, and anything else equals it |
| `:=` | [equals](./language.md#equality-comparisons), ignoring the case of text |
| `:==` | [equals](./language.md#equality-comparisons) exactly |
| `:<` `:<=` | [less than](./language.md#numeric-comparisons), less than or equal to |
| `:>` `:>=` | [greater than](./language.md#numeric-comparisons), greater than or equal to |
| `:~` | [matches a regular expression](./language.md#regular-expression-matching) |
| `:\cms~` | matches a regular expression with [flags](./language.md#regex-flags): `c` case-sensitive, `m` multi-line, `s` dot-all |

- A bare word _anywhere_ on the right-hand side of a comparison is a [string literal](./language.md#bare-text-on-the-right-hand-side-of-a-comparison): `title:performance` is the same as `title:"performance"`. Quote it with backticks to refer to a column instead.
- A date compared against a duration [measures from now](./language.md#comparing-dates-with-durations): `created_at:<1w` finds dates within a week of now.

## Result columns

| Code | Meaning |
| -- | -- |
| `$title` | [result column](./language.md#result-columns) |
| `$title -> name` | [alias](./language.md#aliasing-result-columns) |
| `$*` | [all columns](./language.md#column-globs) |
| `$author.*` | all columns of a [related record](./language.md#column-glob-on-related-table) |
| `$*(description \h)` | all columns, with [adjustments](./language.md#hiding-columns-within-a-glob) to some |
| `$author.( $username $email )` | [nested result columns](./language.md#nesting-result-columns) that share a path |
| `$title \s` | column with [flags](#flags) |
| `$title @{width:100}` | column with an [annotation](./language.md#column-level-annotations), written last |

## Flags

Flags follow a backslash, and several can be combined, as in `\sd2`.

| Flag | Meaning |
| -- | -- |
| `s` | [sort](./language.md#basic-sorting) by the column |
| `d` | sort [descending](./language.md#descending-sorting) |
| `n` | sort [NULL values first](./language.md#sorting-null-values) |
| `g` | [group](./language.md#grouping-and-aggregation) by the column |
| `1` to `9` | the column's place in the [order of sorting](./language.md#multiple-sorting) or grouping |
| `h` | [hide](./language.md#hiding-columns-within-a-glob) the column (within a glob) |
| `p` | partition by the column (within a [window definition](./language.md#window-functions)) |

## Sorting

| Code | Meaning |
| -- | -- |
| `$title \s` | [sort by a result column](./language.md#basic-sorting) |
| `\\created_at` | [sorting expression](./language.md#sorting-outside-of-result-columns): sort without showing the value |
| `\\created_at \dn` | sorting expression that's descending, with NULL values first |
| `\\project.( \\name \\id )` | [scoped sorting expressions](./language.md#scoped-sorting-expressions) that share a path |
| `%list(\\created_at%min \d)` `%list(\sd)` | [sorting within an aggregate function](./language.md#sorting-within-an-aggregate-function) |

## Computations

| Code | Meaning |
| -- | -- |
| `+` `-` `*` `/` | [arithmetic](./language.md#arithmetic) |
| `value\|func` `value\|func(arg)` | [pipe](./language.md#function-piping) a value into a scalar function |
| `@@func(a b)` | [call](./language.md#function-calling) a scalar function without a pipe |
| `column%func` | apply an [aggregate function](./language.md#specifying-an-aggregate-function) to a column, or to a [value computed from one](./language.md#aggregating-a-computed-value) like `#comments.body\|length%avg` |
| `%count` | count the rows of a [group](./language.md#grouping-and-aggregation) |
| `%%(a\p b\s)%func` | apply a [window function](./language.md#window-functions) to a window |
| `? a ~ b ~~ c` | [case expression](./language.md#case-expressions): if `a` then `b`, else `c`. The `a ~ b` part can repeat. |
| `(@x => @x + 1)` | [anonymous function](./language.md#anonymous-functions) |

## Definitions

Definitions go before the base table.

| Code | Meaning |
| -- | -- |
| `@foo = 42` | [constant](./language.md#user-defined-constants) |
| `@foo = #( #issues $%count )` | [constant from a query](./language.md#defining-a-constant-using-the-result-of-a-query) |
| `#issues.age = created_at\|age` | [computed column](./language.md#computed-columns) |
| `@@plus_one = @v => @v + 1` | [function](./language.md#user-defined-functions) |
| `@@plus = @a @b => @a + @b` | [function with several parameters](./language.md#function-with-multiple-parameters) |
| `#issues.comment:@x = ++#comments{body:@x}` | [custom comparison](./language.md#custom-comparisons) |
| `#foo = #( #issues status:open )` | [user-defined table](./language.md#user-defined-tables) |

## Combining queries

| Code | Meaning |
| -- | -- |
| `~~~` | [pipeline](./language.md#pipeline-of-multiple-queries): the next stage queries the result of the one before |
| `+++` | [union](./language.md#union-of-multiple-queries): append the results of another query |
| `\\\{view:board}` | [query annotation](./language.md#query-level-annotations), written last |

## Operator precedence

From the highest precedence (evaluated first) to the lowest:

1. `|` `%` function pipes
1. `*` `/` multiplication and division
1. `+` `-` addition and subtraction
1. `:` and the other [comparison operators](#comparison-operators)
1. `!` negation
1. `,` the shorthand for an ["OR" condition set](./language.md#or-condition-sets). It joins whole conditions, so `status:open,closed` is `status:open` or a search for "closed". To compare with several values, write `status:[open closed]`.

Use parentheses to group an expression. The `..` of a [range](./language.md#ranges) binds more tightly than any of these.

## Unused characters

Querydown gives no meaning to `&` or `;`.
