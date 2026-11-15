# querydown

The compiler for [Querydown](https://querydown.org), a language for writing relational database
queries that compile to SQL. Querydown is succinct, and safe for the power users of an application
to write, which makes it akin to "markdown for SQL". The compiler produces SQL for PostgreSQL and
DuckDB.

```sh
cargo add querydown
```

```rust
use querydown::{Compiler, IdentifierResolution, Options, Postgres};

let schema_json = std::fs::read_to_string("schema.json")?;
let options = Options {
    dialect: Box::new(Postgres()),
    identifier_resolution: IdentifierResolution::Flexible,
    limit: None,
    offset: None,
};
let compiler = Compiler::new(&schema_json, options)?;

let result = compiler.compile("#issues status:open $title $author.username".to_string())?;
println!("{}", result.sql);
```

The compiler needs a description of your database's tables, columns, and foreign keys, as JSON,
which Querydown can [generate from your database](https://querydown.org/docs/setup/introspection).
It doesn't connect to a database itself: what you do with the SQL is up to you.

## Documentation

- [Rust](https://querydown.org/docs/setup/rust): how to use this crate
- [The language](https://querydown.org/docs/language), and a [playground](https://querydown.org/playground)
  to try it in
- [Errors](https://querydown.org/docs/errors): what the compiler reports

The compiler is also available as a [command line tool](https://crates.io/crates/querydown-cli), and
as a [JavaScript package](https://www.npmjs.com/package/querydown-js), built to WebAssembly.

## License

MIT
