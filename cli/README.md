# querydown-cli

The `querydown` command, which compiles [Querydown](https://querydown.org) queries to SQL from a
terminal. Querydown is a language for writing relational database queries that compile to SQL, for
PostgreSQL and DuckDB.

```sh
cargo install querydown-cli
```

```sh
# Print the query that describes a PostgreSQL database, and run it to make a schema file
querydown introspect --dialect postgres > introspect.sql
psql --no-psqlrc --tuples-only --no-align --file introspect.sql mydb > schema.json

# Compile a query
querydown compile --schema schema.json '#issues status:open $title $author.username'
```

The command doesn't connect to a database: it prints SQL, which you run with your database's own
tools. See the [CLI documentation](https://querydown.org/docs/setup/cli), and the
[language](https://querydown.org/docs/language).

## License

MIT
