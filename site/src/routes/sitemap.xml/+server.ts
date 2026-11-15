import { PLAYGROUND } from '#lib/nav.ts';
import { canonicalUrl } from '#lib/seo.ts';
import { pageUrls } from '#lib/server/content.ts';

import type { RequestHandler } from './$types';

export const prerender = true;

/**
 * The list of the site's pages for search engines, in the sitemap format (https://sitemaps.org).
 * `robots.txt` says where it is.
 *
 * It lists each page by its canonical URL, and leaves out the URLs that only redirect (`/docs`,
 * `/releases`). `scripts/check-seo.js` checks that it agrees with the pages that were built.
 */
export const GET: RequestHandler = () => {
  const urls = [...pageUrls(), PLAYGROUND.href].map(canonicalUrl);
  const body = [
    '<?xml version="1.0" encoding="UTF-8"?>',
    '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">',
    ...urls.map((url) => `  <url><loc>${url}</loc></url>`),
    '</urlset>',
    '',
  ].join('\n');
  return new Response(body, { headers: { 'content-type': 'application/xml; charset=utf-8' } });
};
