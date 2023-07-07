# Syntax cheat sheet

_See the **[Language Guide](./language.md)** for more detail._

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->
**Table of Contents**  *generated with [DocToc](https://github.com/thlorenz/doctoc)*

- [Literals](#literals)
- [Comparisons](#comparisons)
- [Comparison operators](#comparison-operators)
- [Result column control](#result-column-control)
- [Paths to data](#paths-to-data)
- [Case expressions](#case-expressions)
- [Functions](#functions)
- [Variables](#variables)
- [Transformations](#transformations)
- [Modules](#modules)
- [Operator precedence](#operator-precedence)
- [Not used](#not-used)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->


## Literals

| Code | Usage | Implemented? |
| -- | -- | -- |
| `//` `/* */` | code comments | ❌ |
| `@2000-01-01` | [dates](./language.md#date-literals) | ✅ |
| `@1y` | [durations](./language.md#duration-literals) | ✅ |
| `@` | sigil for [built-in](./syntax.md#built-in-constants) and [user-defined](./syntax.md#user-defined-constants) constants | ✅ |
| `..` `..<` `<..` `<..<` | [ranges](./language.md#ranges) | ✅ |
| `"` or `'` | string quote | ✅ |
| `^` | [string flag](./language.md#flagged-strings) prefix | ❌ |
| `{ }` | [string interpolation](./language.md#flagged-strings) | ❌ |
| `\` | string escape sequence prefix | ✅ |
| `` ` `` | [identifier quote](./language.md#identifiers-table-names-and-column-names) | ✅ |

## Comparisons

| Code | Usage | Implemented |
| -- | -- | -- |
| `{ }` | [set of `AND` conditions](./language.md#and-condition-sets) | ✅ |
| `[ ]` | [set of `OR` conditions](./language.md#or-condition-sets) | ✅ |
| `..` | [comparison expansion](./language.md#comparison-expansion) | ✅ |
| `++` | [has at least one](./language.md#has-some-and-has-none-conditions) | ✅ |
| `--` | [has none](./language.md#has-some-and-has-none-conditions) | ✅ |

## Comparison operators

| Code | Usage | Implemented |
| -- | -- | -- |
| `:` | equals | ✅ |
| `:<` | less than | ✅ |
| `:<=` | less or equal | ✅ |
| `:>` | greater than | ✅ |
| `:>=` | greater or equal | ✅ |
| `:~` | match regex | ✅ |
| `:\c~` | match regex with flags | ❌ |
| `:~~` | LIKE | ❌ |
| `!` | negate any comparison by using `!` instead of `:` | ✅ |

Regex flags

- `c` - make RegEx comparison case sensitive (the default is case insensitive)

## Result column control

| Code | Usage | Implemented |
| -- | -- | -- |
| `$` | [result column](./language.md#result-columns) prefix | ✅ |
| `*( )` | [column globs](./language.md#column-globs) | ✅ |
| `->` | [alias](./language.md#aliasing-result-columns) prefix | ✅ |
| `\` | column control flags prefix | ✅ |

Column control flags:

- `g` - [group](./language.md#grouping-and-aggregation)
- `s` - [sort](./language.md#basic-sorting)
- `d` - [descending](./language.md#descending-sorting)
- `n` - [nulls first](./language.md#sorting-null-values)
- `h` - [hide](./language.md#hiding-columns-within-a-glob)
- `p` - partition (in a [window definition](./language.md#window-functions))
- digits `1` through `9` - sorting/grouping [ordinality](./language.md#multiple-sorting)

## Paths to data

| Code | Usage | Implemented |
| -- | -- | -- |
| `#` | [table sigil](./language.md#identifiers-table-names-and-column-names) | ✅ |
| `.` | [path separator](./language.md#single-related-records-via-column-name-chains) | ✅ |
| _alphanumeric identifier_ | column | ✅ |
| `>>` | path to [table with a single record](./language.md#single-related-records-via-table-name) | ❌ |

## Case expressions

See [case expressions](./language.md#case-expressions) docs.

| Code | Usage | Implemented |
| -- | -- | -- |
| `?` | if | ❌ |
| `~` | then (can occur many times without nesting) | ❌ |
| `~~` | else | ❌ |

## Functions

| Code | Usage | Implemented |
| -- | -- | -- |
| `+` `-` `*` `/` | basic arithmetic operators | ✅ |
| <tt>&VerticalLine;</tt> | [pipe a value into a scalar function](./language.md#function-piping) | ✅ |
| `%` | pipe a value to an aggregate function | ✅ |
| `@@` | [call a scalar function without piping](./language.md#function-calling) | ❌ |
| `%%( )` | [window definition](./language.md#window-functions) | ❌ |
| `;` | [anonymous scalar function](./language.md#anonymous-functions) | ❌ |

## Variables

| Code | Usage | Implemented |
| -- | -- | -- |
| `@foo = 42` | [constant](./language.md#user-defined-constants) | ❌ |
| `#foo.bar = baz + bat` | [computed column](./language.md#computed-columns) | ❌ |
| `@@plus_one = @v; @v + 1` | [scalar function](./language.md#user-defined-functions) | ❌ |
| `@@plus = @a @b; a + b` | function with two params | ❌ |
| `#foo.@@bar = @a; @a + col` | [table-scoped function](./language.md#table-scoped-functions) | ❌ |
| `#foo = #( )` | [temporary table](./language.md#user-defined-tables) | ❌ |

## Transformations

| Code | Usage | Implemented |
| -- | -- | -- |
| `~~~` | [pipeline](./language.md#pipeline-of-multiple-queries) of multiple queries | ❌ |
| `+++` | [union](./language.md#union-of-multiple-queries) of multiple queries | ❌ |

## Modules

See [modules](./language.md#modules) docs.

| Code | Usage | Implemented |
| -- | -- | -- |
| `===` | export | ❌ |
| `<<<` | import | ❌ |
| `->` | alias | ❌ |

## Operator precedence

1. `|` `%` Function pipes (Highest precedence, evaluated first)
1. `*` `/` Multiplication and division
1. `+` `-` Addition and subtraction
1. `:` _(and all other [comparison operators](#comparisons))_ Comparison

## Not used

- `&`
- `,`

