## Validation

Run the following cargo commands in this order, fixing any errors you notice:

1. `cargo check`
2. `cargo clippy`
3. `cargo build` (this might take up to 3 minutes)
4. `cargo test`
5. `cargo test --features db-tests` (runs the corpus SQL against real Postgres & DuckDB; requires the `postgres`/`duckdb` binaries, which the dev container provides — see DEVELOPMENT.md)
6. `cargo fmt`

## Documentation

Whenever you change headings in a doc that carries a doctoc-generated table of contents (currently
`docs/language.md` and `docs/cheat-sheet.md`), regenerate its TOC so it stays in sync:

- `doctoc --notitle docs/language.md`
- `doctoc docs/cheat-sheet.md`

See DEVELOPMENT.md ("Regenerating documentation tables of contents") for details. `doctoc` is installed
in the dev container by the Dockerfile.

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
