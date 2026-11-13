<p align="center">
  <img src="./branding/logo-optimized.svg" width="120" style="margin: 0 auto;">
</p>

# Querydown

Querydown is a domain-specific language for expressively writing relational databases queries that compile to SQL. The code is succinct and safe for end-users to write, making it akin to **"markdown for SQL"**. The Querydown compiler is written in [Rust](https://www.rust-lang.org/).

## Use case

Querydown is intended to be a general-purpose, schema agnostic library that _applications_ can incorporate to give their users powerful searching and reporting capabilities. While other compile-to-SQL languages like [PRQL](https://prql-lang.org/) and [Malloy](https://github.com/malloydata/malloy) are designed for developers and data scientists, Querydown is designed for less-technical _users_ &mdash; the sort of people who are comfortable writing a formula in a spreadsheet but squeamish with SQL. Writing common sorts of queries in Querydown is _much_ easier than writing them in SQL. However, Querydown can't express everything that SQL can (just like Markdown can't express everything that HTML can).

## Status

Querydown is still in early development and not ready for production use.

## Documentation

**[Documentation](./docs/language.md)**

- [Error handling](./docs/errors.md) &mdash; the error type returned by the Rust API, and how the CLI reports errors


## Design goals

- Expressive and succinct
- Users don't think about joins
- No keywords
- All queries begin with one base table
- Results never have more rows than are present in the base table

## How it works

The querydown processor does the following

1. Takes input
    - Querydown code
    - Database schema (so that it knows about foreign keys)
    - *(optionally) Additional schema hints (like pseudo foreign keys)*
    - *(optionally) Other global settings*
1. Produces output
    - SQL
    - Information about the origin of each column in the results (so that cells may be updated)

## Intentional limitations

- It only produces SELECT queries.
- Requires knowledge of schema
- Joinable tables are specified by configuration. You can't join arbitrary tables.
