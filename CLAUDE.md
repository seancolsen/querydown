## Validation

Run the following cargo commands in this order, fixing any errors you notice:

1. `cargo check`
2. `cargo clippy`
3. `cargo build` (this might take up to 3 minutes)
4. `cargo test`
5. `cargo test --features db-tests` (runs the corpus SQL against real Postgres & DuckDB; requires the `postgres`/`duckdb` binaries, which the dev container provides — see DEVELOPMENT.md)
6. `cargo fmt`

