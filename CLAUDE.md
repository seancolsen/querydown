## Validation

Run the following cargo commands in this order, fixing any errors you notice:

1. `cargo check`
2. `cargo clippy`
3. `cargo build` (this might take up to 3 minutes)
4. `cargo test`
5. `cargo test --features db-tests` (runs the corpus SQL against real Postgres & DuckDB; requires the `postgres`/`duckdb` binaries, which the dev container provides — see DEVELOPMENT.md)
6. `cargo fmt`

## Documentation

The documentation is Markdown in `docs/` (plus the home page and release notes in `site/content/`),
published as a website by the SvelteKit app in `site/`. **Before writing or changing documentation,
read [docs/README.md](docs/README.md)**, which holds its style guidelines, its conventions for
examples and links, and how it's tested.

In short:

- Every ```` ```qd ```` code block in the docs is an example, and `cargo test` compiles each one
  (see `compiler/src/tests/docs_examples.rs`). An example that should fail to compile is marked
  ```` ```qd error ````.
- When you change the language, update the docs to match: `docs/language.md`, and where relevant
  `docs/cheat-sheet.md`, `docs/functions.md`, and the list of features in `site/content/home.md`.
  Add the change to the release notes too (see [Release notes](#release-notes)).
- After changing documentation (or the site), build the site, which renders every page and checks
  every link: `cd site && npm install && npm run build`. This needs Node.js 22.17 or later.

## Branches

querydown.org is published from the `stable` branch, which holds the latest release plus changes to
the docs and the site that are true of it. Development happens on `main`. The "Branches" section of
[DEVELOPMENT.md](DEVELOPMENT.md#branches) explains the strategy.

- Work on the branch you're on (`git branch --show-current`). Don't switch branches unless asked,
  or to make a change to the docs on `stable` (below).
- On `main`, anything goes. When you change the language, update its docs there, never on `stable`.
- On `stable`, change only the docs, the site, and its content, and only in ways that are true of
  the released version. Never change the parser, compiler, CLI, or bindings there, and never
  document anything that isn't released yet.
- After committing to `stable`, merge it into `main`, and return to `stable`
  (`git switch main && git merge stable && git switch stable`). If `main` is checked out in another
  worktree, so that you can't, say in your response that `stable` needs merging.
- Never merge `main` into `stable`, or push either one. `stable` only moves to `main` when a release
  is cut (see `RELEASING.md`).
- Make a change only to the docs or the site on `stable` if it's true of the released version, even
  when you're asked for it on `main`. Don't touch the release notes for it. From `main`, that's:
  `git switch stable`, make the change and commit it, then `git switch main && git merge stable`,
  ending on `main`. If `main` has uncommitted changes, or you can't switch to `stable` (because it's
  checked out in another worktree, say), ask what to do instead.
- Docs that describe a change to Querydown, or anything not yet released, go on `main` with it.

## Releases

[RELEASING.md](RELEASING.md) describes how a release is cut and published to crates.io and npm, with
`scripts/release.sh`. Every crate and npm package shares the version in the root `Cargo.toml`;
change it only with `scripts/release.sh prepare <version>`. Never publish anything, or push a tag,
unless asked to.

## Release notes

The notes for each release are in `site/content/releases/<version>.md`. Write the notes for the next
release on `main`, as the changes land, rather than all at once when it's cut.

- Whenever you add a feature, fix a bug, or otherwise change Querydown in a way that someone using
  it could notice (the language, the SQL it compiles to, the CLI, the bindings, the schema JSON, or
  the Rust API), add it to the next release's notes, in the same commit as the change. Refactoring,
  tests, and the development tooling don't need notes. Nor do changes only to the docs or the site:
  those go on `stable` (see [Branches](#branches)), which never has notes for an unreleased version.
- The next release's notes are the file that still says "_In development. Not yet released._" If
  there isn't one, start it. Name it after the next version, chosen as in "Choosing the version" of
  [RELEASING.md](RELEASING.md) from the latest release (the version in the root `Cargo.toml`), and
  start it from the template that `scripts/release.sh prepare` writes. Don't run `prepare` itself,
  since it changes the version. If a change breaks something and the notes are for a patch version,
  rename them (`git mv`) to the next minor version, and fix their title.
- Follow the earlier notes and the "Finish the release notes" step of RELEASING.md. Put the change
  under "What's new", "Fixes", or "Changes that break things" (which says what to do instead),
  linking to the docs. Keep the summary at the top, and any table of who needs to change anything,
  true of the whole release.
- The notes are on the site, so build it after changing them, as for the docs.

## Git commits

Whenever you change files in the repo, make a Git commit at the end of your turn, without being asked. Commit straight to the current branch (usually `main`), and don't push.

Write your final response first, then use it in the commit message, formatted like this:

```
<Short imperative summary>

<My prompt, verbatim>

---

<Your final response to me, in markdown, verbatim>
```

- Copy my prompt exactly, including its paragraph breaks. Don't summarize or reword it.
- The response in the commit should be the same text you show me at the end of the turn.
- Pass the message with `git commit -F <file>` (write the file to your scratchpad), so markdown, backticks and quotes survive the shell.
- If your work includes refactoring or unrelated bug fixes, try to separate that work out into separate commits that can stand on their own. Only do this if it's easy though.
