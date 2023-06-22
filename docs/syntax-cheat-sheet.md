# Syntax cheat sheet

_See also: **[Language Design](./design.md)**_

## Literals

| Code                    | Usage |
| --                      | -- |
| `` ` ``                 | db entity quote |
| `"` or `'`              | string quote |
| `^`                     | string flag prefix |
| `{ }`                   | string interpolation |
| `\`                     | string escape sequence prefix |
| `@2000-01-01`           | date/time values |
| `@1y`                   | duration values |
| `@now`                  | `now()` |
| `@infinity`             | `Infinity` |
| `@true`                 | `TRUE` |
| `@false`                | `FALSE` |
| `@null`                 | `NULL` |
| `..` `..!` `!..` `!..!` | range |
| `//`                    | single line comment |
| `/* */`                 | multi-line comment |

String flags:

- `f` - formatting (aka interpolation) via `{ }`
- `e` - interpret escape sequences (default is raw)

## String flags

| Flag | Meaning |
| -- | -- |
| `f` | formatting (aka interpolation) via `{ }` |
| `e` | handle escape sequences |

## Comparisons

| Code   | Usage |
| --     | -- |
| `[ ]`  | OR conditions |
| `{ }`  | AND conditions |
| `:`    | equals |
| `:<`   | less than |
| `:<=`  | less or equal |
| `:>`   | greater than |
| `:>=`  | greater or equal |
| `:~`   | match regex |
| `:\c~` | match regex with flags |
| `:~~`  | LIKE |
| `!`    | negate any comparison by using `!` instead of `:` |
| `..`   | comparison expansion |
| `++`   | has at least one |
| `--`   | has none |

Regex flags

- `c` - make RegEx comparison case sensitive (the default is case insensitive)

## Column control

| Code      | Usage |
| --        | -- |
| `$`       | column spec prefix |
| `$*( )`   | incremental column spec |
| `->`      | alias |
| `\`       | column control flags |

Column control flags:

- `g` - group
- `s` - sort
- `d` - descending
- `n` - nulls first
- `h` - hide
- `p` - partition (in a window definition)
- digits `1` through `9` - sorting/grouping ordinality

## Paths to data

| Code                      | Usage |
| --                        | -- |
| `#`                       | table prefix |
| `.`                       | path separator |
| _alphanumeric identifier_ | column |
| `>>`                      | path to table with a single record |

## If/else

| Code | Usage |
| --   | -- |
| `?`  | if |
| `~`  | then (can occur many times without nesting) |
| `~~` | else |

## Functions

| Code                    | Usage |
| --                      | -- |
| `+` `-` `*` `/`         | standard algebraic operators |
| <tt>&VerticalLine;</tt> | pipe a value to a scalar function (higher precedence than algebra) |
| `%`                     | pipe a value to an aggregate function |
| `( )`                   | function arguments (if any) |
| `%%( )`                 | window definition |
| `;`                     | anonymous scalar function |

## Variable definition

| Code                    | Usage |
| --                      | -- |
| `@foo = 42`             | define a constant |
| `@@plus_one = v; v + 1` | define a scalar function with one parameter |
| `@@plus = a b; a + b`   | define a scalar function with two parameters |
| `foo.bar = baz + bat`   | define a computed column |
| `#foo = ( )`            | define a temporary table |

## Transformations

| Code  | Usage |
| --    | -- |
| `:::` | LIMIT and OFFSET |
| `~~~` | pipeline |
| `+++` | union |

## Not used

```
&
_
,
```