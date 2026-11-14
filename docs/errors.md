# Errors

When Querydown can't compile a query, it says what's wrong, where in the query the problem is, and often how to fix it. This page describes the errors that the compiler reports, and how to read them from the [command line](#command-line), from [Rust](#rust), and from [JavaScript](#javascript).

## What an error tells you

Here's the error for a query that names a column that doesn't exist, as the command line tool shows it:

```text
compile error: Column `nope` not found within table `issues`.
 --> line 1, column 10
  |
1 | #issues $nope
  |          ^^^^
```

Every error has the same parts, whichever way you run the compiler:

| Part | Meaning |
| -- | -- |
| Kind | The broad category of the problem. See [kinds of error](#kinds-of-error). |
| Message | A description of the problem, in full sentences |
| Span | Where the problem is in the source: the start and end of the text at fault |
| Labels | Other places that help explain the problem, each with a note. For example, where an unclosed `[` was opened. |
| Expected and found | For a syntax error: what could have come at that point, and what was there instead |
| Hint | A suggestion for fixing the problem, when there is one |

## Kinds of error

| Kind | Meaning | Location |
| -- | -- | -- |
| Syntax | The source isn't valid Querydown. | Always has a span. |
| Compile | The source is valid Querydown, but can't be compiled against the schema. For example, it names a column that doesn't exist. | Has a span, which locates the construct at fault: the name of the missing column, the name of an unknown function, and so on. |
| Schema | The schema JSON is invalid. | No span, because the problem isn't in the query. |
| Argument | Another argument is invalid, such as an unknown dialect. JavaScript only. | No span. |

More kinds may be added, so write code that copes with a kind it doesn't know.

The one compile error without a span is about a base table that was given separately from the source, when [compiling a query in sections](#errors-in-a-section).

## Errors in a section

An application can hold a query in sections (definitions, conditions, sorting, and result columns), and compile them together: with [`compile_sections`](./setup/javascript.md#compile_sections) in JavaScript, or with [`Query::from_parts`](./setup/rust.md#compile-a-query-in-sections) in Rust. An error then names the section that it's in, and its span is relative to that section's own text.

An error isn't always in the section where the problem shows up. An unknown column in the definition of a constant is reported in the definitions section, wherever the constant is used.

## Command line

When `querydown compile` fails, it exits with code **1**.

With the default `--format sql`, it prints a diagnostic to **standard error**, and nothing to standard output:

```text
$ querydown compile --schema schema.json '#issues foo:)'
syntax error: Unexpected `)`, which has no matching `(`. Expected a value.
 --> line 1, column 13
  |
1 | #issues foo:)
  |             ^
  |
  = hint: Remove this `)`, or add a `(` where the group should begin.
```

An error without a span prints just its first line, and its hint if it has one.

With `--format json`, it prints the error as JSON to **standard output** instead, within an `error` key:

```json
{
  "error": {
    "kind": "syntax",
    "message": "Unexpected `)`, which has no matching `(`. Expected a value.",
    "span": {
      "start": {
        "offset": 12,
        "line": 1,
        "column": 13
      },
      "end": {
        "offset": 13,
        "line": 1,
        "column": 14
      }
    },
    "section": null,
    "labels": [],
    "expected": [
      "a value"
    ],
    "found": "`)`",
    "hint": "Remove this `)`, or add a `(` where the group should begin."
  }
}
```

- `kind` is `"syntax"`, `"compile"`, or `"schema"`.
- Each entry of `labels` is `{"span": {...}, "message": "..."}`.
- Absent values are `null`.
- Offsets are in bytes, lines and columns start at 1, and columns count characters.

A failure of the tool itself, such as a schema file that it can't read, prints `error: ...` to standard error and also exits with code 1.

## Rust

Everything that can fail returns `Result<_, querydown::Error>`: `Compiler::new`, `Compiler::compile`, `Compiler::compile_query`, and the parsing functions (`parse`, `parse_definitions`, `parse_conditions`, `parse_sorting`, and `parse_display`).

An `Error` has these public fields:

| Field | Type | Meaning |
| -- | -- | -- |
| `kind` | `ErrorKind` | `Syntax`, `Compile`, or `Schema` |
| `message` | `String` | The description of the problem, without its location |
| `span` | `Option<Span>` | Where in the source the problem is |
| `section` | `Option<Section>` | For a query [in sections](#errors-in-a-section): the section whose text `span` is relative to |
| `labels` | `Vec<Label>` | Other relevant places, each with a `span` and a `message` |
| `expected` | `Vec<String>` | For a syntax error: what would have been accepted at `span`, e.g. `` ["a value", "`]`"] `` |
| `found` | `Option<String>` | For a syntax error: what was found at `span` instead, e.g. `` "`)`" `` or `"end of input"` |
| `hint` | `Option<String>` | A suggestion for fixing the problem |

`Error` and `ErrorKind` are `#[non_exhaustive]`, so match on `kind` with a wildcard arm.

A `Span` has a `start` and an `end` (exclusive), each a `Position` with:

- `offset`: a **byte** offset into the source, so that `&source[span.range()]` is the text of the span
- `line`: starting from 1
- `column`: starting from 1, and counted in `char`s rather than bytes

A span can be empty (`start == end`), most often where the input ended too soon.

To show an error, you have three options:

- `error.to_string()` gives the message on one line, followed by ` (line L, column C)` when the error has a span.
- `error.render(source)` gives a diagnostic of several lines, which quotes the source and underlines the span, like the ones on this page.
- `Error` implements `serde::Serialize`, for sending it elsewhere as JSON.

```rust
use querydown::{Compiler, ErrorKind};

fn compile(compiler: &Compiler, input: &str) -> Option<String> {
    match compiler.compile(input.to_string()) {
        Ok(result) => Some(result.sql),
        Err(err) => {
            match (err.kind, err.span) {
                // E.g. to underline the text at fault in an editor.
                (ErrorKind::Syntax | ErrorKind::Compile, Some(span)) => {
                    println!("{} at {:?}: {}", err.kind, &input[span.range()], err.message);
                }
                _ => println!("{err}"),
            }
            // A full diagnostic, for a terminal:
            eprintln!("{}", err.render(input));
            None
        }
    }
}
```

## JavaScript

Every function of the [JavaScript package](./setup/javascript.md) reports failure by throwing a `QuerydownError`. It's a real `Error` (so `instanceof Error` holds, and it has a `stack`) whose `name` is `"QuerydownError"`, with these properties:

| Property | Type | Meaning |
| -- | -- | -- |
| `message` | `string` | One line: the `reason`, after the name of the section if there is one, and before ` (line L, column C)` if the error has a span |
| `kind` | `"syntax" \| "compile" \| "schema" \| "argument"` | The [kind of error](#kinds-of-error) |
| `reason` | `string` | The description of the problem alone, without the section or the location |
| `section` | `QuerydownSection \| null` | For [`compile_sections`](#errors-in-a-section): the section that the error is in |
| `span` | `SourceSpan \| null` | Where in the source the problem is |
| `labels` | `QuerydownErrorLabel[]` | Other relevant places, each with a `span` and a `message` |
| `expected` | `string[]` | For a syntax error: what would have been accepted at `span` |
| `found` | `string \| null` | For a syntax error: what was found at `span` instead |
| `hint` | `string \| null` | A suggestion for fixing the problem |

Absent values are `null`, never `undefined`.

Use `is_querydown_error` to tell these errors from any others in a `catch` block. It's a TypeScript type guard.

```ts
type QuerydownErrorKind = "syntax" | "compile" | "schema" | "argument";
type QuerydownSection = "definitions" | "conditions" | "sorting" | "display";

interface SourcePosition {
  offset: number; // in UTF-16 code units, from the start of the source
  line: number; // starting from 1
  column: number; // starting from 1, in UTF-16 code units
}

interface SourceSpan {
  start: SourcePosition;
  end: SourcePosition; // exclusive, and equal to `start` for an empty span
}

interface QuerydownErrorLabel {
  span: SourceSpan;
  message: string;
}
```

**Positions are in UTF-16 code units**, the same units as the indices of a JavaScript string. So `input.slice(error.span.start.offset, error.span.end.offset)` is the text at fault, and you can pass a `line` and `column` straight to an editor such as CodeMirror or Monaco. (In Rust and on the command line, offsets are in bytes.)

A span can be empty, at an unexpected end of input for example. Widen it if your editor needs a range that isn't empty.

Here's how you might show an error as a marker in an editor, falling back to its message:

```ts
import { compile, is_querydown_error } from "querydown-js";

try {
  const { sql } = compile(schemaJson, "postgres", input);
  showSql(sql);
  setMarkers([]);
} catch (error) {
  if (!is_querydown_error(error)) throw error;
  if (error.span) {
    const { start, end } = error.span;
    setMarkers([
      {
        from: start.offset,
        // An empty span (at the end of the input, say) is made one character wide.
        to: start.offset === end.offset ? end.offset + 1 : end.offset,
        message: error.hint ? `${error.reason}\nHint: ${error.hint}` : error.reason,
      },
    ]);
  } else {
    showError(error.message); // e.g. "Base table `nope` does not exist."
  }
}
```
