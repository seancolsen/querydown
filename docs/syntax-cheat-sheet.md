# Syntax cheat sheet

## Literals

| Code          | Usage |
| --            | -- |
| `"`           | string quote |
| `'`           | string quote (alternate) |
| `` ` ``       | db entity quote |
| `^`           | interpolated string quote |
| `{ }`         | expression within an interpolated string |
| `\`           | escape sequence within string |
| `@2000-01-01` | date/time values |
| `@1y`         | duration values |
| `@now`        | `now()` |
| `@inf`        | `Infinity` |
| `@true`       | `TRUE` |
| `@false`      | `FALSE` |
| `@null`       | `NULL` |
| `//`          | single line comment |
| `/* */`       | multi-line comment |

## Paths to data (i.e. joins)

| Code   | Usage |
| --     | -- |
| `.`                       | path separator |
| _alphanumeric identifier_ | column |
| `#`                       | path to table with aggregated records |
| `>>`                      | path to table with singular records |

## Conditionals

| Code   | Usage |
| --     | -- |
| `[ ]` | OR conditions |
| `{ }` | AND conditions |
| `=`   | equals |
| `!=`  | not equal |
| `<`   | less than |
| `<=`  | less or equal |
| `>`   | greater than |
| `>=`  | greater or equal |
| `~`   | regex |
| `!~`  | not RLIKE |
| `~~`  | LIKE |
| `!~~` | not LIKE |
| `?`   | comparison expansion |
| `&`   | slot (value from scope outside comparison expansion) |
| `++`  | has at least one |
| `--`  | has none |

## Column control

| Code      | Usage |
| --        | -- |
| `:`       | column spec prefix |
| `:[ ]`    | incremental column spec |
| `->`      |  alias |
| `\`       | column control flags |
| `g`       | "group" flag |
| `s`       | "sort" flag |
| `1` - `9` | sorting/grouping ordinality |
| `d`       | "descending" flag |
| `n`       | "nulls first" flag |
| `h`       | "hide" flag |
| `p`       | "partition" flag (in a window definition) |

## Functions

| Code    | Usage |
| --      | -- |
| `%`     | aggregate function |
| `∣`     | scalar function |
| `( )`   | function arguments (if any) |
| `%%[ ]` | window definition |

## Transformations

| Code   | Usage |
| --     | -- |
| `-->` | LIMIT and OFFSET |
| `~~~` | pipeline |
| `+++` | union |

## Not (yet) used

```
$   reserved for user-defined variables and functions

*   reserved for algebraic expressions
/   reserved for algebraic expressions
+   reserved for algebraic expressions
-   reserved for algebraic expressions

_
;
,
```