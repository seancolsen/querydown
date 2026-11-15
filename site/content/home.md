# Querydown

Querydown is a domain-specific language for expressively writing relational database queries that compile to SQL.

<!-- slideshow -->

> Show open issues created in the past two months, along with their author and date of most recent comment

```qd
#issues
status:open
created_at:<2m
$title
$author.username
$#comments.created_at%max
```

> Show unassigned issues that mention "crash" and are labeled "bug"

```qd
#issues
crash
++#labels{name:bug}
--#assignments
```

<!-- /slideshow -->

<small>All of Querydown's [examples](../../docs/language.md) are written against a fictitious [issue-tracker schema](../../docs/sample-schema.md).</small>

Querydown is designed primarily as a library to be embedded into applications where power users need sophisticated searching and reporting tools. _(Picture GitHub's [issue search](https://docs.github.com/en/search-github/searching-on-github/searching-issues-and-pull-requests), only more powerful.)_ It can be configured to work with any database schema. The code is succinct and safe for end-users to write, making it akin to **"markdown for SQL"**.

Current client bindings available:

- [Rust](../../docs/setup/rust.md)
- [JavaScript](../../docs/setup/javascript.md) (via WebAssembly)
- A [command line tool](../../docs/setup/cli.md)

Current database dialects supported:

- PostgreSQL
- DuckDB

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

Querydown is a nascent project. It works, but it's young, and has not been battle-tested:

- Nothing is published to crates.io or npm yet. You [install it from Git](../../docs/setup/installation.md).
- There are [no versioned releases](./releases/0.0.1.md) yet, and the language can still change in ways that break existing queries.
- It has bindings for two languages and supports two database systems.
- It has no language server, and editor support for CodeMirror only.
- It's the work of one person, with one application in production.

## Questions

### Who built this?

I'm [Sean Colsen](https://github.com/seancolsen), an engineer who likes to tinker with data things. This is one of my hobby projects.

### Why did you build this?

I had the idea for Querydown while writing a lot of SQL by hand for business intelligence and data visualization projects in the CRM space. SQL is powerful, but its syntax has always felt verbose to me. I don't like the amount of code I have to write to access joined data. And I don't like that its design requires so many internally consistent identifiers. I envisioned something simpler and more expressive — even if it meant less power. In 2023, I built an early version of Querydown, partly as a means to learn Rust. Then, with the help of Claude, I fleshed it out in 2026 to serve as a user-facing query language in my high performance music manager app, [RadioCrate](https://github.com/seancolsen/radiocrate).

### Why would end users ever want to learn a new query language?

Many certainly would not! I get it. Querydown is aimed at the kind of user who wants more power than a GUI and more convenience than SQL.

### What inspired the language design?

- The search syntax for [GitHub issues](https://docs.github.com/en/search-github/searching-on-github/searching-issues-and-pull-requests).
- The search syntax within [Amarok](https://amarok.kde.org/), a desktop music player.
- Regular expressions (on a more conceptual level, in that the language is designed without keywords)

### How is this different from other projects?

- <details><summary>Malloy</summary>

    **[Malloy](https://www.malloydata.dev/)** also compiles to SQL, follows relationships with dotted paths such as `author.username`, and aggregates the values of a to-many relationship correctly without your writing a join.
    
    But Malloy is a modeling language for data analysts: before a query can follow a relationship, you declare it in a model with `join_one` or `join_many`, and you build reusable dimensions, measures and views on top of it. Malloy reads your tables' columns from the database, but not their foreign keys. It usually connects to your database to run its queries, supports many databases, and its queries, written as clauses such as `group_by:` and `aggregate:`, are much longer than a search. Querydown takes the relationships from the schema you give it, which it can generate from your foreign keys, and keeps queries short enough for the users of your application to type.

    </details>

- <details><summary>PRQL</summary>

    **[PRQL](https://prql-lang.org/)** is also a concise language with a Rust compiler that turns your queries into SQL for you to run.
    
    But PRQL is a general-purpose replacement for SQL, aimed at people who transform and analyze data: each query is a pipeline of steps such as `filter`, `join` and `select`, every join is written out, and it compiles to many SQL dialects. Querydown is a search language for the users of your application, so it knows your schema, follows the relationships between your tables without any joins, and keeps queries short enough to type into a search box.

    </details>

- <details><summary>Logica, Preql, pql</summary>

    **[Logica](https://logica.dev/)**, **[Preql](https://github.com/erezsh/Preql)** and **[pql](https://github.com/runreveal/pql)** also compile to SQL.
    
    But like PRQL, none of them knows your schema's relationships, so you write every join yourself.

    - Logica, which began at Google, is a logic-programming language in the Datalog family, with support for recursion. It's aimed at engineers and data scientists.

    - Preql is a general-purpose language with Python-like syntax that runs your queries against a live database. It's aimed at data engineers and analysts.

    - pql is the closest of the three to Querydown in purpose: a Go library that compiles a pipeline syntax borrowed from Kusto's KQL, which RunReveal embeds in its product so that customers can search their security logs.

    </details>

- <details><summary>PostgREST</summary>

    **[PostgREST](https://postgrest.org/)** also finds the relationships between your tables from their foreign keys, so a query can pull in columns from related records, and filter by them, without a join.
    
    But PostgREST is a web server that turns a PostgreSQL database into a REST API, with writes, function calls, and authentication through Postgres roles. Its queries are URL parameters, such as `/issues?select=title,author:users(username),comments(count)&status=eq.open`, which are usually built by client libraries (like [Supabase](https://supabase.com/)'s) rather than typed by a person. A to-many relationship comes back as a nested JSON array, and its aggregate functions are turned off by default. Querydown doesn't run anything or serve an API: it gives you SQL, for PostgreSQL or DuckDB.

    </details>

- <details><summary>HTSQL, Rabbit, DataKnots.jl</summary>

    **[HTSQL](https://www.htsql.org/)** is also a schema-aware query language that follows the relationships between your tables for you. You can reach a related record's columns without writing a join, and anything pulled from a to-many relationship has to be aggregated.
    
    But HTSQL, last released in 2013, is a Python web service whose queries are URLs: it runs them against your database itself and returns the results as HTML, JSON or CSV. Querydown is only a compiler that you embed in your application, turning short, search-box-style queries into SQL that you run however you like.

    HTSQL's authors later carried its ideas into **[Rabbit](https://arxiv.org/abs/1702.08409)**, a research language described in their paper "Query Combinators", and **[DataKnots.jl](https://github.com/MechanicalRabbit/DataKnots.jl)**, a Julia library. Both navigate relationships the same way, and they let a query return a to-many value as a nested list, without aggregating it. But both query data in memory rather than producing SQL.

    </details>

- <details><summary>SOQL</summary>

    **[SOQL](https://developer.salesforce.com/docs/atlas.en-us.soql_sosl.meta/soql_sosl/sforce_api_calls_soql.htm)**, Salesforce's query language, also only reads data, and follows relationships without joins. SOQL reaches a parent record's fields with dotted paths like `Account.Name`, and a parent's child records with a nested `SELECT`. 
    
    But SOQL queries only the data in Salesforce, through its APIs, and its syntax is SQL's, keywords and all. It also can't aggregate child records within a query of their parents: to count each case's comments, for example, you run a second query. Querydown works with your own database, and a count of related records is one more column, such as `$#comments`.

    </details>

- <details><summary>EdgeQL</summary>

    **[EdgeQL](https://docs.geldata.com/reference/edgeql)**, the language of the Gel database (formerly EdgeDB), also follows links between records with paths instead of joins, and knows whether each path leads to one value or to many.
    
    But Gel is a database server, built on PostgreSQL, that owns your data: you define your schema in Gel's own language and migrate your data into it, rather than pointing EdgeQL at an existing database. EdgeQL is for application developers, writes data as well as reading it, and returns to-many values as nested sets rather than requiring you to aggregate them. The company behind Gel [shut down](https://www.geldata.com/p/sunset) in December 2025. Gel is still open source, but its repository has been quiet since.

    </details>

- <details><summary>LinkedQL</summary>

    **[LinkedQL](https://linked-ql.netlify.app/)** also follows foreign keys for you. LinkedQL's [DeepRefs](https://linked-ql.netlify.app/lang/deeprefs) let you write `author ~> name` in a query of `posts`, and `<~` to go the other way.
    
    But LinkedQL is SQL with extensions, so the rest of each query is ordinary SQL, including writes. It's a JavaScript database client that connects to PostgreSQL or MySQL (or its own embedded engine) and runs your queries, and a reference to many records gives you one row per record unless you aggregate them yourself. Querydown is a language of its own, designed for searching rather than for application code.

    </details>

### Is this AI slop?

Maybe some of it. 🤷🏻 But not entirely! I started this project before the AI boom, and I laid the foundations by hand to build a working Querydown compiler the old fashioned way. Then I later leveraged Claude Code heavily to flesh it out. I've been thoughtful of how I guide AI in developing this project, and it has a lot of tests!
