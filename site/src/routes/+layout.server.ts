import { releasePages } from '#lib/server/content.ts';
import { buildNav } from '#lib/nav.ts';

import type { LayoutServerLoad } from './$types';

export const load: LayoutServerLoad = () => ({ nav: buildNav(releasePages()) });
