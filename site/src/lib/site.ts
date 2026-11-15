/** Facts about the site and the project that several pages need. */

export const SITE_NAME = 'Querydown';
export const SITE_URL = 'https://querydown.org';
export const SITE_TAGLINE = 'A query language that compiles to SQL';

/**
 * The picture that a link to the site is previewed with, in chat apps and on social networks. It's
 * made by `scripts/make-social-card.js`.
 */
export const SOCIAL_CARD = {
  path: '/social-card.png',
  width: 1200,
  height: 630,
  alt: `The ${SITE_NAME} logo, above the words “${SITE_TAGLINE}”`,
};

export const GITHUB_REPO = 'seancolsen/querydown';
export const GITHUB_URL = `https://github.com/${GITHUB_REPO}`;
/** Where a file of the repository can be read on GitHub. */
export const githubFileUrl = (path: string) => `${GITHUB_URL}/blob/main/${path}`;
