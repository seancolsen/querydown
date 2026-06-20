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

