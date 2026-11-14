# CLI

The `querydown` command compiles Querydown to SQL from a terminal. It's the quickest way to try the compiler against your own database, and it's handy in scripts. It doesn't connect to a database: it prints SQL, which you run with your database's own tools.

If you haven't yet, [install the command line tool](./installation.md#install-the-command-line-tool).

## Generate your schema file

The compiler needs a description of your database, as a JSON file.

1. Print the query that describes a database, for your dialect (`postgres` or `duckdb`):

    ```sh
    querydown introspect --dialect postgres > introspect.sql
    ```

1. Run that query against your database, and save its result. With PostgreSQL:

    ```sh
    psql --no-psqlrc --tuples-only --no-align --file introspect.sql mydb > schema.json
    ```

    With DuckDB:

    ```sh
    duckdb mydb.duckdb -noheader -list < introspect.sql > schema.json
    ```

1. Run it again whenever the structure of your database changes.

The [introspection](./introspection.md) page explains what's in the file, and how to write or adjust one by hand.

To follow along without a database, download the [sample schema](../sample-schema.md#the-schema-as-files) instead, which the rest of this page uses.

## Compile a query

1. Pass the schema file and a query to `querydown compile`. Quote the query with single quotes, so that your shell leaves its `$` and `#` characters alone:

    ```sh
    querydown compile --schema schema.json '#issues status:open $title $author.username'
    ```

    The command prints the SQL:

    ```sql
    SELECT
      "issues"."title",
      "users"."username"
    FROM "issues"
    LEFT JOIN "users" ON
      "issues"."author" = "users"."id"
    WHERE
      COALESCE(strpos(lower("issues"."status" COLLATE "C"), lower('open' COLLATE "C")) > 0, FALSE);
    ```

1. To compile a longer query, save it in a file and pass it on standard input:

    ```sh
    querydown compile --schema schema.json < query.qd
    ```

1. To run the query, pipe the SQL to your database:

    ```sh
    querydown compile --schema schema.json < query.qd | psql mydb
    ```

## Options

`querydown compile` takes these options:

| Option | Meaning |
| -- | -- |
| `-s`, `--schema <FILE>` | The schema JSON file. Required. |
| `-d`, `--dialect <DIALECT>` | The SQL dialect to produce: `postgres` (the default) or `duckdb`. |
| `-f`, `--format <FORMAT>` | What to print: `sql` (the default) or `json`. |
| `--limit <ROWS>` | The most rows the query may return. See [limit and offset](../language.md#limit-and-offset). |
| `--offset <ROWS>` | The number of rows to skip before returning any. |

`querydown introspect` takes only `--dialect`.

Run `querydown help`, or any command with `--help`, for the same information in your terminal.

## Get the annotations too

A query can carry [annotations](../language.md#annotations): data for the application that runs it, which the compiler passes through. The SQL alone doesn't include them.

1. Ask for JSON with `--format json`:

    ```sh
    querydown compile --schema schema.json --format json '#issues $title @{width:300} $status'
    ```

1. Read the SQL and the annotations from the object that it prints:

    ```json
    {
      "sql": "SELECT\n  \"issues\".\"title\",\n  \"issues\".\"status\"\nFROM \"issues\";",
      "columnAnnotations": [
        {
          "width": 300
        },
        null
      ],
      "queryAnnotation": null
    }
    ```

`columnAnnotations` has one entry for each column of the results, in order, and `queryAnnotation` holds the query's own annotation.

## Handle errors

When a query doesn't compile, `querydown compile` exits with code 1 and describes the problem.

```text
$ querydown compile --schema schema.json '#issues $nope'
compile error: Column `nope` not found within table `issues`.
 --> line 1, column 10
  |
1 | #issues $nope
  |          ^^^^
```

With `--format json`, it prints the error as JSON instead, for a program to read. The [errors](../errors.md#command-line) page describes both forms.
