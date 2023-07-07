<p align="center">
  <img src="./branding/logo-optimized.svg" width="120" style="margin: 0 auto;">
</p>

# Querydown

Querydown is a modern [domain-specific programming language](https://en.wikipedia.org/wiki/Domain-specific_language) for expressively writing relational databases queries that compile to [SQL](https://en.wikipedia.org/wiki/SQL). The code is succinct and safe for end-users to write, making it like **"markdown for SQL"**. The Querydown compiler is written in [Rust](https://www.rust-lang.org/).

## Use case

Developers write HTML &mdash; users write Markdown.

Developers write SQL &mdash; users write... _Querydown_!

Querydown is intended to be a general-purpose, schema agnostic library that _applications_ can incorporate to give their users powerful searching and reporting capabilities. While other compile-to-SQL languages like [PRQL](https://prql-lang.org/) and [Malloy](https://github.com/malloydata/malloy) are designed for developers and data scientists, Querydown is designed for less-technical _users_ &mdash; the sort of people who are comfortable writing a formula in a spreadsheet but squeamish with SQL. Writing common sorts of queries in Querydown is _much_ easier than writing them in SQL. However, Querydown can't express everything that SQL can (just like Markdown can't express everything that HTML can).

## Status

⚠️ Querydown is currently in **_very_ early development**. ⚠️

- The [language design](./docs/syntax.md) is still in flux, but only changing slowly.
- Parsing is implemented for _most_ of the language, as currently designed.
- Compilation is implemented for some simple queries, but there is still a lot of work to do here!
- PostgreSQL is the only dialect implemented so far.

## Example

> _Given an example [issue-tracker schema](./docs/syntax.md#example-schema)_...<br/>
> - Find **issues**
> - that were created in the past 6 months,
> - and have no assignments,
> - and have at least one label which is named "regression" or named "bug",
> - and have between 10 and 20 comments by users whose team is not named "backend".
> - Show all columns in the issues table.
> - Show the author's username,
> - Show the date of the first comment by anyone &mdash; while also sorting the results on that date (with the most recent values shown first).

_(The bullet points above correspond, line for line, with the querydown code below)_

```text
#issues
created_at:>@6M|ago
--#assignments
++#labels{name:..["Regression" "Bug"]}
10..20:#comments{user.team.name!"Backend"}
$*
$author.username
$#comments.created_at%min \sd
```
## Documentation

**[Documentation](./docs/syntax.md)**


## Design goals

- Expressive and succinct
- Safety from arbitrary joins which may cause unwanted compute if missing the proper `ON` clause
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
