# Syntax cheat sheet

## Literals

| Code     | Usage |
| --       | -- |
| `"`      | string quote |
| `'`      | string quote (alternate) |
| `` ` ``  | db entity quote |
| `^`      | interpolated string quote |
| `{ }`    | expression within an interpolated string |
| `\`      | escape sequence within string |
| `@`      | prefix for date/time values (e.g. `@2000-01-01`) |
| `@P`     | prefix for duration values |
| `@now`   | `now()` |
| `@inf`   | `Infinity` |
| `@true`  | `TRUE` |
| `@false` | `FALSE` |
| `@null`  | `NULL` |
| `//`     | single line comment |
| `/* */`  | multi-line comment |

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
| `#`   | value from scope outside comparison expansion |

## Joins

| Code   | Usage |
| --     | -- |
| `.`   | related column |
| `..`  | transitively related table |
| `*`   | one-to-many join |
| `++`  | has at least one |
| `--`  | has none |

## Column control

| Code   | Usage |
| --     | -- |
| `-`    | column spec prefix |
| `-( )` | relative column spec |
| `[ ]`  | column control |
| `s`    | "sort" flag |
| `h`    | "hide" flag |
| `g`    | "group" flag |
| `d`    | "descending" flag (in a sort spec) |
| `n`    | "nulls first" flag (in a sort spec) |
| `p`    | "partition" flag (in a window definition) |
| `:`    |  alias |

## Functions

| Code    | Usage |
| --      | -- |
| `%`     | aggregate function |
| `∣`     | scalar function |
| `( )`   | function arguments (if any) |
| `,`     | function argument delimiter |
| `:`     |  associative function arguments |
| `%%( )` | window definition |

## Transformations

| Code   | Usage |
| --     | -- |
| `-->` | LIMIT and OFFSET |
| `~~~` | pipeline |
| `+++` | union |

## Not (yet) used

```
$   may be used for user-defined variable
&   may be used for parameterization
/
+
_
;
```