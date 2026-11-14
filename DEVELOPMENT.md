# Containerized development & testing

This project ships a Docker setup so you can do development and testing inside a container. The main reason is to run **Claude Code with full permissions** (`claude --dangerously-skip-permissions`) safely.

## How it's wired up

- The project directory is bind-mounted at `/workspace` inside the container, so edits on the host (or by the agent in the container) are immediately visible on both sides.
- Build artifacts are kept in a named volume mounted at `/workspace/target`, so the container's builds **don't clobber your host `./target`** and persist across container runs.
- The cargo registry and git caches are persisted in named volumes, so crates aren't re-downloaded every time.
- The host `~/.claude` and `~/.claude.json` are mounted into the container, so the container reuses your existing Claude Code login.

## First-time setup

Build the image (takes a few minutes — it compiles `wasm-pack` tooling and installs Node/Claude Code):

```sh
docker compose build
```

If your host UID/GID aren't 1000:1000, pass them explicitly:

```sh
docker compose build --build-arg USER_UID=$(id -u) --build-arg USER_GID=$(id -g)
```

## Daily use

Open an interactive shell in the container:

```sh
docker compose run --rm dev
```

`--rm` removes the container when you exit; the named volumes (build cache, crate cache) survive, so the next run is fast.

Inside the container you're at `/workspace` with the full toolchain so you can run cargo commands.

### Running Claude Code with full permissions

From inside the container shell:

```sh
claude --dangerously-skip-permissions
```

Because the container is isolated and only your project (plus caches) is mounted, you can let the agent run commands without approving each one.

To jump straight into Claude Code without a separate shell step:

```sh
docker compose run --rm dev claude --dangerously-skip-permissions
```

### Working on the wasm bindings

```sh
# Build the JS bindings (outputs to bindings/js/pkg)
wasm-pack build bindings/js
```

For how a downstream app builds and consumes the binding (target choice, initialization, and the
exported function signatures), see [bindings/js/README.md](bindings/js/README.md).

Run the binding's own tests against a real JS runtime with:

```sh
wasm-pack test --node bindings/js
```

These are `wasm-bindgen-test`s, so `wasm-bindgen` and `wasm-bindgen-test` in
[bindings/js/Cargo.toml](bindings/js/Cargo.toml) must stay a matched pair — a version mismatch
compiles fine but the runner silently reports "no tests to run".

### Working on the documentation and the site

The documentation is Markdown in [docs/](docs), and [site/](site) is the SvelteKit app that publishes
it (along with the home page, the release notes, and the playground) as a static website.

```sh
cd site
npm install
npm run dev     # serve the site, reloading as you edit code or Markdown
npm run build   # build it into site/build, and check it for broken links
npm run wasm    # rebuild the compiler for the site after changing Rust code
```

- [docs/README.md](docs/README.md) has the guidelines for writing documentation: its style, and its
  conventions for examples and links.
- [site/README.md](site/README.md) explains how the site works.
- The site needs Node.js 22.17 or later, which the container provides.

The dev server binds inside the container; add a port mapping to `docker-compose.yml` (e.g. `ports: ["5173:5173"]`) and run `npm run dev -- --host` if you want to reach it from the host browser.

### Documentation examples and their tests

Every fenced code block in the documentation whose language is `qd` is an example, and is tested.
The [docs_examples](compiler/src/tests/docs_examples.rs) test finds them all (in `README.md`,
`docs/`, and `site/content/`) and compiles each one against the sample issue-tracker schema, for
every dialect. With the `db-tests` feature, each example's SQL is also planned by real Postgres and
DuckDB databases, alongside the corpus (see below).

Words after `qd` in a block's info string say what's expected of it: `qd error` for an example that
must fail to compile, `qd fragment` for code that isn't a whole query, and `qd limit=50` to compile
with an option. A following block marked `sql output` or `json output` states the example's output,
which is checked too. [docs/README.md](docs/README.md#examples) documents these for authors.

So when a change to the compiler breaks an example, `cargo test` fails and names the file and line.
Fix the compiler or the docs, whichever is wrong.

### Syntax highlighting, autocompletion, and their tests

Syntax highlighting starts in the parser crate's [`highlight`](parser/src/highlight/mod.rs) module,
which classifies source into tokens using the parser itself (with a lexer as a fallback for source
that doesn't parse). The JS bindings expose it as `tokenize`, and
[bindings/codemirror](bindings/codemirror/README.md) turns it into a CodeMirror 6 language with light
and dark highlighting.

Autocompletion lives in the compiler crate's [`completion`](compiler/src/compiler/completion/mod.rs)
module, since it needs the schema. It parses the source with a sentinel identifier in place of the
word being typed, finds the sentinel in the syntax tree, and works out from its surroundings what it
is and which table is in scope. The JS bindings expose it as `QuerydownCompleter`, and the CodeMirror
package turns that into a completion source (completing column and query annotations from JSON
Schemas itself). Its Rust tests mark the cursor with `‸` in each source and run with `cargo test`.

When you change the grammar, think about both:

- **Highlighting:** a new construct that introduces a new role for some text needs a `mark(...)` in
  the parser (and maybe a new `TokenKind`), or it will be highlighted only by the lexer's guess.
- **Completion:** a new construct that contains names, or changes which table is in scope, needs
  handling in the completion module's syntax-tree walk (`locate.rs`). Likewise, a new built-in
  function needs a description in `catalog.rs` (a test fails until it has one).

The CodeMirror package has Playwright tests that render sample queries in light and dark editors and
compare them against committed screenshots in
[bindings/codemirror/test/snapshots](bindings/codemirror/test/snapshots), and that type into an
editor to check what it completes (with a couple of screenshots of the completion popup). Run them
inside the container, which provides the headless Chromium they need (screenshots taken elsewhere
won't match pixel for pixel):

```sh
cd bindings/codemirror
npm install
npm run build:wasm          # builds the bindings into test/wasm; rerun after changing Rust code
npm run test:visual         # compares against the committed screenshots
npm run test:visual:update  # rewrites the screenshots after an intended change; review the PNGs!
npm run test:visual:serve   # serves the samples on port 4183 to look at directly (?theme=dark),
                            # and an editor to try completion in at /completion.html
```

The comparison is strict (any changed pixel fails), so a failure is a real change in rendering. Open
the `-diff.png` under `test-results/` to see what changed. To cover a new construct, add source that
uses it to [the samples](bindings/codemirror/test/harness/samples.ts).

## Database tests (`db-tests` feature)

The corpus tests in [compiler/src/tests/corpus.md](compiler/src/tests/corpus.md) normally just assert
that compiled SQL matches an expected string. The optional **`db-tests`** feature goes further: it runs
each case's compiled SQL against **real Postgres and DuckDB** databases to catch syntax and
query-planning bugs a string comparison can't.

```sh
# Must be run inside the Docker container to work!
cargo test --features db-tests
```

How it works:

- Every case is compiled under **both** dialects (Postgres and DuckDB) — not just the dialect it
  declares for its string-match assertion — and each result is validated with `EXPLAIN`, so queries are
  planned but never executed. No data is inserted; empty, correctly typed tables are enough.
- The test starts a throwaway Postgres server (unix socket in a temp dir) and an in-memory DuckDB
  database once, loads the schemas, runs all cases, then tears everything down. DuckDB is compiled
  and statically linked via the bundled `duckdb` crate rather than the `duckdb` CLI, so the tests
  exercise the same library a downstream application would embed — notably *without* the ICU
  extension that the official CLI bundles.
- Table structure comes from hand-authored DDL in
  [compiler/resources/test/](compiler/resources/test/): `issue_schema.sql` and `library_schema.sql`.
  **When you add or rename a column in a `*_schema.json`, make the matching change in its `*_schema.sql`**
  (an unknown column surfaces as an `EXPLAIN` failure).
- The `postgres` binaries are installed by the [Dockerfile](Dockerfile); DuckDB is built from
  source by the bundled `duckdb` crate (the first build takes a while). The feature is off by
  default so a plain `cargo test` doesn't need Postgres.

To exclude a specific case that intentionally produces SQL which won't plan against the minimal schema,
add a `db_skip` to its ```` ```toml options ```` block: `db_skip = true` (all engines) or
`db_skip = ["duckdb"]` (one dialect). The case still runs its normal SQL string-match assertion.

This setup is also the foundation for future data-driven E2E tests: seed rows into the tables and assert
on result sets instead of just running `EXPLAIN`.

## Optional: VS Code / Codespaces dev container

If you use VS Code, [.devcontainer/devcontainer.json](.devcontainer/devcontainer.json) lets you run your **whole editor** inside this same container instead of opening a shell with `docker compose run`. It's a supplement — it reuses the exact same `docker-compose.yml` (Dockerfile, cache volumes, UID matching, entrypoint), so nothing about the CLI workflow above changes.

With the **Dev Containers** extension installed, open the Command Palette and choose **"Dev Containers: Reopen in Container."** VS Code builds/starts the compose service, installs `rust-analyzer` and the Claude Code extension inside it, and reopens the workspace at `/workspace`. Your terminal, language server, and Claude Code now all run in the container — so the agent's command execution is confined there too, which is the same isolation goal as running `claude --dangerously-skip-permissions` in the shell.

Notes specific to the dev container:

- It sets `overrideCommand: true` so VS Code keeps the container alive with its own keep-alive process; our `entrypoint.sh` still runs first, so the cache-volume ownership fix still applies.
- The shared `~/.claude` credential mount works the same locally. In **cloud Codespaces** there's no host `~/.claude` to mount, so you'd log into Claude Code separately inside the Codespace.
- This file is only meaningful to VS Code / Codespaces / the `devcontainer` CLI. Plain `docker compose` users can ignore it.

## Maintenance

- Rebuild the image after changing the `Dockerfile`:

    `docker compose build`
  
- Wipe the cached build artifacts and crates (forces a clean rebuild):

    `docker compose down -v`

- Bump the Rust version by editing the `FROM rust:1.91-bookworm` line in the `Dockerfile` to match a new host toolchain.
