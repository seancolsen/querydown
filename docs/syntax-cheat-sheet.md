# Syntax cheat sheet

## Literals

| Code          | Usage |
| --            | -- |
| `` ` ``       | db entity quote |
| `"` or `'`    | string quote |
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

## Column control

| Code      | Usage |
| --        | -- |
| <tt>&VerticalLine;</tt>       | column spec prefix |
| <tt>&VerticalLine;[ ]</tt>    | incremental column spec |
| `->`      | alias |
| `\`       | column control flags |
| `g`       | "group" flag |
| `s`       | "sort" flag |
| `1` - `9` | sorting/grouping ordinality |
| `d`       | "descending" flag |
| `n`       | "nulls first" flag |
| `h`       | "hide" flag |
| `p`       | "partition" flag (in a window definition) |

## Paths to data

| Code   | Usage |
| --     | -- |
| `.`                       | path separator |
| _alphanumeric identifier_ | column |
| `#`                       | path to table with many records (to be aggregated) |
| `>>`                      | path to table with a single record |

## Conditions

| Code   | Usage |
| --     | -- |
| `[ ]` | OR conditions |
| `{ }` | AND conditions |
| `=`   | equals |
| `<`   | less than |
| `<=`  | less or equal |
| `>`   | greater than |
| `>=`  | greater or equal |
| `~`   | regex |
| `~~`  | LIKE |
| `!`   | negate any comparison |
| `++`  | has at least one |
| `--`  | has none |

## Ternary

| Code      | Usage |
| --        | -- |
| `?`       | if |
| `=>`      | then (can occur many times without nesting) |
| `*=>`     | else |

## Functions

| Code    | Usage |
| --      | -- |
| `+` `-` `*` `/` | standard algebraic operators |
| `:`     | pipe a value to a scalar function (higher precedence than algebra) |
| `%`     | pipe a value to an aggregate function |
| `( )`   | function arguments (if any) |
| `%%[ ]` | window definition |
| `;`     | anonymous scalar function |

## Variable definition

| Code    | Usage |
| --      | -- |
| `@foo := 42`           | define a constant |
| `foo := v; v + 42`     | define a scalar function with one parameter |
| `plus := (a b); a + b` | define a scalar function with two parameters |
| `foo.bar := baz + bat` | define a computed column |
| `#foo := ( )`          | define a temporary table |

## Transformations

| Code   | Usage |
| --     | -- |
| `:::` | LIMIT and OFFSET |
| `~~~` | pipeline |
| `+++` | union |

## Not used

```
$
&
_
,
```