# CodeMirror

The `querydown-codemirror` package gives a [CodeMirror 6](https://codemirror.net/) editor syntax highlighting and autocompletion for Querydown. The [playground](/playground) uses it.

Both features run on Querydown's own parser, through the [JavaScript package](./javascript.md), so they follow the language's rules exactly. For example, a bare word is a column in `$title`, a string in `title:performance`, and a search term in `#issues performance`, and each is colored accordingly.

If you haven't yet, [install the package](./installation.md#install-the-codemirror-package).

## Add highlighting

1. [Load the compiler](./javascript.md#load-the-compiler), before your editor first renders.

1. Pass the JavaScript package's `tokenize` function to `querydown()`, and add the result to your editor's extensions:

    ```ts
    import { EditorView } from "@codemirror/view";
    import init, { tokenize } from "querydown-js";
    import wasmUrl from "querydown-js/querydown_js_bg.wasm?url";
    import { querydown } from "querydown-codemirror";

    await init({ module_or_path: wasmUrl });

    new EditorView({
      doc: "#issues status:open $id $title",
      extensions: [querydown({ tokenize })],
      parent: document.body,
    });
    ```

You pass `tokenize` in, rather than the package importing it, so that the package works however you load the compiler. Use the same version of `querydown-codemirror` as of `querydown-js`. If `tokenize` throws (because the compiler isn't loaded yet, say), the editor still works, only without highlighting, and the error is logged to the console once.

Besides highlighting, the extension tells CodeMirror about Querydown's comments (`//` and `/* */`), for the commands that toggle comments, and about its brackets, for `closeBrackets()` and `bracketMatching()`.

Source that doesn't parse, which source being edited often doesn't, is still highlighted. Past the point where the parser gives up, the highlighting falls back to a simpler one.

## Follow the editor's theme

The extension colors tokens with a light palette in an editor with a light theme, and with a dark palette in an editor with a dark one. You don't need to do anything, as long as CodeMirror knows which kind of theme your editor has.

CodeMirror takes a theme to be dark when the theme's extension says so, as `EditorView.theme(spec, { dark: true })` does, and as themes like `@codemirror/theme-one-dark` do. Otherwise it takes the theme to be light.

To switch themes while the editor is open, put your theme extension in a `Compartment` and reconfigure it. The highlighting follows.

## Customize the colors

1. Decide which [kinds of token](#kinds-of-token) to restyle.

1. Pass your styles to `querydownHighlighting()`, separately for light and dark themes. Each one is either a CSS color or an object of styles:

    ```ts
    import { querydown, querydownHighlighting } from "querydown-codemirror";

    querydown({
      tokenize,
      highlighting: querydownHighlighting({
        light: {
          column: "#005cc5",
          "base-table": { color: "#22863a", fontWeight: "bold", textDecoration: "underline" },
        },
        dark: { column: "#79b8ff" },
      }),
    });
    ```

- An object of styles can have `color`, `backgroundColor`, `fontWeight`, `fontStyle`, and `textDecoration`.
- A style of `null` leaves that kind of token unstyled.
- Whatever you don't mention keeps its default style, from `lightPalette` or `darkPalette`.
- A color can be a CSS custom property, such as `"var(--querydown-column)"`, which lets your stylesheet decide.

To start from nothing instead of from the default palettes, build a style from a palette of your own with `querydownHighlightStyle(palette, { themeType })`, install it with CodeMirror's `syntaxHighlighting()`, and pass `highlighting: false` to `querydown()`.

### Use a highlight style of your own

Every token carries two highlighting tags:

- one that's specific to Querydown, from `querydownTags` (such as `querydownTags.column`), and
- the closest standard tag of `@lezer/highlight` (such as `tags.propertyName`), which `standardTags` lists.

So a `HighlightStyle` written for other languages, like CodeMirror's `defaultHighlightStyle`, colors Querydown sensibly. And in a style of your own, you can target Querydown's tokens precisely:

```ts
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { querydownTags } from "querydown-codemirror";

const style = HighlightStyle.define([
  { tag: querydownTags["search-term"], color: "#b31d28", fontStyle: "italic" },
]);
```

## Add autocompletion

1. Create a `QuerydownCompleter` with your database's schema, the same JSON that you pass to `compile`. Create it once for each schema.

1. Pass it to `querydown()` as `completion`.

1. Add CodeMirror's `autocompletion()` extension, unless you use `basicSetup`, which includes it.

```ts
import { autocompletion } from "@codemirror/autocomplete";
import { EditorView } from "@codemirror/view";
import { QuerydownCompleter, tokenize } from "querydown-js";
import { querydown } from "querydown-codemirror";

const completer = new QuerydownCompleter(schemaJson);

new EditorView({
  extensions: [autocompletion(), querydown({ tokenize, completion: { completer } })],
  parent: document.body,
});
```

### What's completed

The completer reads the query with Querydown's own parser, so it knows what the word being typed is and, for a column, which table it belongs to.

| Where | What's offered |
| -- | -- |
| `#` at the start of a query | The schema's tables, and user-defined tables |
| `#` within a query, as in `$#comments` | The schema's tables, with those linked directly to the current table first |
| `$`, `\\`, a condition, or after `.` | The columns of the table in scope, with their types (and where links lead), plus computed columns and, in conditions, custom comparisons |
| `\|`, `@@` | Scalar functions, built-in and user-defined, with their usage and a description |
| `%` | Aggregate functions, or window functions after a `%%( … )` window definition |
| `@` | Built-in constants, constants defined in the query, and the parameters and local assignments of the function that the cursor is in |
| `#table(` | The foreign keys of `table` that could link it |
| `@{`, `\\\{` | Your application's annotation keys and values, if you [describe them](#complete-annotations) |

- The table in scope follows the rules of the language. Within `author.team.`, the columns offered are those of `teams`. Within `#comments issue{…}`, they're those of `issues`. In a later stage of a pipeline, they're the columns that the stage before outputs.
- Completion starts by itself once something completable has been typed: a letter, or one of `#` `.` `$` `\` `|` `%` `@`. Press <kbd>Ctrl</kbd>+<kbd>Space</kbd> to ask for it anywhere else.
- Nothing is offered within strings, comments, numbers, or flags, nor for a bare word on the right of a comparison (which is a string, as in `status:open`).
- Source that doesn't parse is still completed. The completer closes any open brackets and tries again, and failing that, it offers the columns of the base table.
- A name that isn't a plain identifier is inserted with backticks, as in `` `Due Date` ``.

### Complete annotations

Querydown passes [annotations](../language.md#annotations) through without knowing what they mean, so by itself it can't complete them. Your application knows which annotations it accepts. Describe them with a [JSON Schema](https://json-schema.org/), and their keys and values are completed too.

```ts
querydown({
  tokenize,
  completion: {
    completer,
    // For column annotations: `@{ … }`
    annotations: {
      type: "object",
      properties: {
        width: { type: "number", description: "The column's width, in pixels." },
        align: { enum: ["left", "center", "right"] },
        hidden: { type: "boolean" },
      },
    },
    // For the query annotation: `\\\{ … }`
    queryAnnotations: {
      type: "object",
      properties: { view: { enum: ["table", "board"] }, pageSize: { type: "number" } },
    },
  },
});
```

- Keys are offered from `properties`, except those that the object already has. Accepting one inserts its `:` and offers its values.
- Values are offered from `enum` and `const`, as `@true` and `@false` for a `boolean`, and as `@null` for `null`. Strings are quoted when they aren't identifiers.
- The schema can nest objects and arrays (`properties`, `additionalProperties`, `items`, `prefixItems`), list alternatives (`anyOf`, `oneOf`, `allOf`), and refer to `$defs` with `$ref`.
- A `description` is shown beside the option.

For what a schema can't express, pass a function instead. It's called with the cursor's position within the annotation, and returns the CodeMirror completions to offer:

```ts
annotations: ({ path, slot, keys }) =>
  slot === "value" && path.at(-1) === "formatter" ? formatters.map((label) => ({ label })) : null,
```

`path` holds the keys and array indices that lead to the object or array that the cursor is in, `slot` is `"key"` or `"value"`, and `keys` lists the keys that the object already has.

### Style the options

Each option gets one of CodeMirror's standard icons (`class` for tables, `property` for columns, `function`, `variable`, and so on). It also carries its Querydown kind as a CSS class, so you can style particular kinds, as in `.cm-completionIcon-computed-column` or `.cm-completionIcon-aggregate-function`. `COMPLETION_KINDS` lists the kinds.

## Edit a query in sections

If your application edits a query in sections, as passed to [`compile_sections`](./javascript.md#compile_sections), each section gets an editor of its own.

1. Tell each editor which section it holds, so that its text is read with that section's grammar: `"definitions"`, `"conditions"`, `"sorting"`, or `"display"`. The default, `"query"`, is for a whole query.

1. Give the completion the query's base table and its definitions, which live elsewhere in your interface. Pass them as values, or as functions that read them when needed:

    ```ts
    querydown({
      tokenize,
      grammar: "conditions",
      completion: {
        completer,
        baseTable: () => baseTableSelect.value,
        definitions: () => definitionsEditor.state.doc.toString(),
      },
    });
    ```

## Kinds of token

These are the kinds of token that `tokenize` produces, each of which you can style. `TOKEN_KINDS` lists them.

| Kind | Examples |
| -- | -- |
| `comment` | `// note`, `/* note */` |
| `string` | `"open"`, `'open'`, and the bare `open` in `status:open` |
| `string-flags` | the `^f` that begins `^f"Hi {name}"` |
| `string-escape` | `\n` in `^e"a\nb"`, and `{{` in `^f"{{x}}"` |
| `string-interpolation` | the `{` and `}` around `name` in `^f"Hi {name}"` |
| `search-term` | `backend` in `#issues backend`, which searches the table's text |
| `number`, `duration`, `date` | `42`, `3d`, `@2024-01-31` |
| `constant` | `@true`, `@null`, `@now` |
| `variable` | `@user_id`, a function's `@x` |
| `base-table` | `#issues` at the start of a query |
| `table` | `#comments` |
| `column` | `title`, `` `due date` `` |
| `function` | `upper` in `\|upper`, `@@max` |
| `aggregate-function` | `count` in `%count`, and window functions |
| `pipe` | `\|`, `%`, `%%` |
| `comparison-operator` | `:`, `:=`, `:>=`, `:~` |
| `operator` | `+ - * /`, `!`, `,`, `..`, `->` |
| `has-quantity` | `++`, `--` |
| `glob` | `*` in `$*` |
| `case` | `?`, `~`, `~~` |
| `path-separator` | `.` |
| `result-column` | `$` |
| `sort` | `\\` |
| `flag` | `\sd`, `\g` |
| `alias` | `total` in `-> total` |
| `annotation`, `annotation-key` | the `@` and the `width` in `@{width:100}`, and the `\\\` of a query annotation |
| `annotation-value` | the `100` in `@{width:100}`, including strings and `@true` |
| `annotation-bracket` | the brackets in `@{tags:[a b]}` |
| `pipeline` | `~~~` |
| `union` | `+++` |
| `definition` | `=` and `=>` in definitions |
| `bracket` | `( ) [ ] { }` |
| `punctuation` | the `:` in `@{width:100}` |
| `invalid` | a character that can't begin any Querydown syntax |

## What's missing

- Each edit tokenizes the whole document again. That takes about a millisecond for a typical query, so the package doesn't attempt incremental parsing, but it isn't meant for very long documents.
- Errors aren't shown in the editor for you. To underline them, compile the document as it changes and turn the [error's span](../errors.md#javascript) into a diagnostic with `@codemirror/lint`. The playground's source does this.
- There's no support for other editors, and no language server.
