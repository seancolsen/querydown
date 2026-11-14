/** Facts about the site and the project that several pages need. */

export const SITE_NAME = 'Querydown';
export const SITE_URL = 'https://querydown.org';
export const SITE_TAGLINE = 'A query language that compiles to SQL';

export const GITHUB_REPO = 'seancolsen/querydown';
export const GITHUB_URL = `https://github.com/${GITHUB_REPO}`;
/** Where a file of the repository can be read on GitHub. */
export const githubFileUrl = (path: string) => `${GITHUB_URL}/blob/main/${path}`;
