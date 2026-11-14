import { allPages, sourceOf } from '#lib/server/content.ts';

import type { RequestHandler } from './$types';

export const prerender = true;

/**
 * The Markdown source of every page, in one file, for AI agents that would rather read the docs in
 * one go. Each page is preceded by a comment giving its URL.
 */
export const GET: RequestHandler = async ({ url }) => {
  const pages = await allPages();
  const body = pages
    .map((page) => `<!-- ${url.origin}${page.url} -->\n\n${sourceOf(page).trim()}\n`)
    .join('\n\n---\n\n');
  return new Response(body, { headers: { 'content-type': 'text/plain; charset=utf-8' } });
};
