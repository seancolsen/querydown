# Querydown JavaScript bindings

WebAssembly bindings for the [Querydown](https://github.com/seancolsen/querydown) compiler,
generated with [`wasm-pack`](https://rustwasm.github.io/wasm-pack/). They let a JavaScript or
TypeScript application compile Querydown source into SQL in the browser.

This package is **not published to npm**. Consume it by building from a git checkout (typically a
pinned Querydown revision) as described below.

## Building the package

From the repository root:

```sh
# Recommended for a browser app (e.g. Vite): you call init() yourself and the
# bundler serves the .wasm as a same-origin asset.
wasm-pack build bindings/js --target web

# Default target: ESM that a bundler wires up (may need vite-plugin-wasm — see below).
wasm-pack build bindings/js --target bundler
```

Either command emits an ES-module package to `bindings/js/pkg/` (git-ignored) containing a `.js`
entry point, a `querydown_js.d.ts` with full TypeScript types, the compiled `.wasm`, and a
`package.json`. Point your app's dependency at that `pkg/` directory (or copy/vendor it).

## Using it

### `--target web` (recommended)

With the `web` target you must initialize the module once before calling anything. The default
export loads and instantiates the `.wasm`; under Vite, import the asset URL so it is served
correctly:

```ts
import init, { compile, compile_sections, introspection_sql } from "querydown-js";
import wasmUrl from "querydown-js/querydown_js_bg.wasm?url"; // Vite: resolves to a served asset

await init(wasmUrl); // call once at startup

const result = compile(schemaJson, "postgres", "#issues $id $title");
console.log(result.sql);
```

### `--target bundler`

The bundler target is imported directly with no `init()` call; the bundler handles instantiation.
With Vite this usually requires [`vite-plugin-wasm`](https://github.com/Menci/vite-plugin-wasm)
(and `vite-plugin-top-level-await`):

```ts
import { compile } from "querydown-js";

const result = compile(schemaJson, "postgres", "#issues $id $title");
```

## API

### `introspection_sql(dialect: string): string`

Returns the static SQL that introspects a database of the given `dialect` (`"postgres"` or
`"duckdb"`) to produce the schema JSON consumed by `compile` / `compile_sections`. The query yields
a single row with a single column containing the JSON. Throws a [`QuerydownError`](#errors) of kind
`"argument"` on an unknown dialect.

### `compile(schemaJson: string, dialect: string, input: string): CompileResult`

Compiles whole-query Querydown `input` against `schemaJson` for the given `dialect`. Returns a typed
`CompileResult` **object** (see below). Throws a [`QuerydownError`](#errors) on a syntax, schema,
compile, or argument (unknown dialect) failure; its `span` is relative to `input`.

### `compile_sections(schemaJson, dialect, baseTable, definitions, conditions, sorting, display): CompileResult`

Compiles a query supplied as **independently-parsed sections** plus a base table, instead of one
whole-query string:

| Parameter     | Meaning                                                        |
| ------------- | ------------------------------------------------------------- |
| `baseTable`   | Name of the table the query is built on (chosen on its own)   |
| `definitions` | Prelude of constants, functions, computed columns, etc.       |
| `conditions`  | The filter section                                            |
| `sorting`     | Standalone `\\` sort expressions                              |
| `display`     | `$`-prefixed result columns                                   |

Each section is parsed with its own parser, so one section's syntax cannot leak into another — a
stray display `$` typed into the sorting input is reported as a `"Sort section: …"` error rather
than silently changing the result set. This mirrors a multi-input query-builder UI. Returns the same
`CompileResult` as `compile`; throws a [`QuerydownError`](#errors) on failure. A syntax error names
the section it occurred in (`err.section`, plus a prefix such as `"Sort section: "` on
`err.message`), and its `span` is relative to that section's text.

### `tokenize(source: string, grammar: QuerydownGrammar): QuerydownToken[]`

Classifies `source` for **syntax highlighting**. `grammar` says how to read it: `"query"` for a whole
query (as passed to `compile`), or `"definitions"`, `"conditions"`, `"sorting"`, or `"display"` for
one section (as passed to `compile_sections`). The grammar matters because the same text can play
different roles in different places.

Returns `{ from, to, kind }` tokens, in order and without overlap, covering every character except
whitespace. `from`/`to` are UTF-16 offsets, like JS string indices. `kind` is a
`QuerydownTokenKind`: `"column"`, `"string"`, `"search-term"`, `"base-table"`, `"function"`,
`"comparison-operator"`, and so on. The full list, with what each kind means, is in the generated
`querydown_js.d.ts`.

```ts
tokenize("#issues title:performance backend", "query");
// [ { from: 0, to: 7, kind: "base-table" }, { from: 8, to: 13, kind: "column" },
//   { from: 13, to: 14, kind: "comparison-operator" }, { from: 14, to: 25, kind: "string" },
//   { from: 26, to: 33, kind: "search-term" } ]
```

The tokens come from Querydown's own parser, so they follow the language's rules exactly — e.g.
`performance` above is a string because it's on the right-hand side of a comparison, and `backend`
is a default text search term because it stands alone. `tokenize` **never throws on invalid
Querydown**: source being edited is often invalid, so past the point where parsing fails, tokens fall
back to a lexical classification. It throws a [`QuerydownError`](#errors) of kind `"argument"` only
for an unknown `grammar`.

For CodeMirror 6, [`querydown-codemirror`](../codemirror/README.md) builds a complete language with
light and dark highlighting on top of this function.

### `new QuerydownCompleter(schemaJson: string)`

Creates an **autocompletion** engine for the database described by `schemaJson` (the same JSON as
for `compile`). The schema is parsed once, here, so create one completer per schema and reuse it.
Throws a [`QuerydownError`](#errors) of kind `"schema"` if the schema is invalid. (Call `free()` to
release its memory early; otherwise it's released when garbage collected.)

### `completer.complete(source: string, cursor: number, options?: QuerydownCompletionOptions): QuerydownCompletions`

Lists what could be typed at `cursor` (a UTF-16 offset) in `source`: tables after `#`, the columns
of the table in scope after `$` or `.` or in a condition, functions after `|`, `%` or `@@`, variables
after `@`, and so on. The source is read with Querydown's own parser, so, for example, the columns
offered in `#comments issue{…}` are those of `issues`, and in a later pipeline stage, those output by
the previous one. Never throws on invalid Querydown (source being edited is usually invalid); throws
a `QuerydownError` of kind `"argument"` for an unknown grammar.

```ts
const completer = new QuerydownCompleter(schemaJson);
completer.complete("#issues $author.us", 18);
// { from: 16, to: 18, annotation: null, items: [
//   { label: "id", kind: "column", detail: "integer", insert: null, info: null, boost: 0 },
//   { label: "username", kind: "column", detail: "text", insert: null, info: null, boost: 0 },
//   … ] }
```

`options` (all optional):

| Option        | Meaning |
| ------------- | ------- |
| `grammar`     | How to read `source`, as for [`tokenize`](#tokenizesource-string-grammar-querydowngrammar-querydowntoken): `"query"` (the default) or a section name |
| `baseTable`   | For a section other than `"definitions"`: the query's base table, whose columns are offered |
| `definitions` | For a section other than `"definitions"`: the definitions section's text, so that what it defines is offered |

The result:

- `from`/`to`: the UTF-16 range the items would replace — the word being typed, up to the cursor.
- `items`: what could be typed there, **not** filtered by what has been typed so far (leave that to
  your editor). Each has a `label`, a `kind` (`"table"`, `"column"`, `"computed-column"`,
  `"custom-comparison"`, `"function"`, `"aggregate-function"`, `"window-function"`, `"constant"`,
  or `"variable"`), and possibly an `insert` text differing from the label (e.g. a name quoted with
  backticks), a `detail` (e.g. a column's type), an `info` (e.g. what a function does), and a
  `boost` that ranks it above (or below) equally good matches.
- `annotation`: set when the cursor is within a [column annotation](../../docs/language.md#column-level-annotations),
  whose keys and values only your application knows. It gives the `path` of keys and indices to the
  object or array the cursor is in, the `slot` being typed (`"key"` or `"value"`), and the `keys`
  the object already has. There are no items then.

For CodeMirror 6, [`querydown-codemirror`](../codemirror/README.md#autocompletion) turns this into
autocompletion, including completion of annotations from a JSON Schema.

### `is_querydown_error(value: unknown): value is QuerydownError`

Returns `true` if `value` is an error thrown by this package. A TypeScript type guard; see
[Errors](#errors).

### The `CompileResult` type

```ts
interface CompileResult {
  sql: string;
  // One entry per output column, in order; null where a column has no annotation.
  columnAnnotations: (AnnotationValue | null)[];
}

type AnnotationValue =
  | null
  | boolean
  | number
  | string
  | AnnotationValue[]
  | { [key: string]: AnnotationValue };
```

## Errors

Every exported function reports failure by throwing a `QuerydownError`: a real JS `Error` (so
`instanceof Error` holds and it has a `stack`) whose `name` is `"QuerydownError"`, with these extra
properties:

| Property   | Type                                              | Meaning                                                                       |
| ---------- | ------------------------------------------------- | ----------------------------------------------------------------------------- |
| `message`  | `string`                                          | One line: optional section prefix + `reason` + `" (line L, column C)"` if located |
| `kind`     | `"syntax" \| "compile" \| "schema" \| "argument"` | The broad category of error (see below)                                      |
| `reason`   | `string`                                          | The description of the problem alone, without section or location            |
| `section`  | `QuerydownSection \| null`                        | For `compile_sections`: the section the error is in                           |
| `span`     | `SourceSpan \| null`                              | Where in the source the problem is                                            |
| `labels`   | `QuerydownErrorLabel[]`                           | Secondary locations, each with a note (e.g. where an unclosed `[` was opened) |
| `expected` | `string[]`                                        | For syntax errors: what would have been accepted at `span`                    |
| `found`    | `string \| null`                                  | For syntax errors: what was found at `span` instead                           |
| `hint`     | `string \| null`                                  | A suggestion on how to fix the problem, if available                          |

Absent values are `null` (never `undefined`). The kinds are:

- `"syntax"`: the Querydown source isn't valid syntax. **Always has a `span`.**
- `"compile"`: the source is valid but can't be compiled against the schema (e.g. it names a column
  that doesn't exist). No `span` yet, since the syntax tree doesn't record source positions.
- `"schema"`: the schema JSON is invalid. No `span`.
- `"argument"`: another argument is invalid, e.g. an unknown dialect. No `span`.

More kinds may be added in the future, so handle unknown kinds gracefully.

The generated `querydown_js.d.ts` declares these types:

```ts
type QuerydownErrorKind = "syntax" | "compile" | "schema" | "argument";
type QuerydownSection = "definitions" | "conditions" | "sorting" | "display";

interface SourcePosition {
  offset: number; // UTF-16 code units from the start of the source
  line: number;   // 1-based
  column: number; // 1-based, in UTF-16 code units
}

interface SourceSpan {
  start: SourcePosition;
  end: SourcePosition; // exclusive; equal to start for an empty span
}

interface QuerydownErrorLabel {
  span: SourceSpan;
  message: string;
}

interface QuerydownError extends Error {
  name: "QuerydownError";
  kind: QuerydownErrorKind;
  reason: string;
  section: QuerydownSection | null;
  span: SourceSpan | null;
  labels: QuerydownErrorLabel[];
  expected: string[];
  found: string | null;
  hint: string | null;
}

function is_querydown_error(value: unknown): value is QuerydownError;
```

**Positions are in UTF-16 code units**, the same units as JavaScript string indices, so
`input.slice(err.span.start.offset, err.span.end.offset)` extracts the offending text, and `line` /
`column` can be passed straight to editors such as Monaco or CodeMirror. (This differs from the Rust
API, whose offsets are in bytes; see [docs/errors.md](../../docs/errors.md).) A span may be empty
(`start.offset === end.offset`), e.g. at an unexpected end of input, so widen it if your editor
needs a non-empty range.

**Section attribution:** for `compile_sections`, a syntax error's `section` names the input it came
from (`"definitions"`, `"conditions"`, `"sorting"`, or `"display"`), and its `span` is relative to
that section's text. Compile errors can't be attributed to one section and have `section: null`.
For `compile`, `section` is always `null`.

For example, to show an error as a marker in an editor, falling back to the message:

```ts
import { compile, is_querydown_error } from "querydown-js";

try {
  const { sql } = compile(schemaJson, "postgres", input);
  showSql(sql);
  setMarkers([]);
} catch (err) {
  if (!is_querydown_error(err)) throw err;
  if (err.span) {
    const { start, end } = err.span;
    setMarkers([{
      startLineNumber: start.line,
      startColumn: start.column,
      endLineNumber: end.line,
      // An empty span (e.g. at the end of the input) is made one character wide.
      endColumn: start.offset === end.offset ? end.column + 1 : end.column,
      message: err.hint ? `${err.reason}\nHint: ${err.hint}` : err.reason,
    }]);
  } else {
    showError(err.message); // e.g. "Column `nope` not found within table `issues`."
  }
}
```

## ⚠️ Breaking change: errors are now `QuerydownError`s

Previously, failures threw a plain `Error` (with `name` `"Error"`) carrying only a `message`, and
every syntax error had the same generic message, `"Invalid querydown code"`. `introspection_sql`
threw a bare **string** for an unknown dialect. Now every exported function throws a
[`QuerydownError`](#errors) with structured `kind`, `span`, `hint`, etc., and syntax errors have
specific messages that include the location, e.g. ``"Unexpected `)`, which has no matching `(`.
Expected a value. (line 2, column 8)"``. Code that matched on the old message text, or caught
`introspection_sql`'s string, needs updating. Consumers pinning Querydown by git revision adopt
this when they bump to the revision containing this change.

## ⚠️ Breaking change: `compile` now returns an object

Previously `compile` returned a JSON **string** that callers had to `JSON.parse`. It now returns a
structured `CompileResult` **object** directly (via `serde-wasm-bindgen`), so read `result.sql`
instead of `JSON.parse(result).sql`. Errors still throw. Consumers pinning Querydown by git revision
adopt this when they bump to the revision containing this change.
