import { json } from '@sveltejs/kit';

import { breadcrumbs, buildNav } from '#lib/nav.ts';
import type { SearchEntry } from '#lib/search.ts';
import { allPages, releasePages } from '#lib/server/content.ts';

import type { RequestHandler } from './$types';

export const prerender = true;

/** The search index: one entry for each section of each page. See `Search.svelte`. */
export const GET: RequestHandler = async () => {
  const nav = buildNav(releasePages());
  const entries: SearchEntry[] = [];
  for (const page of await allPages()) {
    const groups = breadcrumbs(nav, page.url);
    for (const section of page.sections) {
      // A page's first section is headed by its title, which `page` already holds.
      const isTitle = section.parents.length === 0 && section.heading === page.title;
      entries.push({
        id: entries.length,
        url: section.id && !isTitle ? `${page.url}#${section.id}` : page.url,
        page: page.title,
        heading: isTitle ? '' : section.heading,
        // The navigation groups, the page, and then the headings above the section (the first of
        // which is the page's title again).
        trail: [...groups, page.title, ...section.parents.slice(1)],
        text: section.text,
      });
    }
  }
  return json(entries);
};
