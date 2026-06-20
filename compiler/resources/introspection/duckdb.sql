-- Querydown schema introspection query for DuckDB.
--
-- Running this against a live database yields a single row with a single column containing the
-- schema JSON that the Querydown compiler consumes (see `PrimitiveSchema`). A consumer is expected
-- to execute this query, read the lone value, and feed it back into the compiler.
--
-- This query is intentionally static: it only needs to change when the structure of the schema JSON
-- changes. Notes on what it captures:
--
--   * Only the `main` schema is introspected. Querydown's schema JSON is a flat namespace of table
--     names, so introspecting multiple DuckDB schemas could produce colliding names.
--   * Column types are emitted as DuckDB's type names (e.g. `INTEGER`, `TIMESTAMP`, `VARCHAR[]`).
--     The compiler classifies these via `ValueType::parse`.
--   * Links are derived from single-column foreign keys (the schema model only supports
--     single-column links). A link is marked `unique` when its referencing column is covered by a
--     single-column unique or primary-key constraint, which is what tells the compiler the reverse
--     relationship is to-one rather than to-many.
--
-- `COALESCE(..., [])` guards the empty-database case so each key is an empty array rather than null.

SELECT json_object(
  'tables', to_json(COALESCE((
    SELECT list(struct_pack(name := table_name, columns := cols) ORDER BY table_name)
    FROM (
      SELECT
        table_name,
        list(
          struct_pack(name := column_name, "type" := data_type)
          ORDER BY ordinal_position
        ) AS cols
      FROM information_schema.columns
      WHERE table_schema = 'main'
      GROUP BY table_name
    )
  ), [])),
  'links', to_json(COALESCE((
    SELECT list(struct_pack(
      "from" := struct_pack("table" := fk.table_name, "column" := fk.constraint_column_names[1]),
      "to" := struct_pack("table" := fk.referenced_table, "column" := fk.referenced_column_names[1]),
      "unique" := EXISTS (
        SELECT 1 FROM duckdb_constraints() u
        WHERE u.table_oid = fk.table_oid
          AND u.constraint_type IN ('UNIQUE', 'PRIMARY KEY')
          AND len(u.constraint_column_names) = 1
          AND u.constraint_column_names[1] = fk.constraint_column_names[1]
      )
    ) ORDER BY fk.table_name, fk.constraint_column_names[1])
    FROM duckdb_constraints() fk
    WHERE fk.constraint_type = 'FOREIGN KEY'
      AND fk.schema_name = 'main'
      AND len(fk.constraint_column_names) = 1
  ), []))
) AS schema;
