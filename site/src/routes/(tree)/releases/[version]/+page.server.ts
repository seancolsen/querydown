import { error } from '@sveltejs/kit';

import { RELEASES_PATH } from '#lib/nav.ts';
import { getPage, releases } from '#lib/server/content.ts';

import type { EntryGenerator, PageServerLoad } from './$types';

export const entries: EntryGenerator = () => releases().map((version) => ({ version }));

export const load: PageServerLoad = async ({ params }) => {
  const page = await getPage(`${RELEASES_PATH}/${params.version}`);
  if (!page) error(404, 'Not found');
  const { sections: _, ...rest } = page;
  return rest;
};
