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
a single row with a single column containing the JSON. Throws on an unknown dialect.

### `compile(schemaJson: string, dialect: string, input: string): CompileResult`

Compiles whole-query Querydown `input` against `schemaJson` for the given `dialect`. Returns a typed
`CompileResult` **object** (see below). Throws a JS `Error` on a parse, schema, or compile failure.

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
`CompileResult` as `compile`; throws on failure, with parse errors prefixed by their section name.

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

## ⚠️ Breaking change: `compile` now returns an object

Previously `compile` returned a JSON **string** that callers had to `JSON.parse`. It now returns a
structured `CompileResult` **object** directly (via `serde-wasm-bindgen`), so read `result.sql`
instead of `JSON.parse(result).sql`. Errors still throw. Consumers pinning Querydown by git revision
adopt this when they bump to the revision containing this change.
