# Rust

The `querydown` crate is the compiler itself. Use it to compile queries within a Rust application, typically one that then runs the SQL against its own database connection.

If you haven't yet, [add the crate to your project](./installation.md#add-the-rust-crate).

## Compile a query

1. Get your database's schema as JSON. To begin with, [generate it with the CLI](./cli.md#generate-your-schema-file) and save it as `schema.json`. Later, your application can [generate it itself](#generate-the-schema-yourself).

1. Create a `Compiler` from the schema and some [options](#options):

    ```rust
    use querydown::{Compiler, IdentifierResolution, Options, Postgres};

    let schema_json = std::fs::read_to_string("schema.json").expect("could not read schema.json");
    let options = Options {
        dialect: Box::new(Postgres()),
        identifier_resolution: IdentifierResolution::Flexible,
        limit: None,
        offset: None,
    };
    let compiler = Compiler::new(&schema_json, options)?;
    ```

    Creating a compiler parses the schema, so keep the compiler and reuse it for each query.

1. Compile a query:

    ```rust
    let query = "#issues status:open $title @{width:300} $author.username";
    let result = compiler.compile(query.to_string())?;
    ```

1. Run `result.sql` against your database, with whichever database library you use. Querydown doesn't connect to a database itself.

    ```rust
    println!("{}", result.sql);
    ```

## Read the annotations

A query can carry [annotations](../language.md#annotations), which the compiler doesn't interpret. It hands them to you along with the SQL, for your application to make use of.

The result of compiling is a `CompileResult`:

| Field | Type | Meaning |
| -- | -- | -- |
| `sql` | `String` | The SQL to run |
| `column_annotations` | `Vec<Option<AnnotationValue>>` | One entry for each column of the results, in order. `None` for a column without an annotation. |
| `query_annotation` | `Option<AnnotationValue>` | The [query's own annotation](../language.md#query-level-annotations), if it has one |

`AnnotationValue` is a JSON-like value. It implements `serde::Serialize`, as `CompileResult` does, so you can turn either into JSON:

```rust
println!("{}", serde_json::to_string_pretty(&result.column_annotations).unwrap());
```

```json
[
  {
    "width": 300
  },
  null
]
```

## Options

`Options` has these fields, all of which you must set:

| Field | Type | Meaning |
| -- | -- | -- |
| `dialect` | `Box<dyn Dialect>` | The SQL dialect to produce: `Box::new(Postgres())` or `Box::new(DuckDB())` |
| `identifier_resolution` | `IdentifierResolution` | `Flexible` to resolve table and column names [flexibly](../language.md#flexible-identifiers), or `Strict` to require exact names |
| `limit` | `Option<u64>` | The most rows the query may return. See [limit and offset](../language.md#limit-and-offset). |
| `offset` | `Option<u64>` | The number of rows to skip before returning any |

The options belong to the compiler, not to each query. To compile the same query for another page of results, create another compiler with a different `offset`.

## Handle errors

Everything that can fail returns `Result<_, querydown::Error>`. An error says what kind of problem it is, where in the query it is, and often how to fix it.

```rust
match compiler.compile(input.to_string()) {
    Ok(result) => run(&result.sql),
    // Prints a diagnostic that quotes the query and underlines the problem.
    Err(error) => eprintln!("{}", error.render(input)),
}
```

The [errors](../errors.md#rust) page describes the error type.

## Generate the schema yourself

Rather than keeping a schema file up to date, your application can ask the database to describe itself whenever it starts (or whenever the structure of the database changes).

1. Get the introspection query for your dialect. It's static, so you can do this once:

    ```rust
    use querydown::{Dialect, Postgres};

    let sql = Postgres().introspection_sql();
    ```

1. Run that query against your database. It returns one row with one column, which holds the schema JSON as text.

1. Pass that text to `Compiler::new`.

The [introspection](./introspection.md) page explains what the query finds, and what the JSON looks like.

## Supply definitions for your users

Querydown's [definitions](../language.md#variables) let you extend the language for your application: constants, functions, [computed columns](../language.md#computed-columns), [custom comparisons](../language.md#custom-comparisons), and a table's [default text search](../language.md#configuring-the-default-text-search). To give your users definitions without making them write them, put the definitions before each query that you compile.

```rust
let prelude = "#issues.comment:@x = ++#comments{body:@x}";
let result = compiler.compile(format!("{prelude}\n{query}"))?;
```

The locations in an error then count the prelude's lines too. If that matters to you, compile the query [in sections](#compile-a-query-in-sections) instead, which keeps the definitions apart.

## Compile a query in sections

An application with a query builder might hold a query as several inputs instead of as one piece of text: a base table chosen from a menu, and separate fields for the conditions, the sorting, and the result columns. You can parse each of those on its own and assemble them into a query.

```rust
use querydown::{ast::Query, parse_conditions, parse_definitions, parse_display, parse_sorting};

let query = Query::from_parts(
    "issues".to_string(),
    parse_definitions("@recent = 1w")?,
    parse_conditions("status:open created_at:<@recent")?,
    parse_sorting(r"\\created_at \d")?,
    parse_display("$title $author.username")?,
);
let result = compiler.compile_query(query)?;
```

Each section has a parser of its own, so the syntax of one can't leak into another: a `$` typed into the sorting field is an error, not a result column. An error from a section [says which section it's in](../errors.md#rust), and locates the problem within that section's text.

A query assembled this way has one stage and no [query annotation](../language.md#query-level-annotations).

## Highlight and complete queries

The crate also has what an editor needs, built on the same parser as the compiler:

- `querydown::highlight::tokenize(source, Grammar::Query)` classifies the source into tokens (a column, a string, a search term, and so on) for syntax highlighting. It never fails, even on source that doesn't parse.
- `compiler.complete(&CompletionRequest::query(source, cursor))` lists what could be typed at a position: the tables, the columns of the table in scope, functions, and so on.

```rust
use querydown::completion::CompletionRequest;

let source = "#issues $author.us";
let completions = compiler.complete(&CompletionRequest::query(source, source.len()));
for item in &completions.items {
    println!("{} ({})", item.label, item.kind.as_str()); // "username (column)", …
}
```

Offsets are in bytes, as usual for Rust strings. If your editor is CodeMirror, the [CodeMirror package](./codemirror.md) has already done this work for you.

## What's missing

The crate has no documentation on docs.rs yet, because it isn't published. Until it is, the doc comments in [the source](../../compiler/src/lib.rs) are the reference for anything this page doesn't cover.
