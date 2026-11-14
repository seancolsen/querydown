import { querydownThemeCss } from '#lib/server/highlight.ts';

import type { RequestHandler } from './$types';

export const prerender = true;

/** The colors of highlighted Querydown, generated from the CodeMirror package's palettes. */
export const GET: RequestHandler = () =>
  new Response(querydownThemeCss(), { headers: { 'content-type': 'text/css; charset=utf-8' } });
