/**
 * The playground keeps its state in the URL's fragment, so that a link can open it with a
 * particular query. The fragment is formatted like a query string:
 *
 *     /playground#q=%23issues+%24title&dialect=duckdb&limit=50
 *
 * | Key       | Meaning                                                              |
 * | --------- | -------------------------------------------------------------------- |
 * | `q`       | The Querydown source                                                 |
 * | `dialect` | `postgres` (the default) or `duckdb`                                 |
 * | `limit`   | The compiler's `limit` option                                        |
 * | `offset`  | The compiler's `offset` option                                       |
 * | `schema`  | A customized schema (deflated, then base64url); absent for the sample |
 */

export const DIALECTS = ['postgres', 'duckdb'] as const;
export type Dialect = (typeof DIALECTS)[number];

export interface PlaygroundLink {
  query: string;
  dialect?: Dialect;
  limit?: number | null;
  offset?: number | null;
  /** The encoded schema; see `encodeSchema` in the playground. */
  schema?: string | null;
}

/** The fragment (without its `#`) for a playground link. */
export function playgroundFragment(link: PlaygroundLink): string {
  const params = new URLSearchParams();
  params.set('q', link.query);
  if (link.dialect && link.dialect !== 'postgres') params.set('dialect', link.dialect);
  if (link.limit != null) params.set('limit', String(link.limit));
  if (link.offset != null) params.set('offset', String(link.offset));
  if (link.schema) params.set('schema', link.schema);
  return params.toString();
}

function parseCount(value: string | null): number | null {
  if (value === null || !/^\d+$/.test(value)) return null;
  return Number(value);
}

/** Reads a fragment (with or without its `#`) written by {@link playgroundFragment}. */
export function parsePlaygroundFragment(fragment: string): Partial<PlaygroundLink> {
  const params = new URLSearchParams(fragment.replace(/^#/, ''));
  const dialect = params.get('dialect');
  return {
    query: params.get('q') ?? undefined,
    dialect: (DIALECTS as readonly string[]).includes(dialect ?? '')
      ? (dialect as Dialect)
      : undefined,
    limit: parseCount(params.get('limit')),
    offset: parseCount(params.get('offset')),
    schema: params.get('schema'),
  };
}
