# Introspection

Before the compiler can compile a query, it has to know your database's tables, their columns, and how the tables are linked. That's how `author.username` becomes a join, and how `#comments` knows which comments belong to an issue. You give the compiler this knowledge as a JSON document: the schema.

Introspection is how you get that document without writing it by hand. Querydown has a SQL query for each dialect that makes the database describe itself.

## Generate the schema

1. Get the introspection query for your database, in whichever way suits you:

    | From | How |
    | -- | -- |
    | The [CLI](./cli.md#generate-your-schema-file) | `querydown introspect --dialect postgres` |
    | [Rust](./rust.md#generate-the-schema-yourself) | `Postgres().introspection_sql()` |
    | [JavaScript](./javascript.md#introspection_sql) | `introspection_sql("postgres")` |

    The dialect is `postgres` or `duckdb`. The query is the same every time, so you can also save it and keep it with your code.

1. Run the query against your database. It returns a single row with a single column, which holds the schema as JSON text.

1. Give that text to the compiler: as the `--schema` file of the CLI, or as the `schema_json` argument in Rust and JavaScript.

1. Generate the schema again whenever the structure of your database changes. An application can simply do so each time it starts.

## What introspection finds

The query reads the database's catalog. It finds:

- **Tables and their columns**, with the type of each column, in every schema but the database's own system schemas. In PostgreSQL, that includes views, materialized views, partitioned tables, and foreign tables. In DuckDB, it includes the schemas of every attached database.
- **Links between tables**, from foreign key constraints, including those between tables in different schemas.
- **Whether each link is unique**, from unique constraints and primary keys (and, in PostgreSQL, unique indexes) on the foreign key column. This is what tells the compiler that a record has only [one related record](../language.md#related-tables-with-single-vs-multiple-records) rather than many.

It leaves some things out, which you may need to [add by hand](#adjust-the-schema):

- **Foreign keys of more than one column are skipped.** A link joins one column to one column.
- **Relationships that the database doesn't enforce** aren't found, since there's no constraint to find. That includes any relationships of a view.

## The schema JSON

The schema is an object with two arrays: the tables, and the links between them. Here is a schema with two of the tables of the [sample schema](../sample-schema.md):

```json
{
  "tables": [
    {
      "name": "users",
      "columns": [
        { "name": "id", "type": "integer" },
        { "name": "username", "type": "text" }
      ]
    },
    {
      "name": "issues",
      "columns": [
        { "name": "id", "type": "integer" },
        { "name": "title", "type": "text" },
        { "name": "created_at", "type": "timestamp" },
        { "name": "author", "type": "integer" }
      ]
    }
  ],
  "links": [
    {
      "from": { "table": "issues", "column": "author" },
      "to": { "table": "users", "column": "id" },
      "unique": false
    }
  ]
}
```

### Tables

| Field | Meaning |
| -- | -- |
| `name` | The table's name, exactly as it is in the database |
| `schema` | The [schema](#schemas) the table is in. Absent for a table in the current schema. |
| `columns` | The table's columns, in the order that `$*` should output them |
| `columns[].name` | The column's name, exactly as it is in the database |
| `columns[].type` | The column's type. Optional. |

### Column types

Querydown uses a column's type to decide what a comparison means. For example, the [match operator](../language.md#match-comparisons) searches within text but tests anything else for equality, and a date compared with a duration [measures from now](../language.md#comparing-dates-with-durations). Some functions choose their SQL by type too: [`json_path`](../functions.md#json_path) reads a JSON column without converting it from text first.

The type is the name that the database gives it, and Querydown recognizes these names, ignoring case:

| Kind of value | Type names |
| -- | -- |
| Text | `text` `varchar` `char` `character varying` `bpchar` `string` |
| Number | `integer` `int` `int4` `int8` `bigint` `smallint` `numeric` `decimal` `real` `double precision` `double` `float` `float4` `float8` |
| Date | `date` |
| Timestamp | `timestamp` `timestamp without time zone` `datetime` `timestamp_s` `timestamp_ms` `timestamp_ns` |
| Timestamp with a time zone | `timestamptz` `timestamp with time zone` |
| Duration | `interval` |
| Boolean | `boolean` `bool` |
| JSON | `json` `jsonb` |
| Array | any of the above, followed by `[]` |

A column with any other type, or with no type, still works. Querydown passes its values to the database as they are, and compares them for plain equality. If one of your columns has a type that isn't listed but behaves like one that is (a `citext` column, say), you can change its type in the schema to a name from the table.

### Links

| Field | Meaning |
| -- | -- |
| `from` | The column that refers to another table: the foreign key |
| `to` | The column that it refers to, usually a primary key |
| `from.schema` `to.schema` | The [schema](#schemas) of the column's table. Absent for a table in the current schema. |
| `unique` | Whether each value of the `from` column occurs at most once |

A link works in both directions. With the link above, an issue reaches its one author as `author`, and a user reaches their many issues as `#issues`. When `unique` is true, the second direction leads to one record too.

### Schemas

A table in the current schema has no `schema`. In Querydown, it's written by its name alone, as in `#users`. Any other table has a `schema`: a list of names, outermost first, as in `["billing"]`. In Querydown, it's written with those names first, as in `#/billing/invoices`. The compiler writes the same names into the SQL, as in `"billing"."invoices"`.

```json
{
  "tables": [
    {
      "name": "clients",
      "columns": [
        { "name": "id", "type": "integer" },
        { "name": "name", "type": "text" }
      ]
    },
    {
      "name": "invoices",
      "schema": ["billing"],
      "columns": [
        { "name": "id", "type": "integer" },
        { "name": "client", "type": "integer" }
      ]
    }
  ],
  "links": [
    {
      "from": { "table": "invoices", "schema": ["billing"], "column": "client" },
      "to": { "table": "clients", "column": "id" },
      "unique": false
    }
  ]
}
```

- Introspection makes the database's current schema the current schema here too. That's the first schema on the search path in PostgreSQL (usually `public`), and the current schema of the current database in DuckDB (usually `main`). A database with only one schema has no `schema` in its JSON at all.
- A schema can be inside another. In DuckDB, a table in another schema of the current database has a `schema` of one name, as in `["billing"]`, and a table in another attached database has the database's name and then the schema's, as in `["archive", "main"]`.
- A `schema` can also be written as a string, for a schema of one name: `"schema": "billing"` is the same as `"schema": ["billing"]`.
- The SQL refers to a table in the current schema by its name alone, so the database finds it on its search path. You can leave out the `schema` of tables that are on the search path in some other schema, too, as long as no two tables then have the same name.

## Adjust the schema

The schema is plain JSON, so your application can change it between introspecting the database and creating the compiler. Some reasons to:

- **Add a link that the database doesn't declare**, such as a relationship of a view, or one that the database has no foreign key constraint for. Append it to `links`.
- **Remove what your users shouldn't query.** The compiler only knows the tables and columns in the schema, and it rejects a query that names anything else.
- **Reorder columns**, to change the order in which `$*` outputs them.
- **Correct a column's type**, as described [above](#column-types).

The names in the schema have to match the database's, though, because the compiler writes them into the SQL as they are. To give your users friendlier names, define a view in the database.
