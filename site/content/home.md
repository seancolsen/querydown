# Querydown

Querydown is a domain-specific language for expressively writing relational database queries that compile to SQL.

<!-- slideshow -->

> Open issues that are past their due date

```qd
#issues
status:open due_date:<@now
$title $due_date
```

> Each issue, with the username and the team of its author

```qd
#issues
$title
$author.username
$author.team.name
```

> The number of issues with each status, most common first

```qd
#issues
$status \g
$%count \sd
```

> Each project, with how many issues it has and how many of them are open

```qd
#projects
$name
$#issues -> total
$#issues{status:open} -> open
```

> Unassigned issues that mention "crash" and are labeled "bug"

```qd
#issues
crash ++#labels{name:bug} --#assignments
$id $title
```

> Issues created in the past week, newest first

```qd
#issues
created_at:<1w
\\created_at \d
$title $author.username
```

<!-- /slideshow -->

<small>All of Querydown's [examples](../../docs/language.md) are written against a fictitious [issue-tracker schema](../../docs/sample-schema.md).</small>

It is designed primarily as a library to be embedded into applications where power users need sophisticated searching and reporting tools. (Picture GitHub's issue search, only more powerful and general-purpose.) I'm [Sean Colsen](https://github.com/seancolsen/), and I built Querydown for [RadioCrate](https://github.com/seancolsen/radiocrate), a personal music collection manager and player for power users. But it can be configured to work with any database schema, and is well-suited for business intelligence use cases too. The code is succinct and safe for end-users to write, making it akin to **"markdown for SQL"**.

Current client bindings available:

- [Rust](../../docs/setup/rust.md)
- [JavaScript](../../docs/setup/javascript.md) (via WebAssembly)
- A [command line tool](../../docs/setup/cli.md)

Current database dialects supported:

- PostgreSQL
- DuckDB

## Try it

- Write a query in the [playground](/playground), and see the SQL it compiles to. Nothing to install.
- Read about the [language](../../docs/language.md), or skim the [cheat sheet](../../docs/cheat-sheet.md).
- [Install](../../docs/setup/installation.md) the compiler, and point it at a database of your own.
- Read the source on [GitHub](https://github.com/seancolsen/querydown).

## How it works

1. Querydown has a Rust-based compiler.
1. You pass the compiler:
    - A JSON representation of your database schema, which the compiler can [generate from your database](../../docs/setup/introspection.md)
    - Your Querydown code
    - Your desired SQL dialect
1. The compiler gives you:
    - The SQL to run against your database
    - Any application-specific metadata defined within the query, as [annotations](../../docs/language.md#annotations)

The compiler doesn't connect to your database or run anything. What you do with the SQL is up to you.

## Features

[Intuitive joins](../../docs/language.md#referencing-single-related-records), [math](../../docs/language.md#arithmetic), [functions](../../docs/functions.md), [aggregations](../../docs/language.md#referencing-multiple-related-records), [custom display columns](../../docs/language.md#result-columns), [complex comparisons](../../docs/language.md#comparison-expansion), [window functions](../../docs/language.md#window-functions), [unions](../../docs/language.md#union-of-multiple-queries), [pipelines](../../docs/language.md#pipeline-of-multiple-queries), [annotations](../../docs/language.md#annotations), [comments](../../docs/language.md#query-anatomy), [variables](../../docs/language.md#variables), [user-defined functions](../../docs/language.md#user-defined-functions)

For editors, there's [syntax highlighting and autocompletion for CodeMirror](../../docs/setup/codemirror.md).

## Limitations

Querydown doesn't attempt to support all of SQL. Some SQL queries will never be possible to write with Querydown. That's okay! Querydown tries to make it very _much easier_ to generate the _most common_ kinds of queries.

Specifically:

- It only produces `SELECT` queries.
- The compiler requires knowledge of the database schema.
- Joinable tables are specified within the schema definition you pass the compiler. You can't join arbitrary tables within Querydown code.
- Lots of [functions](../../docs/functions.md) are still missing.
- Graph traversal and data series generation that you'd do in SQL with recursive CTEs is impossible.
- It has no module system.

Querydown is _not_:

- a language for application development. Use SQL instead.
- a robust tool for building data pipelines. Consider [dbt](https://www.getdbt.com/) instead.

## Status

Querydown is a nascent project. It works, and RadioCrate depends on it, but it's young:

- Nothing is published to crates.io or npm yet. You [install it from Git](../../docs/setup/installation.md).
- There are [no versioned releases](./releases/0.0.1.md) yet, and the language can still change in ways that break existing queries.
- It has bindings for two languages and supports two database systems.
- It has no language server, and editor support for CodeMirror only.
- It's the work of one person, with one application in production.

## Questions

### Why did you build this?

I had the idea for Querydown while writing a lot of SQL by hand for business intelligence and data visualization projects in the CRM space. While powerful, SQL's syntax felt verbose. I envisioned something simpler and more expressive — even if it meant less power. In 2023, I built an early version of Querydown, partly as a means to learn Rust. Then, with the help of Claude, I fleshed it out in 2026 to serve as a user-facing query language in my high performance music manager app, [RadioCrate](https://github.com/seancolsen/radiocrate).

### Why would end users ever want to learn a new query language?

Many certainly would not! I get it. __TODO__

### What inspired the language design?

__TODO__

### Why no keywords?

__TODO__

### Why does it look so ugly?

__TODO__

### How is this different from...

- PRQL?
- __TODO__

### Is this AI slop?

Maybe some of it. But not entirely! I started this project before the AI boom, but then leveraged Claude heavily to get it over the finish line.
