# Installation

Querydown is a compiler: you give it a query and a description of your database, and it gives you SQL. This page shows you how to get the compiler, in whichever form suits how you'll use it.

| To… | Install… |
| -- | -- |
| try Querydown out, or compile queries in scripts | the [command line tool](#install-the-command-line-tool) |
| compile queries in a Rust application | the [Rust crate](#add-the-rust-crate) |
| compile queries in a browser or in Node.js | the [JavaScript package](#build-the-javascript-package) |
| give a CodeMirror editor syntax highlighting and autocompletion | the [CodeMirror package](#build-the-codemirror-package), on top of the JavaScript package |

If you only want to see what the language is like, you don't need to install anything: use the [playground](/playground).

## Before you start

Querydown is a young project, and it shows in how you install it:

- **Nothing is published to crates.io or npm yet.** For now, you install from the [Git repository](https://github.com/seancolsen/querydown).
- **There are no versioned releases yet** (see [releases](../../site/content/releases/0.0.1.md)), and the language can still change in incompatible ways. So pin the exact commit you install, and upgrade deliberately.

You need a recent stable [Rust toolchain](https://www.rust-lang.org/tools/install) for every form of Querydown, because each one is built from source.

## Pick a commit to pin

1. Find the latest commit on the `main` branch:

    ```sh
    git ls-remote https://github.com/seancolsen/querydown main
    ```

1. Copy the commit hash that it prints. The steps below call it `<commit>`.

## Install the command line tool

1. Install the `querydown-cli` package with Cargo:

    ```sh
    cargo install --git https://github.com/seancolsen/querydown --rev <commit> querydown-cli
    ```

1. Check that the `querydown` command works:

    ```sh
    querydown --version
    ```

1. Continue with the [CLI](./cli.md) page.

## Add the Rust crate

1. Add the `querydown` crate to your `Cargo.toml`, as a Git dependency:

    ```toml
    [dependencies]
    querydown = { git = "https://github.com/seancolsen/querydown", rev = "<commit>" }
    ```

1. Build your project, so that Cargo fetches and compiles the crate:

    ```sh
    cargo build
    ```

1. Continue with the [Rust](./rust.md) page.

## Build the JavaScript package

The JavaScript package is the compiler built to WebAssembly, with TypeScript types. You build it from a checkout of the repository.

1. Install [`wasm-pack`](https://rustwasm.github.io/wasm-pack/installer/), which builds Rust into a JavaScript package.

1. Add the Querydown repository to your project as a Git submodule, at the commit you picked:

    ```sh
    git submodule add https://github.com/seancolsen/querydown vendor/querydown
    git -C vendor/querydown checkout <commit>
    ```

1. Build the package:

    ```sh
    wasm-pack build vendor/querydown/bindings/js --target web
    ```

    This writes the package to `vendor/querydown/bindings/js/pkg`. The `web` target suits browsers and bundlers such as Vite. The [JavaScript](./javascript.md#choose-a-build-target) page describes the other targets.

1. Add the package to your `package.json`, as a dependency on that directory:

    ```json
    {
      "dependencies": {
        "querydown-js": "file:./vendor/querydown/bindings/js/pkg"
      }
    }
    ```

1. Install your dependencies:

    ```sh
    npm install
    ```

1. Continue with the [JavaScript](./javascript.md) page.

Run the `wasm-pack build` step again whenever you move the submodule to another commit. The built package isn't committed to your repository, so your build or CI needs to run that step too.

## Build the CodeMirror package

The CodeMirror package is written in TypeScript, and works together with the JavaScript package.

1. [Build the JavaScript package](#build-the-javascript-package), if you haven't yet.

1. Build the CodeMirror package, from the same checkout:

    ```sh
    cd vendor/querydown/bindings/codemirror
    npm install
    npm run build
    ```

1. Add it to your `package.json`, beside `querydown-js`:

    ```json
    {
      "dependencies": {
        "querydown-codemirror": "file:./vendor/querydown/bindings/codemirror",
        "querydown-js": "file:./vendor/querydown/bindings/js/pkg"
      }
    }
    ```

1. Make sure that your project depends on CodeMirror itself. The package's peer dependencies are `@codemirror/autocomplete`, `@codemirror/language`, `@codemirror/state`, `@codemirror/view`, `@lezer/common`, and `@lezer/highlight`, which an application that uses CodeMirror already has.

1. Continue with the [CodeMirror](./codemirror.md) page.

## Describe your database

Whichever form you installed, the compiler needs to know your database's tables, columns, and foreign keys before it can compile a query. You give it that as a JSON document, which Querydown can generate from your database. See [introspection](./introspection.md).
