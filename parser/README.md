# querydown-parser

The parser of [Querydown](https://querydown.org), a language for writing relational database queries
that compile to SQL. It turns Querydown source into a syntax tree, and classifies source into tokens
for syntax highlighting.

This crate is an internal part of the [`querydown`](https://crates.io/crates/querydown) compiler,
and is published only because the compiler depends on it. Its API can change in any release. To
compile Querydown, use the `querydown` crate.

## License

MIT
