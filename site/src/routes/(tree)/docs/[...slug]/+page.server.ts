import { error, redirect } from '@sveltejs/kit';

import { DOCS, firstPage, groupsOf, pagesOf } from '#lib/nav.ts';
import { getPage } from '#lib/server/content.ts';

import type { EntryGenerator, PageServerLoad } from './$types';

const PREFIX = '/docs';
const slugOf = (url: string) => url.slice(PREFIX.length + 1);

/** Every docs page, plus the URL of every group (which redirects to the group's first page). */
export const entries: EntryGenerator = () =>
  [...pagesOf(DOCS).map((page) => page.href), ...groupsOf(DOCS).map((group) => group.path)].map(
    (url) => ({ slug: slugOf(url) }),
  );

export const load: PageServerLoad = async ({ params }) => {
  const url = params.slug ? `${PREFIX}/${params.slug}` : PREFIX;

  const group = groupsOf(DOCS).find((g) => g.path === url);
  const target = group && firstPage(group);
  if (target) redirect(307, target.href);

  const page = await getPage(url);
  if (!page) error(404, 'Not found');
  const { sections: _, ...rest } = page;
  return rest;
};
