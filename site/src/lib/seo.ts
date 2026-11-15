/**
 * What search engines and link previews are told about a page. `components/PageMeta.svelte` puts
 * it in the page's `<head>`.
 */

import { breadcrumbs, type NavNode } from './nav';
import { SITE_NAME, SITE_URL } from './site';

/** The address that search engines should know a page by, whichever URL they found it at. */
export const canonicalUrl = (path: string) => `${SITE_URL}${path}`;

/**
 * The `<title>` of a page in the navigation tree: its own title, then the groups that it's in, then
 * the site, as in `CLI · Setup · Docs · Querydown`. A title like "CLI" or "0.0.1" says little by
 * itself in a list of search results.
 */
export function titleInTree(nav: NavNode[], page: { title: string; url: string }): string {
  return [page.title, ...breadcrumbs(nav, page.url).reverse(), SITE_NAME].join(' · ');
}

/** About as much of a description as a search engine shows before cutting it short. */
const DESCRIPTION_LENGTH = 160;
/** A description shorter than this says too little to stand by itself. */
const DESCRIPTION_MIN_LENGTH = 80;

/**
 * Shortens a description to the whole sentences that a search engine has room to show, so that it
 * ends at a full stop and not wherever the search engine cuts it. A sentence that doesn't fit is
 * only kept if what comes before it is too short to say much (as the first sentence always is).
 */
export function summarize(description: string): string {
  let summary = '';
  for (const sentence of description.split(/(?<=[.!?])\s+(?=\p{Lu})/u)) {
    const longer = summary ? `${summary} ${sentence}` : sentence;
    if (longer.length > DESCRIPTION_LENGTH && summary.length >= DESCRIPTION_MIN_LENGTH) break;
    summary = longer;
  }
  return summary;
}
