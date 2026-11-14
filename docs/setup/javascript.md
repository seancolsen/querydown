# JavaScript

The `querydown-js` package is the Querydown compiler, built to WebAssembly. It lets a JavaScript or TypeScript application compile queries in the browser, with no server involved, or in Node.js. It comes with TypeScript types.

If you haven't yet, [build the package](./installation.md#build-the-javascript-package).

## Choose a build target

When you build the package, the `--target` that you give `wasm-pack` decides how your code loads the WebAssembly.

| Target | Use it… | Loading |
| -- | -- | -- |
| `web` | in a browser, with or without a bundler such as Vite. **Start here.** | You call `init()` once before anything else. |
| `bundler` | if your bundler handles WebAssembly modules itself | Nothing to call, but Vite needs [`vite-plugin-wasm`](https://github.com/Menci/vite-plugin-wasm) and `vite-plugin-top-level-await`. |

The rest of this page assumes the `web` target. With the `bundler` target, leave out the call to `init()`.

## Load the compiler

1. Import the package's `init` function, and the URL of the `.wasm` file. With Vite, a `?url` import gives you the URL of a file that Vite serves:

    ```ts
    import init from "querydown-js";
    import wasmUrl from "querydown-js/querydown_js_bg.wasm?url";
    ```

1. Call `init` once, when your application starts, and wait for it:

    ```ts
    await init({ module_or_path: wasmUrl });
    ```

    Every other function of the package throws if you call it before `init` has finished.

In Node.js there's no URL to fetch, so pass `init` the contents of the file instead:

```ts
import { readFile } from "node:fs/promises";
import init from "querydown-js";

const wasm = await readFile(new URL(import.meta.resolve("querydown-js/querydown_js_bg.wasm")));
await init({ module_or_path: wasm });
```

## Compile a query

1. Get your database's schema as a JSON string. To begin with, [generate it with the CLI](./cli.md#generate-your-schema-file). Later, your application can run the [introspection query](#introspection_sql) itself.

1. Compile a query for your dialect, `"postgres"` or `"duckdb"`:

    ```ts
    import { compile } from "querydown-js";

    const result = compile(schemaJson, "postgres", "#issues status:open $title @{width:300}");
    ```

1. Run `result.sql` against your database. Querydown doesn't connect to a database itself.

    ```ts
    console.log(result.sql);
    console.log(result.columnAnnotations); // [{ width: 300 }]
    ```

1. Catch the error that `compile` throws when the query is invalid, and show it to whoever wrote the query:

    ```ts
    import { compile, is_querydown_error } from "querydown-js";

    try {
      run(compile(schemaJson, "postgres", input).sql);
    } catch (error) {
      if (!is_querydown_error(error)) throw error;
      showMessage(error.hint ? `${error.message} ${error.hint}` : error.message);
    }
    ```

    The error also says where in the query the problem is. See [errors](../errors.md#javascript).

## Functions

### `compile`

```ts
function compile(
  schemaJson: string,
  dialect: string,
  input: string,
  options?: QuerydownCompileOptions,
): CompileResult;
```

Compiles a whole query, `input`, against the schema, for a dialect (`"postgres"` or `"duckdb"`). It throws a [`QuerydownError`](../errors.md#javascript) if the query, the schema, or another argument is invalid.

The `options` control pagination, which belongs to your application rather than to the query's author. See [limit and offset](../language.md#limit-and-offset).

| Option | Type | Meaning |
| -- | -- | -- |
| `limit` | `number \| null` | The most rows the query may return |
| `offset` | `number \| null` | The number of rows to skip before returning any |

Each must be a non-negative integer. Leave one out, or pass `null`, for no limit or no offset.

```ts
// The third page of 50 issues
const { sql } = compile(schemaJson, "postgres", String.raw`#issues \\id $title`, {
  limit: 50,
  offset: 100,
});
```

The result has the SQL, and the [annotations](../language.md#annotations) that the query carries:

```ts
interface CompileResult {
  sql: string;
  // One entry for each column of the results, in order. `null` for a column without an annotation.
  columnAnnotations: (AnnotationValue | null)[];
  // The query's own annotation, or `null` if it has none.
  queryAnnotation: AnnotationValue | null;
}

type AnnotationValue =
  | null
  | boolean
  | number
  | string
  | AnnotationValue[]
  | { [key: string]: AnnotationValue };
```

### `compile_sections`

```ts
function compile_sections(
  schemaJson: string,
  dialect: string,
  baseTable: string,
  definitions: string,
  conditions: string,
  sorting: string,
  display: string,
  options?: QuerydownCompileOptions,
): CompileResult;
```

Compiles a query that your application holds in parts, as a query builder with several inputs would, instead of as one piece of text.

| Parameter | Holds |
| -- | -- |
| `baseTable` | The name of the query's base table, without a `#` |
| `definitions` | [Definitions](../language.md#variables): constants, functions, computed columns, and so on |
| `conditions` | The [conditions](../language.md#comparisons-and-conditions) |
| `sorting` | [Sorting expressions](../language.md#sorting-outside-of-result-columns), each prefixed with `\\` |
| `display` | The [result columns](../language.md#result-columns), each prefixed with `$` |

Each section is parsed on its own, so the syntax of one can't leak into another: a `$` typed into the sorting input is an error, not a result column. An error [names the section](../errors.md#errors-in-a-section) it's in.

It takes the same `options` and returns the same `CompileResult` as [`compile`](#compile), though `queryAnnotation` is always `null`, as the sections have nowhere to write one.

The `definitions` section is also where your application can [supply definitions for its users](./rust.md#supply-definitions-for-your-users), such as custom comparisons, ahead of what they write.

### `introspection_sql`

```ts
function introspection_sql(dialect: string): string;
```

Returns the SQL query that makes a database of the given dialect describe itself. Run it against your database, and it returns one row with one column: the schema JSON to pass to `compile`. See [introspection](./introspection.md).

### `tokenize`

```ts
function tokenize(source: string, grammar: QuerydownGrammar): QuerydownToken[];
```

Classifies Querydown source into tokens, for **syntax highlighting**. The `grammar` says how to read the source: `"query"` for a whole query, or `"definitions"`, `"conditions"`, `"sorting"`, or `"display"` for one section of a query, as passed to [`compile_sections`](#compile_sections).

```ts
tokenize("#issues title:performance backend", "query");
// [ { from: 0, to: 7, kind: "base-table" }, { from: 8, to: 13, kind: "column" },
//   { from: 13, to: 14, kind: "comparison-operator" }, { from: 14, to: 25, kind: "string" },
//   { from: 26, to: 33, kind: "search-term" } ]
```

- The tokens are in order, don't overlap, and cover every character except whitespace.
- `from` and `to` are offsets in UTF-16 code units, like the indices of a JavaScript string.
- `kind` is one of the [kinds of token](./codemirror.md#kinds-of-token).
- The tokens come from Querydown's own parser, so they follow the language's rules exactly. Above, `performance` is a string because it's on the right of a comparison, while `backend` is a search term because it stands alone.
- `tokenize` doesn't throw on invalid Querydown, which source being edited usually is. Past the point where the parser gives up, the tokens fall back to a simpler classification.

For CodeMirror, the [CodeMirror package](./codemirror.md) turns this function into a language with highlighting.

### `QuerydownCompleter`

```ts
class QuerydownCompleter {
  constructor(schemaJson: string);
  complete(source: string, cursor: number, options?: QuerydownCompletionOptions): QuerydownCompletions;
  free(): void;
}
```

Lists what could be typed at a position in a query, for **autocompletion**. Create one completer for each schema and reuse it, since creating it parses the schema.

```ts
const completer = new QuerydownCompleter(schemaJson);
completer.complete("#issues $author.us", 18);
// { from: 16, to: 18, annotation: null, items: [
//   { label: "id", kind: "column", detail: "integer", insert: null, info: null, boost: 0 },
//   { label: "username", kind: "column", detail: "text", insert: null, info: null, boost: 0 },
//   … ] }
```

`cursor` is an offset in UTF-16 code units. What's offered depends on what's being typed there: tables after `#`, the columns of the table in scope after `$` or `.` or in a condition, functions after `|` or `%` or `@@`, and variables after `@`. The source is read by Querydown's own parser, so the columns offered within `#comments issue{…}` are those of `issues`.

The `options` are for completing one section of a query:

| Option | Meaning |
| -- | -- |
| `grammar` | How to read `source`, as for [`tokenize`](#tokenize). The default is `"query"`. |
| `baseTable` | The query's base table, whose columns are offered. For any section but `"definitions"`. |
| `definitions` | The text of the definitions section, so that what it defines is offered. For any section but `"definitions"`. |

The result:

| Property | Meaning |
| -- | -- |
| `from`, `to` | The range that an item would replace: the word being typed, up to the cursor |
| `items` | What could be typed there. They're **not** filtered by what has been typed so far, which is left to your editor. |
| `annotation` | Where the cursor is within an [annotation](../language.md#annotations), if it's in one. There are no items then, because only your application knows what its annotations may hold. |

Each item has a `label` and a `kind` (`"table"`, `"column"`, `"computed-column"`, `"custom-comparison"`, `"function"`, `"aggregate-function"`, `"window-function"`, `"constant"`, or `"variable"`). It may also have an `insert` text that differs from the label (a name quoted with backticks, say), a `detail` (such as a column's type), an `info` (such as what a function does), and a `boost` that ranks it above or below equally good matches.

The completer's memory is released when it's garbage collected. Call `free()` to release it sooner.

For CodeMirror, the [CodeMirror package](./codemirror.md#add-autocompletion) turns the completer into autocompletion, and completes annotations too, from a JSON Schema.

### `is_querydown_error`

```ts
function is_querydown_error(value: unknown): value is QuerydownError;
```

Tells you whether a caught value is an error thrown by this package. See [errors](../errors.md#javascript).

## What's missing

- The package isn't on npm yet, so you [build it yourself](./installation.md#build-the-javascript-package).
- The `.wasm` file is about 1 MB (under 400 kB compressed), so load it only on the pages that compile queries.
- There are no bindings for other languages yet. From a language without bindings, you can run the [CLI](./cli.md) as a subprocess.
