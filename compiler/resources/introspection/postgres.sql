-- Querydown schema introspection query for PostgreSQL.
--
-- Running this against a live database yields a single row with a single column containing the
-- schema JSON that the Querydown compiler consumes (see `PrimitiveSchema`). A consumer is expected
-- to execute this query, read the lone value, and feed it back into the compiler.
--
-- This query is intentionally static: it only needs to change when the structure of the schema JSON
-- changes. Notes on what it captures:
--
--   * Every schema is introspected except the system's own (`pg_catalog`, `information_schema`, and
--     those whose names begin with `pg_`). A table in the current schema (the first schema on the
--     search path that exists) has no `schema` key, and is written `#table` in Querydown. Every other
--     table has a `schema` key, a list holding the name of its schema, and is written
--     `#/schema/table`. A link's ends are marked the same way. The JSON of a database with only one
--     schema is therefore the same as it would be without schemas.
--   * Tables, partitioned tables, views, materialized views, and foreign tables are all included.
--   * Column types are emitted as Postgres's internal type names (e.g. `int4`, `timestamptz`), with
--     array types rendered as `<element>[]`. The compiler classifies these via `ValueType::parse`.
--   * Links are derived from single-column foreign keys (the schema model only supports
--     single-column links), including those between tables in different schemas. A link is marked
--     `unique` when its referencing column is covered by a single-column unique index or constraint
--     (including a primary key), which is what tells the compiler the reverse relationship is to-one
--     rather than to-many.

WITH relations AS (
  SELECT
    c.oid,
    c.relname,
    n.nspname,
    n.nspname IS NOT DISTINCT FROM current_schema() AS is_current
  FROM pg_class c
  JOIN pg_namespace n ON n.oid = c.relnamespace
  WHERE c.relkind IN ('r', 'p', 'v', 'm', 'f')
    AND n.nspname NOT IN ('pg_catalog', 'information_schema')
    AND n.nspname NOT LIKE 'pg\_%'
)
SELECT json_build_object(
  'tables', COALESCE((
    SELECT json_agg(
      CASE
        WHEN t.is_current THEN json_build_object('name', t.relname, 'columns', t.columns)
        ELSE json_build_object(
          'name', t.relname,
          'schema', json_build_array(t.nspname),
          'columns', t.columns
        )
      END
      ORDER BY NOT t.is_current, t.nspname, t.relname
    )
    FROM (
      SELECT
        r.relname,
        r.nspname,
        r.is_current,
        json_agg(
          json_build_object(
            'name', a.attname,
            'type', CASE
              WHEN elem.typname IS NOT NULL THEN elem.typname || '[]'
              ELSE atype.typname
            END
          ) ORDER BY a.attnum
        ) AS columns
      FROM relations r
      JOIN pg_attribute a ON a.attrelid = r.oid
      JOIN pg_type atype ON atype.oid = a.atttypid
      LEFT JOIN pg_type elem
        ON atype.typcategory = 'A' AND elem.oid = atype.typelem
      WHERE a.attnum > 0
        AND NOT a.attisdropped
      GROUP BY r.oid, r.relname, r.nspname, r.is_current
    ) t
  ), '[]'::json),
  'links', COALESCE((
    SELECT json_agg(
      json_build_object(
        'from', CASE
          WHEN b.is_current THEN json_build_object('table', b.relname, 'column', ba.attname)
          ELSE json_build_object(
            'table', b.relname,
            'schema', json_build_array(b.nspname),
            'column', ba.attname
          )
        END,
        'to', CASE
          WHEN t.is_current THEN json_build_object('table', t.relname, 'column', ta.attname)
          ELSE json_build_object(
            'table', t.relname,
            'schema', json_build_array(t.nspname),
            'column', ta.attname
          )
        END,
        'unique', EXISTS (
          SELECT 1 FROM pg_index i
          WHERE i.indrelid = con.conrelid
            AND i.indisunique
            AND i.indnkeyatts = 1
            AND i.indkey[0] = con.conkey[1]
        )
      ) ORDER BY NOT b.is_current, b.nspname, b.relname, ba.attname
    )
    FROM pg_constraint con
    JOIN relations b ON b.oid = con.conrelid
    JOIN relations t ON t.oid = con.confrelid
    JOIN pg_attribute ba ON ba.attrelid = con.conrelid AND ba.attnum = con.conkey[1]
    JOIN pg_attribute ta ON ta.attrelid = con.confrelid AND ta.attnum = con.confkey[1]
    WHERE con.contype = 'f'
      AND cardinality(con.conkey) = 1
  ), '[]'::json)
) AS schema;
