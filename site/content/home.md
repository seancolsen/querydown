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

Querydown's [examples](../../docs/language.md) use a fictitious [issue-tracker schema](../../docs/sample-schema.md).

<!-- /slideshow -->

Querydown is designed primarily as a library to be embedded into applications where power users need sophisticated searching and reporting tools. _(Picture GitHub's [issue search](https://docs.github.com/en/search-github/searching-on-github/searching-issues-and-pull-requests), only more powerful.)_ It can be configured to work with any database schema. The code is succinct and safe for end-users to write, making it akin to **"markdown for SQL"**.

<div class="platforms">
<div>

Client bindings for

- [Rust](../../docs/setup/rust.md)
- [JavaScript](../../docs/setup/javascript.md)
- [CLI](../../docs/setup/cli.md)

</div>
<div>

Supported Database dialects

- PostgreSQL
- DuckDB

</div>
</div>

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

<!-- features -->

## Language features at a glance

- [Text search](../../docs/language.md#default-text-search)

  > This query shows issues that mention "crash", but not "duplicate".

  ```qd
  #issues crash !duplicate
  ```

- [Filters](../../docs/language.md#comparisons-and-conditions)

  > This query shows open issues created in the past two weeks that have no due date.

  ```qd
  #issues
  status:open
  created_at:<2w
  due_date:@null
  ```

- [Intuitive joins](../../docs/language.md#referencing-single-related-records)

  > This query shows issues written by members of the Backend team, with each one's author and project.

  ```qd
  #issues
  author.team.name:Backend
  $title
  $author.username
  $project.name
  ```

- [Aggregations](../../docs/language.md#referencing-multiple-related-records)

  > This query shows each project, with its number of issues and the date of its newest one.

  ```qd
  #projects
  $name
  $#issues
  $#issues.created_at%max
  ```

- [Has some, has none](../../docs/language.md#has-some-and-has-none-conditions)

  > This query shows active projects that have open issues, but none created in the past month.

  ```qd
  #projects
  is_active
  ++#issues{status:open}
  --#issues{created_at:<1m}
  ```

- [Complex comparisons](../../docs/language.md#comparison-expansion)

  > This query shows open or reopened issues whose title or description mentions "crash" or "freeze".

  ```qd
  #issues
  status:[open reopened]
  [title description]:[crash freeze]
  ```

- [Custom display columns](../../docs/language.md#result-columns)

  > This query shows each user's name and contact details, most recently seen first.

  ```qd
  #users
  $username -> name
  $^f"{username} <{email}>" -> contact
  $last_seen \sd
  ```

- [Grouping](../../docs/language.md#grouping-and-aggregation)

  > This query shows the number of issues with each status, and when the newest of them was created.

  ```qd
  #issues
  $status \g
  $%count -> issues
  $created_at%max -> newest
  ```

- [Math](../../docs/language.md#arithmetic)

  > This query shows each issue, with the number of days between its creation and its due date.

  ```qd
  #issues
  $title
  $(due_date - created_at)|days -> days_allowed
  ```

- [Functions](../../docs/functions.md)

  > This query shows each issue, with its author's email address in lowercase, and the number of days until it's due (or zero, once it's overdue).

  ```qd
  #issues
  $title
  $author.email|lowercase
  $due_date|countdown|days|max(0) -> days_left
  ```

- [Case expressions](../../docs/language.md#case-expressions)

  > This query shows each issue, categorized as "overdue", "due soon", or "due later".

  ```qd
  #issues
  $title
  $ ?
    due_date|countdown|days:<0  ~ "overdue"
    due_date|countdown|days:<30 ~ "due soon"
    ~~ "due later"
  ```

- [Window functions](../../docs/language.md#window-functions)

  > This query shows the newest issue in each project.

  ```qd
  #issues
  %%(project\p created_at\sd)%row_number:1
  $project.name
  $title
  $created_at
  ```

- [Unions](../../docs/language.md#union-of-multiple-queries)

  > This query shows the issues that duplicate issue 1234, and the issues that it blocks.

  ```qd
  #issues duplicate_of:1234
  $id
  $title
  $"Duplicate"
  +++
  #blocks blocker:1234
  $blocking.id
  $blocking.title
  $"Blocked" -> relationship
  ```

- [Pipelines](../../docs/language.md#pipeline-of-multiple-queries)

  > This query shows, for each project, the number of months in which at least 10 issues were created.

  ```qd
  #issues
  $project \g
  $created_at|year_month \g
  $%count -> issue_count
  ~~~
  issue_count:>=10
  $project \g
  $%count
  ```

- [Variables](../../docs/language.md#variables)

  > This query shows issues created in the past two weeks by users who have been seen in that time.

  ```qd
  @recent = 2w
  #users.is_active = last_seen:<@recent
  #issues
  created_at:<@recent
  author.is_active
  ```

- [User-defined functions](../../docs/language.md#user-defined-functions)

  > Given a fiscal year that begins on February 1st, this query shows issues opened in fiscal year 2020 and due in fiscal year 2021.

  ```qd
  @@fiscal_year = @date => (@date - 1m)|year
  #issues
  created_at|fiscal_year:2020
  due_date|fiscal_year:2021
  ```

- [Annotations](../../docs/language.md#annotations)

  > This query shows issues, along with display settings for the application that runs the query to use.

  ```qd
  #issues
  $title      @{width:300}
  $created_at @{format:relative}
  $#comments  @{chart:bar}
  ```

- [Comments](../../docs/language.md#query-anatomy)

  > This query shows open bugs that nobody is working on.

  ```qd
  // Bugs that need an owner
  #issues
  status:open
  ++#labels{name:bug}  // labeled "bug"
  --#assignments       /* and unassigned */
  ```

<!-- /features -->

## Limitations

Querydown doesn't attempt to support all of SQL. Some SQL queries will never be possible to write with Querydown. That's okay! Querydown tries to make it very _much easier_ to generate the _most common_ kinds of queries.

Specifically:

- It only produces `SELECT` queries.
- The compiler requires knowledge of the database schema.
- Joinable tables are specified within the schema definition you pass the compiler. You can't join arbitrary tables within Querydown code, or join on anything but equality.
- A result can't have more rows than its base table, since [related data is always aggregated](../../docs/language.md#mandatory-aggregation).
- It implements some [functions](../../docs/functions.md), but many that you might want are still missing.
- Queries that dynamically traverse graphs and hierarchies (which require recursive CTEs) are not yet possible.
- Data series generation is not yet possible. For example, you can't count the issues opened each month with a row for each month in which none were.
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

<div class="comparisons">

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

</div>

### Is this AI slop?

Maybe some of it. 🤷🏻 But not entirely! I started this project before the AI boom, and I laid the foundations by hand to build a working Querydown compiler the old fashioned way. Then I later leveraged Claude Code heavily to flesh it out. I've been thoughtful of how I guide AI in developing this project, and it has a lot of tests!
