-- Querydown schema introspection query for PostgreSQL.
--
-- Running this against a live database yields a single row with a single column containing the
-- schema JSON that the Querydown compiler consumes (see `PrimitiveSchema`). A consumer is expected
-- to execute this query, read the lone value, and feed it back into the compiler.
--
-- This query is intentionally static: it only needs to change when the structure of the schema JSON
-- changes. Notes on what it captures:
--
--   * Only the `public` schema is introspected. Querydown's schema JSON is a flat namespace of table
--     names, so introspecting multiple Postgres schemas could produce colliding names.
--   * Tables, partitioned tables, views, materialized views, and foreign tables are all included.
--   * Column types are emitted as Postgres's internal type names (e.g. `int4`, `timestamptz`), with
--     array types rendered as `<element>[]`. The compiler classifies these via `ValueType::parse`.
--   * Links are derived from single-column foreign keys (the schema model only supports
--     single-column links). A link is marked `unique` when its referencing column is covered by a
--     single-column unique index or constraint (including a primary key), which is what tells the
--     compiler the reverse relationship is to-one rather than to-many.

SELECT json_build_object(
  'tables', COALESCE((
    SELECT json_agg(
      json_build_object('name', t.table_name, 'columns', t.columns)
      ORDER BY t.table_name
    )
    FROM (
      SELECT
        c.relname AS table_name,
        json_agg(
          json_build_object(
            'name', a.attname,
            'type', CASE
              WHEN elem.typname IS NOT NULL THEN elem.typname || '[]'
              ELSE atype.typname
            END
          ) ORDER BY a.attnum
        ) AS columns
      FROM pg_class c
      JOIN pg_namespace n ON n.oid = c.relnamespace
      JOIN pg_attribute a ON a.attrelid = c.oid
      JOIN pg_type atype ON atype.oid = a.atttypid
      LEFT JOIN pg_type elem
        ON atype.typcategory = 'A' AND elem.oid = atype.typelem
      WHERE n.nspname = 'public'
        AND c.relkind IN ('r', 'p', 'v', 'm', 'f')
        AND a.attnum > 0
        AND NOT a.attisdropped
      GROUP BY c.relname
    ) t
  ), '[]'::json),
  'links', COALESCE((
    SELECT json_agg(
      json_build_object(
        'from', json_build_object('table', bc.relname, 'column', ba.attname),
        'to', json_build_object('table', tc.relname, 'column', ta.attname),
        'unique', EXISTS (
          SELECT 1 FROM pg_index i
          WHERE i.indrelid = con.conrelid
            AND i.indisunique
            AND i.indnkeyatts = 1
            AND i.indkey[0] = con.conkey[1]
        )
      ) ORDER BY bc.relname, ba.attname
    )
    FROM pg_constraint con
    JOIN pg_class bc ON bc.oid = con.conrelid
    JOIN pg_namespace bn ON bn.oid = bc.relnamespace
    JOIN pg_class tc ON tc.oid = con.confrelid
    JOIN pg_namespace tn ON tn.oid = tc.relnamespace
    JOIN pg_attribute ba ON ba.attrelid = con.conrelid AND ba.attnum = con.conkey[1]
    JOIN pg_attribute ta ON ta.attrelid = con.confrelid AND ta.attnum = con.confkey[1]
    WHERE con.contype = 'f'
      AND cardinality(con.conkey) = 1
      AND bn.nspname = 'public'
      AND tn.nspname = 'public'
  ), '[]'::json)
) AS schema;
