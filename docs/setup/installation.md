# Installation

Querydown is a compiler: you give it a query and a description of your database, and it gives you SQL. This page shows you how to install the compiler, in whichever form suits how you'll use it.

| To… | Install… | From |
| -- | -- | -- |
| try Querydown out, or compile queries in scripts | the [command line tool](#install-the-command-line-tool) | crates.io |
| compile queries in a Rust application | the [Rust crate](#add-the-rust-crate) | crates.io |
| compile queries in a browser or in Node.js | the [JavaScript package](#install-the-javascript-package) | npm |
| give a CodeMirror editor syntax highlighting and autocompletion | the [CodeMirror package](#install-the-codemirror-package), on top of the JavaScript package | npm |

If you only want to see what the language is like, you don't need to install anything: use the [playground](/playground).

## Before you start

Querydown is a young project. Until version 1.0, a release that changes the minor version (from 0.1 to 0.2, say) can break queries and code written for the one before. A release that changes only the patch version (from 0.1.0 to 0.1.1) doesn't.

- Cargo and npm both take a version such as `0.1.0` to mean "0.1.0 or a later 0.1 release", so they'll update you to fixes, but not across a release that can break things.
- Before you move to a new minor version, read its [release notes](../../site/content/releases/0.1.0.md), which say what changed and what to do about it.

Every package of a release has the same version number. Use the same version of each package that you install.

## Install the command line tool

1. Install a recent stable [Rust toolchain](https://www.rust-lang.org/tools/install), if you don't have one. There are no prebuilt binaries of the tool yet.

1. Install the `querydown-cli` package with Cargo:

    ```sh
    cargo install querydown-cli
    ```

1. Check that the `querydown` command works:

    ```sh
    querydown --version
    ```

1. Continue with the [CLI](./cli.md) page.

## Add the Rust crate

1. Add the `querydown` crate to your project:

    ```sh
    cargo add querydown
    ```

1. Continue with the [Rust](./rust.md) page.

The crate's API documentation is on [docs.rs](https://docs.rs/querydown).

## Install the JavaScript package

The `querydown-js` package is the compiler built to WebAssembly, with TypeScript types. It works in browsers, with or without a bundler such as Vite, and in Node.js.

1. Install it:

    ```sh
    npm install querydown-js
    ```

1. Continue with the [JavaScript](./javascript.md) page, which shows you how to load it.

## Install the CodeMirror package

The `querydown-codemirror` package is written in TypeScript, and works together with the JavaScript package.

1. Install it along with the JavaScript package:

    ```sh
    npm install querydown-codemirror querydown-js
    ```

1. Make sure that your project depends on CodeMirror itself. The package's peer dependencies are `@codemirror/autocomplete`, `@codemirror/language`, `@codemirror/state`, `@codemirror/view`, `@lezer/common`, and `@lezer/highlight`, which an application that uses CodeMirror already has.

1. Continue with the [CodeMirror](./codemirror.md) page.

## Build from a commit

To use a change that hasn't been released yet, or to build the JavaScript package for a [`wasm-pack` target](./javascript.md#other-build-targets) other than `web`, build Querydown from its [Git repository](https://github.com/seancolsen/querydown) instead. Pin the exact commit, since the code between releases can change in ways that break things.

1. Find the latest commit on the `main` branch, and copy the commit hash that it prints. The steps below call it `<commit>`.

    ```sh
    git ls-remote https://github.com/seancolsen/querydown main
    ```

1. For the command line tool, install it from the commit:

    ```sh
    cargo install --git https://github.com/seancolsen/querydown --rev <commit> querydown-cli
    ```

1. For the Rust crate, depend on the commit in your `Cargo.toml`:

    ```toml
    [dependencies]
    querydown = { git = "https://github.com/seancolsen/querydown", rev = "<commit>" }
    ```

1. For the JavaScript packages, you need [`wasm-pack`](https://rustwasm.github.io/wasm-pack/installer/) and a Rust toolchain. Add the repository to your project as a Git submodule, at the commit, and build the packages from it:

    ```sh
    git submodule add https://github.com/seancolsen/querydown vendor/querydown
    git -C vendor/querydown checkout <commit>
    wasm-pack build vendor/querydown/bindings/js --target web
    (cd vendor/querydown/bindings/codemirror && npm install && npm run build)
    ```

    Then depend on the directories that those commands built:

    ```json
    {
      "dependencies": {
        "querydown-codemirror": "file:./vendor/querydown/bindings/codemirror",
        "querydown-js": "file:./vendor/querydown/bindings/js/pkg"
      }
    }
    ```

    Build them again whenever you move the submodule to another commit. The built packages aren't committed to your repository, so your build or CI needs to build them too.

## Describe your database

Whichever form you installed, the compiler needs to know your database's tables, columns, and foreign keys before it can compile a query. You give it that as a JSON document, which Querydown can generate from your database. See [introspection](./introspection.md).
