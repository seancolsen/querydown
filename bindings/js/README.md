# querydown-js

The [Querydown](https://querydown.org) compiler, built to WebAssembly. It lets a JavaScript or
TypeScript application compile Querydown queries into SQL in a browser, with no server involved, or
in Node.js. It also provides the tokenizer and the completion engine that editors build on. It comes
with TypeScript types.

Querydown is a language for writing relational database queries that compile to SQL, succinct and
safe enough for the power users of an application to write.

```sh
npm install querydown-js
```

```ts
import init, { compile } from "querydown-js";
import wasmUrl from "querydown-js/querydown_js_bg.wasm?url"; // With Vite

await init({ module_or_path: wasmUrl });

const result = compile(schemaJson, "postgres", "#issues status:open $title");
console.log(result.sql);
```

The package is built for `wasm-pack`'s `web` target: call `init()` once, before anything else. In
Node.js, pass `init()` the contents of the `.wasm` file instead of its URL.

## Documentation

- [JavaScript](https://querydown.org/docs/setup/javascript): how to load the compiler, and every
  function the package exports (`compile`, `compile_sections`, `introspection_sql`, `tokenize`,
  `QuerydownCompleter`, `is_querydown_error`)
- [Errors](https://querydown.org/docs/errors#javascript): the `QuerydownError` that those functions
  throw
- [Introspection](https://querydown.org/docs/setup/introspection): how to describe your database to
  the compiler
- [The language](https://querydown.org/docs/language), and a [playground](https://querydown.org/playground)
  to try it in
- [querydown-codemirror](https://www.npmjs.com/package/querydown-codemirror): syntax highlighting
  and autocompletion for CodeMirror 6, built on this package

The TypeScript declarations in the package (`querydown_js.d.ts`) document every type in detail.

## Development

The bindings are a thin layer over the [`querydown`](https://crates.io/crates/querydown) crate, in
[`bindings/js/src/lib.rs`](https://github.com/seancolsen/querydown/blob/main/bindings/js/src/lib.rs).
To build them from a checkout of the repository:

```sh
wasm-pack build bindings/js --target web
```

That writes the package to `bindings/js/pkg/` (which Git ignores). Their tests run against a real
JavaScript runtime:

```sh
wasm-pack test --node bindings/js
```

These are `wasm-bindgen-test`s, so `wasm-bindgen` and `wasm-bindgen-test` in `Cargo.toml` must stay
a matched pair. A mismatch compiles fine, but the runner then silently reports "no tests to run".

See [DEVELOPMENT.md](https://github.com/seancolsen/querydown/blob/main/DEVELOPMENT.md) for more.

## License

MIT
