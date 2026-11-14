import { error, redirect } from '@sveltejs/kit';

import { releasePages } from '#lib/server/content.ts';

import type { PageServerLoad } from './$types';

/** "Releases" is a group, not a page, so its URL leads to the newest release. */
export const load: PageServerLoad = () => {
  const [latest] = releasePages();
  if (!latest) error(404, 'Not found');
  redirect(307, latest.href);
};
