# Querydown JavaScript bindings

WebAssembly bindings for the [Querydown](https://github.com/seancolsen/querydown) compiler,
generated with [`wasm-pack`](https://rustwasm.github.io/wasm-pack/). They let a JavaScript or
TypeScript application compile Querydown into SQL in the browser or in Node.js, and provide the
tokenizer and the completion engine that editors build on.

This package is **not published to npm** yet. Build it from a checkout of the repository:

```sh
wasm-pack build bindings/js --target web
```

That writes an ES-module package, with TypeScript types, to `bindings/js/pkg/` (which Git ignores).

## Documentation

- [Installation](../../docs/setup/installation.md#build-the-javascript-package): how to build the
  package and depend on it
- [JavaScript](../../docs/setup/javascript.md): how to load the compiler, and every function it
  exports (`compile`, `compile_sections`, `introspection_sql`, `tokenize`, `QuerydownCompleter`,
  `is_querydown_error`)
- [Errors](../../docs/errors.md#javascript): the `QuerydownError` that those functions throw
- [CodeMirror](../../docs/setup/codemirror.md): the editor package built on these bindings

The TypeScript declarations in the built package (`querydown_js.d.ts`) document every type in
detail.

## Development

The bindings are a thin layer over the `querydown` crate, in [`src/lib.rs`](./src/lib.rs). Their
tests run against a real JavaScript runtime:

```sh
wasm-pack test --node bindings/js
```

These are `wasm-bindgen-test`s, so `wasm-bindgen` and `wasm-bindgen-test` in
[`Cargo.toml`](./Cargo.toml) must stay a matched pair. A mismatch compiles fine, but the runner then
silently reports "no tests to run".

See [DEVELOPMENT.md](../../DEVELOPMENT.md) for more.
