-- Querydown schema introspection query for DuckDB.
--
-- Running this against a live database yields a single row with a single column containing the
-- schema JSON that the Querydown compiler consumes (see `PrimitiveSchema`). A consumer is expected
-- to execute this query, read the lone value, and feed it back into the compiler.
--
-- This query is intentionally static: it only needs to change when the structure of the schema JSON
-- changes. Notes on what it captures:
--
--   * Every schema of every attached database is introspected, except the `system` database and the
--     `information_schema` and `pg_catalog` schemas. A table in the current schema of the current
--     database has no `schema` key, and is written `#table` in Querydown. A table in another schema
--     of the current database has a `schema` key listing that schema's name (written
--     `#/schema/table`), and a table in another database lists the database's name and then the
--     schema's (written `#/database/schema/table`). A link's ends are marked the same way. The JSON
--     of a database with only one schema is therefore the same as it would be without schemas.
--   * Column types are emitted as DuckDB's type names (e.g. `INTEGER`, `TIMESTAMP`, `VARCHAR[]`).
--     The compiler classifies these via `ValueType::parse`.
--   * Links are derived from single-column foreign keys (the schema model only supports
--     single-column links). DuckDB's foreign keys refer to tables in their own schema. A link is
--     marked `unique` when its referencing column is covered by a single-column unique or
--     primary-key constraint, which is what tells the compiler the reverse relationship is to-one
--     rather than to-many.
--
-- `COALESCE(..., '[]')` guards the empty-database case so each key is an empty array rather than
-- null.

WITH
  -- Where each schema is, as Querydown writes it: the names before the table's.
  schemas AS (
    SELECT DISTINCT
      table_catalog,
      table_schema,
      CASE
        WHEN table_catalog = current_database() AND table_schema = current_schema() THEN []
        WHEN table_catalog = current_database() THEN [table_schema]
        ELSE [table_catalog, table_schema]
      END AS path
    FROM information_schema.tables
    WHERE table_catalog <> 'system'
      AND table_schema NOT IN ('information_schema', 'pg_catalog')
  ),
  tables AS (
    SELECT
      c.table_name,
      s.path,
      list(
        struct_pack(name := c.column_name, "type" := c.data_type)
        ORDER BY c.ordinal_position
      ) AS cols
    FROM information_schema.columns c
    JOIN schemas s USING (table_catalog, table_schema)
    GROUP BY c.table_name, s.path
  ),
  links AS (
    SELECT
      s.path,
      fk.table_name,
      fk.constraint_column_names[1] AS column_name,
      fk.referenced_table,
      fk.referenced_column_names[1] AS referenced_column,
      EXISTS (
        SELECT 1 FROM duckdb_constraints() u
        WHERE u.table_oid = fk.table_oid
          AND u.constraint_type IN ('UNIQUE', 'PRIMARY KEY')
          AND len(u.constraint_column_names) = 1
          AND u.constraint_column_names[1] = fk.constraint_column_names[1]
      ) AS is_unique
    FROM duckdb_constraints() fk
    JOIN schemas s ON s.table_catalog = fk.database_name AND s.table_schema = fk.schema_name
    WHERE fk.constraint_type = 'FOREIGN KEY'
      AND len(fk.constraint_column_names) = 1
  )
SELECT json_object(
  'tables', COALESCE((
    SELECT to_json(list(
      CASE
        WHEN len(path) = 0 THEN json_object('name', table_name, 'columns', cols)
        ELSE json_object('name', table_name, 'schema', path, 'columns', cols)
      END
      ORDER BY len(path) > 0, path, table_name
    ))
    FROM tables
  ), '[]'),
  'links', COALESCE((
    SELECT to_json(list(
      json_object(
        'from', CASE
          WHEN len(path) = 0 THEN json_object('table', table_name, 'column', column_name)
          ELSE json_object('table', table_name, 'schema', path, 'column', column_name)
        END,
        'to', CASE
          WHEN len(path) = 0 THEN json_object('table', referenced_table, 'column', referenced_column)
          ELSE json_object(
            'table', referenced_table,
            'schema', path,
            'column', referenced_column
          )
        END,
        'unique', is_unique
      )
      ORDER BY len(path) > 0, path, table_name, column_name
    ))
    FROM links
  ), '[]')
) AS schema;
