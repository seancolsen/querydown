<p align="center">
  <img src="./branding/logo-optimized.svg" width="120" alt="Querydown logo">
</p>

<h1 align="center">Querydown</h1>

<p align="center">
  A domain-specific language for expressively writing relational database queries that compile to SQL.
</p>

<p align="center">
  <a href="https://querydown.org">Website</a> ·
  <a href="https://querydown.org/playground">Playground</a> ·
  <a href="./docs/language.md">Language</a> ·
  <a href="./docs/cheat-sheet.md">Cheat sheet</a> ·
  <a href="./docs/setup/installation.md">Installation</a>
</p>

---

Querydown is designed primarily as a library to be embedded into applications where power users need sophisticated searching and reporting tools. (Picture GitHub's issue search, only more powerful and general-purpose.) The code is succinct and safe for end-users to write, making it akin to **"markdown for SQL"**.

> Each project, with how many issues it has and how many of them are open

```qd
#projects
$name
$#issues -> total
$#issues{status:open} -> open
```

That compiles to this SQL (for PostgreSQL, here):

```sql output
WITH
  "cte0" AS (
    SELECT
      "issues"."project" AS "pk",
      count(*) AS "v1"
    FROM "issues"
    GROUP BY "issues"."project"
  ),
  "cte1" AS (
    SELECT
      "issues"."project" AS "pk",
      count(*) AS "v1"
    FROM "issues"
    WHERE
      COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE)
    GROUP BY "issues"."project"
  )
SELECT
  "projects"."name",
  COALESCE("cte0"."v1", 0) AS "total",
  COALESCE("cte1"."v1", 0) AS "open"
FROM "projects"
LEFT JOIN "cte0" ON
  "projects"."id" = "cte0"."pk"
LEFT JOIN "cte1" ON
  "projects"."id" = "cte1"."pk";
```

A few more, to give you the flavor. They're all written against a fictitious [issue-tracker schema](./docs/sample-schema.md), and you can try them in the [playground](https://querydown.org/playground).

> Open issues that are past their due date

```qd
#issues status:open due_date:<@now $title $due_date
```

> Each issue, with the username and the team of its author

```qd
#issues $title $author.username $author.team.name
```

> Unassigned issues that mention "crash" and are labeled "bug"

```qd
#issues crash ++#labels{name:bug} --#assignments $id $title
```

> The number of issues with each status, most common first

```qd
#issues $status \g $%count \sd
```

## How it works

1. Querydown has a compiler, written in [Rust](https://www.rust-lang.org/).
1. You pass the compiler:
    - A JSON representation of your database schema, which the compiler can [generate from your database](./docs/setup/introspection.md)
    - Your Querydown code
    - Your desired SQL dialect
1. The compiler gives you:
    - The SQL to run against your database
    - Any application-specific metadata defined within the query, as [annotations](./docs/language.md#annotations)

The compiler doesn't connect to your database or run anything. What you do with the SQL is up to you.

## What's here

| | |
| -- | -- |
| **Ways to run the compiler** | A [Rust crate](./docs/setup/rust.md), a [JavaScript package](./docs/setup/javascript.md) (WebAssembly), and a [command line tool](./docs/setup/cli.md) |
| **SQL dialects** | PostgreSQL and DuckDB |
| **Editor support** | Syntax highlighting and autocompletion for [CodeMirror 6](./docs/setup/codemirror.md) |
| **Language features** | Intuitive joins, math, functions, aggregations, custom display columns, complex comparisons, window functions, unions, pipelines, annotations, comments, variables, and user-defined functions. See the [language docs](./docs/language.md). |

## Status

Querydown is a nascent project. It works, and it has one application in production, but it's young:

- Nothing is published to crates.io or npm yet. You [install it from this repository](./docs/setup/installation.md), pinned to a commit.
- There are no versioned releases yet, and the language can still change in ways that break existing queries.
- Lots of [functions](./docs/functions.md) are still missing.
- There's no language server, and no bindings beyond Rust and JavaScript.

## Limitations

Querydown doesn't attempt to support all of SQL. Some SQL queries will never be possible to write with Querydown. That's okay! Querydown tries to make it very _much easier_ to generate the _most common_ kinds of queries.

- It only produces `SELECT` queries.
- The compiler requires knowledge of the database schema.
- Joinable tables are specified within the schema definition you pass the compiler. You can't join arbitrary tables within Querydown code.
- Results never have more rows than the query's base table. Data from related tables is always aggregated.
- Graph traversal and data series generation that you'd do in SQL with recursive CTEs is impossible.
- It has no module system.

Querydown is _not_ a language for application development (use SQL), nor a robust tool for building data pipelines (consider [dbt](https://www.getdbt.com/)).

## Documentation

The documentation is published at [querydown.org](https://querydown.org), and its source is the Markdown in [`docs/`](./docs):

- **Setup**: [installation](./docs/setup/installation.md), the [CLI](./docs/setup/cli.md), [Rust](./docs/setup/rust.md), [JavaScript](./docs/setup/javascript.md), [introspection](./docs/setup/introspection.md), and [CodeMirror](./docs/setup/codemirror.md)
- **[Language](./docs/language.md)**: every feature, by example
- **[Functions](./docs/functions.md)**: the built-in functions
- **[Errors](./docs/errors.md)**: what the compiler reports, and how to read it
- **[Cheat sheet](./docs/cheat-sheet.md)**: all the syntax on one page
- **[Sample schema](./docs/sample-schema.md)**: the issue tracker that the examples query

## Repository layout

| Directory | Contents |
| -- | -- |
| [`parser/`](./parser) | The parser, and the tokenizer behind syntax highlighting |
| [`compiler/`](./compiler) | The compiler (the `querydown` crate), its SQL dialects, and its tests |
| [`cli/`](./cli) | The `querydown` command line tool |
| [`bindings/js/`](./bindings/js) | The JavaScript bindings, built to WebAssembly |
| [`bindings/codemirror/`](./bindings/codemirror) | The CodeMirror 6 language package |
| [`docs/`](./docs) | The documentation, as Markdown |
| [`site/`](./site) | The website that publishes the docs, and the playground |

To work on Querydown itself, see [DEVELOPMENT.md](./DEVELOPMENT.md).

## Who's behind this

I'm [Sean Colsen](https://github.com/seancolsen/). I had the idea for Querydown while writing a lot of SQL by hand for business intelligence projects, and built an early version in 2023, partly as a means to learn Rust. In 2026, with the help of Claude, I fleshed it out to serve as the user-facing query language of [RadioCrate](https://github.com/seancolsen/radiocrate), a personal music collection manager and player for power users.

## License

[MIT](./LICENSE)
