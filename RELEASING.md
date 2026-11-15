# Releasing Querydown

This is how a maintainer cuts a release. [`scripts/release.sh`](./scripts/release.sh) does the
mechanical parts. The judgment calls (the version number, and the release notes) are yours.

## What a release publishes

Every package shares one version number, and they're all released together, even when some of them
haven't changed.

| Package | Where | Built from |
| -- | -- | -- |
| [`querydown-parser`](https://crates.io/crates/querydown-parser) | crates.io | [`parser/`](./parser) |
| [`querydown`](https://crates.io/crates/querydown) | crates.io, with its API docs on [docs.rs](https://docs.rs/querydown) | [`compiler/`](./compiler) |
| [`querydown-cli`](https://crates.io/crates/querydown-cli) | crates.io | [`cli/`](./cli) |
| [`querydown-js`](https://www.npmjs.com/package/querydown-js) | npm | [`bindings/js/`](./bindings/js), built by `wasm-pack` for its `web` target |
| [`querydown-codemirror`](https://www.npmjs.com/package/querydown-codemirror) | npm | [`bindings/codemirror/`](./bindings/codemirror) |

Each release also gets:

- a Git tag, `v<version>`, on the commit that was published,
- a [GitHub release](https://github.com/seancolsen/querydown/releases) with its notes, and
- a page of release notes on the site, from `site/content/releases/<version>.md`.

The crates depend on each other at exactly the same version (the pins are in
`[workspace.dependencies]` of the root `Cargo.toml`). `querydown-codemirror` names `querydown-js` as
an optional peer dependency, at the same minor version.

## Choosing the version

Querydown follows [semantic versioning](https://semver.org/), as it applies before 1.0:

- **Bump the minor version** (0.1.3 → 0.2.0) for a release that breaks something: a query that used
  to compile no longer does, or compiles to SQL that means something different; or the Rust or
  JavaScript API, the schema's JSON, or the command line changes incompatibly.
- **Bump the patch version** (0.1.3 → 0.1.4) for anything else: fixes, new functions, new syntax
  that didn't compile before, and new API.
- For a release candidate, add a suffix, as in `0.2.0-rc.1`. It's published to npm with the
  dist-tag `next` instead of `latest`, so `npm install` doesn't pick it.

Never reuse a version number, even for a release that was yanked.

## Before your first release

1. **crates.io.** Have an account (it signs in with GitHub) that owns the crates. To add another
   owner, run `cargo owner --add <github-user> <crate>` for each crate.
1. **npm.** Have an account with two-factor authentication on, which owns the packages. To add
   another owner, run `npm owner add <npm-user> <package>`.
1. **The dev container.** Run the release from it (see [DEVELOPMENT.md](./DEVELOPMENT.md)), since
   it has everything that the tests need: PostgreSQL, `wasm-pack`, Node.js, and Playwright's
   Chromium.

## Cutting a release

1. **Pick the version**, as above, and look over what's changed since the last release:

    ```sh
    git log --oneline v0.1.0..main
    ```

1. **Set the version** of every package. This updates `Cargo.toml`, `Cargo.lock`, and the
   CodeMirror package's `package.json` and lockfile, and starts the release notes if they don't
   exist yet:

    ```sh
    scripts/release.sh prepare 0.2.0
    ```

1. **Write the release notes**, in `site/content/releases/0.2.0.md`. Follow the earlier releases:
    - The paragraph after the title sums up the release. The site uses it as the page's
      description.
    - List what's new, with links to the docs.
    - List every change that breaks a query or code written for the previous release, and say what
      to do instead. Be complete here. It's what people upgrading read.
    - Replace the note "_In development. Not yet released._" with the date of the release, as in
      "_Released on 2026-11-20._". The script won't publish while the note's still there.

1. **Check the docs.** Anything that mentions a version (such as the "Status" sections of
   `README.md` and `site/content/home.md`) should still be true.

1. **Commit**, with a message such as "Release 0.2.0".

1. **Verify.** This runs every test (Rust, against real PostgreSQL and DuckDB; the bindings; the
   CodeMirror package's screenshots; and the site's build), builds every package, and does a dry
   run of publishing each one. It takes a while.

    ```sh
    scripts/release.sh verify
    ```

    If anything fails, fix it, commit, and run `verify` again. `publish` refuses to run unless
    `verify` has passed at the current commit.

1. **Log in** to both registries, in the container:

    ```sh
    cargo login    # Paste a token from https://crates.io/settings/tokens
    npm login
    ```

    For crates.io, make a token that expires soon, with the scopes `publish-new` and
    `publish-update`, limited to the crates `querydown*`.

1. **Publish.** The script asks you to confirm, then publishes the crates in the order they depend
   on each other, then the npm packages (npm asks for your second factor), and then tags the
   commit:

    ```sh
    scripts/release.sh publish
    ```

    If it stops partway, fix the problem and run it again: it skips whatever is already
    published. Don't change the commit in between, since part of it is already out.

1. **Push** the commit and the tag, from wherever you can push to GitHub:

    ```sh
    git push origin main v0.2.0
    ```

1. **Create the GitHub release.** With the [GitHub CLI](https://cli.github.com/) installed, this
   creates it from the release notes. Without it, the script tells you what to paste where.

    ```sh
    scripts/release.sh github-release
    ```

1. **Deploy the site**, so that its docs and release notes match what's published.

## After a release

Check that the release works for someone who installs it:

- The pages of the crates on crates.io and the packages on npm show the new version and their
  READMEs.
- docs.rs has [built the docs](https://docs.rs/crate/querydown/latest) for `querydown`. (It takes
  a few minutes, and failures are shown there.)
- `cargo install querydown-cli` installs a `querydown` whose `--version` is the new one.
- A new project can `npm install querydown-js querydown-codemirror` and compile a query, following
  [the JavaScript docs](./docs/setup/javascript.md).

Then log out of the registries in the container (`cargo logout` and `npm logout`), or revoke the
crates.io token.

## When something goes wrong

A published version can't be changed or reused, so fix a bad release with a new one:

1. Fix the problem, and release a patch version.
1. If the bad version is harmful, steer people away from it:
    - On crates.io: `cargo yank --version 0.2.0 querydown` (for each crate). Yanking keeps existing
      lockfiles working, but stops new projects from choosing the version.
    - On npm: `npm deprecate querydown-js@0.2.0 "Use 0.2.1, which fixes …"` (for each package).
      npm allows unpublishing only within 72 hours, and only for packages that nothing depends on.
      Prefer deprecating.
1. Say so in the release notes of both versions.

## Possible improvements

- **Publish from GitHub Actions**, with [trusted publishing](https://crates.io/docs/trusted-publishing)
  on crates.io and npm, instead of from a maintainer's machine. That needs no long-lived tokens, and
  npm marks the packages with their provenance. Both registries require a package to exist before
  it can be published this way, so it's possible only after the first release. It also needs CI
  that can run the tests (PostgreSQL, Playwright), which the repository doesn't have yet.
- **Attach prebuilt binaries of the CLI** to each GitHub release (with
  [cargo-dist](https://opensource.axo.dev/cargo-dist/), say), so that installing it doesn't need a
  Rust toolchain.
