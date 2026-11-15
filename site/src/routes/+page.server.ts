import { error } from '@sveltejs/kit';

import { getPage } from '#lib/server/content.ts';

import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async () => {
  const page = await getPage('/');
  if (!page) error(404, 'Not found');
  return { segments: page.segments, styles: page.styles, description: page.description };
};
