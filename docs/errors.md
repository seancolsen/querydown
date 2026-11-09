# Error handling

Every fallible stage of Querydown processing — parsing, setting up the compiler with a schema, and
compiling — reports failure with the same structured error type. This page covers that type from
Rust and from the command line. For JavaScript, see the [`QuerydownError` documentation in the JS
bindings README](../bindings/js/README.md#errors).

## Rust

`querydown::parse`, `parse_definitions`, `parse_conditions`, `parse_sorting`, `parse_display`,
`Compiler::new`, `Compiler::compile`, and `Compiler::compile_query` all return
`Result<_, querydown::Error>`. (The type, along with `ErrorKind`, `Span`, `Position`, `Section`, and
`Label`, is defined in the `querydown-parser` crate and re-exported from `querydown`.)

An `Error` has these public fields:

| Field      | Type               | Meaning                                                                                 |
| ---------- | ------------------ | --------------------------------------------------------------------------------------- |
| `kind`     | `ErrorKind`        | `Syntax`, `Schema`, or `Compile` (see below)                                            |
| `message`  | `String`           | Human-readable description, as complete sentences, without the location                |
| `span`     | `Option<Span>`     | Where in the source the problem is, if it can be pinpointed                             |
| `section`  | `Option<Section>`  | For a query parsed in sections: the section whose text `span` is relative to            |
| `labels`   | `Vec<Label>`       | Secondary locations, each with a note (e.g. where an unclosed `[` was opened)           |
| `expected` | `Vec<String>`      | For syntax errors: what would have been accepted at `span`, e.g. `` ["a value", "`]`"] `` |
| `found`    | `Option<String>`   | For syntax errors: what was found at `span` instead, e.g. `` "`)`" `` or `"end of input"` |
| `hint`     | `Option<String>`   | A suggestion on how to fix the problem, when one is available                           |

The kinds are:

- **`Syntax`** — the Querydown source isn't valid syntax. Syntax errors **always** have a `span`.
- **`Compile`** — the source is valid syntax but can't be compiled, e.g. because it names a column
  that doesn't exist. Compile errors have a `span` locating the construct at fault: the column
  reference naming the missing column, the name of an unknown function, the `\\` sort entry that
  `list` can't be sorted by, and so on. The one exception is a construct that wasn't parsed from
  source, which has nowhere to point: the base table passed to `Query::from_parts`.
- **`Schema`** — the schema JSON given to `Compiler::new` is invalid. The problem is in the schema,
  not the query, so there's no `span`.

`ErrorKind` and `Error` are `#[non_exhaustive]`, so match `kind` with a wildcard arm.

A `Span` has a `start` and an `end` (exclusive) `Position`, each with:

- `offset`: a **byte** offset into the source, so `&source[span.range()]` slices out the span's text.
- `line`: 1-based.
- `column`: 1-based, counted in `char`s (Unicode scalar values), not bytes.

A span may be empty (`start == end`), most commonly when the input ended unexpectedly.

When a query is parsed in sections (`parse_conditions` etc.), spans are relative to the section's own
text, and `section` names that section (`Definitions`, `Conditions`, `Sorting`, or `Display`). This
holds for compile errors from `Compiler::compile_query` too, because the syntax tree records where
each node was written. A compile error isn't necessarily in the section where the problem shows up:
an unknown column in a constant's definition is reported in the definitions section, wherever the
constant is used.

For display, `Error` implements `Display` (the message plus ` (line L, column C)` when located) and
`std::error::Error`, and `Error::render(source)` draws a multi-line, rustc-style diagnostic that
quotes and underlines the source. `Error` also implements `serde::Serialize`.

```rust
use querydown::{Compiler, ErrorKind, IdentifierResolution, Options, Postgres};

fn compile(schema_json: &str, input: &str) -> Option<String> {
    let options = Options {
        dialect: Box::new(Postgres()),
        identifier_resolution: IdentifierResolution::Flexible,
    };
    let compiler = Compiler::new(schema_json, options).ok()?;
    match compiler.compile(input.to_string()) {
        Ok(result) => Some(result.sql),
        Err(err) => {
            match (err.kind, err.span) {
                // E.g. for an editor underline.
                (ErrorKind::Syntax | ErrorKind::Compile, Some(span)) => {
                    println!("{} at {:?}: {}", err.kind, &input[span.range()], err.message);
                }
                _ => println!("{err}"),
            }
            // A full diagnostic for a terminal:
            eprintln!("{}", err.render(input));
            None
        }
    }
}
```

## Command line

When `querydown compile` fails, it exits with code **1**.

With the default `--format sql`, it prints a rendered diagnostic to **stderr** (nothing is printed
to stdout):

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

Compile errors are rendered the same way, underlining the construct at fault:

```text
$ querydown compile --schema schema.json '#issues $nope'
compile error: Column `nope` not found within table `issues`.
 --> line 1, column 10
  |
1 | #issues $nope
  |          ^^^^
```

Errors without a span print just the first line (plus a `hint:` line, if there is a hint), e.g.
``schema error: The schema has multiple links from the same column.``

With `--format json`, it instead prints the serialized error, wrapped in an `error` key, to
**stdout**. Absent values are `null`, offsets are in bytes, and columns are in `char`s:

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

`kind` is one of `"syntax"`, `"compile"`, or `"schema"`, and each entry of `labels` is
`{"span": {...}, "message": "..."}`. Failures of the CLI itself, such as an unreadable schema file,
print `error: ...` to stderr and also exit with code 1.

## JavaScript

The JS bindings throw a `QuerydownError` — a real `Error` with the same information, but with
offsets and columns in UTF-16 code units and attribution to a section for `compile_sections`. See
[bindings/js/README.md](../bindings/js/README.md#errors).
