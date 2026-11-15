/**
 * Finds the site's Markdown content and maps it to pages.
 *
 * Content lives in two places, each mirroring a part of the site's URL space:
 *
 * | Files                                   | Pages         |
 * | --------------------------------------- | ------------- |
 * | Markdown files anywhere under `docs`    | `/docs/…`     |
 * | Those in `site/content/releases`        | `/releases/…` |
 * | `site/content/home.md`                  | `/`           |
 *
 * Links between Markdown files are written as relative paths to the `.md` file (so that they work
 * on GitHub too) and are rewritten here to the URLs of the pages.
 *
 * A page can have a stylesheet of its own: a CSS file beside its Markdown file, with the same name
 * (`site/content/home.css` for `site/content/home.md`). The page includes it, so that its Markdown
 * can style the HTML it contains, by class name.
 */

import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import path from 'node:path';

import type { Page } from '../content-types';
import { DOCS, pagesOf, PLAYGROUND, RELEASES_PATH, type NavPage } from '../nav';
import { GITHUB_URL } from '../site';
import { headingIds, parseMarkdown, renderMarkdown } from './markdown';
import { repoPath } from './paths';
import { SAMPLE_SCHEMA_FILE } from './querydown';

export const DOCS_DIR = 'docs';
export const RELEASES_DIR = 'site/content/releases';
export const HOME_FILE = 'site/content/home.md';

// -------------------------------------------------------------------------------------------------
// Files and URLs
// -------------------------------------------------------------------------------------------------

/** Markdown files that sit among the content but aren't pages, such as `docs/README.md`. */
const isInternal = (file: string) => path.basename(file) === 'README.md';

function markdownFiles(dir: string): string[] {
  return readdirSync(repoPath(dir), { withFileTypes: true }).flatMap((entry) => {
    const file = `${dir}/${entry.name}`;
    if (entry.isDirectory()) return markdownFiles(file);
    return entry.name.endsWith('.md') && !isInternal(file) ? [file] : [];
  });
}

/** The URL of the page that a Markdown file becomes, or `null` if the file isn't a page. */
export function urlOfFile(file: string): string | null {
  if (!file.endsWith('.md') || isInternal(file)) return null;
  const name = file.slice(0, -'.md'.length);
  if (file === HOME_FILE) return '/';
  if (name.startsWith(`${DOCS_DIR}/`)) return `/${name}`;
  if (name.startsWith(`${RELEASES_DIR}/`)) {
    return `${RELEASES_PATH}/${name.slice(RELEASES_DIR.length + 1)}`;
  }
  return null;
}

/** The Markdown file behind a page's URL, or `null` if no file is. */
export function fileOfUrl(url: string): string | null {
  let file: string;
  if (url === '/') file = HOME_FILE;
  else if (url.startsWith('/docs/')) file = `${url.slice(1)}.md`;
  else if (url.startsWith(`${RELEASES_PATH}/`)) {
    file = `${RELEASES_DIR}/${url.slice(RELEASES_PATH.length + 1)}.md`;
  } else return null;
  return existsSync(repoPath(file)) && urlOfFile(file) === url ? file : null;
}

/** The stylesheet that goes with a Markdown file, whether or not it exists. */
const stylesheetOf = (file: string) => file.replace(/\.md$/, '.css');

/** Compares version numbers such as `0.10.0` and `0.9.2`, for sorting in descending order. */
function compareVersionsDescending(a: string, b: string): number {
  return b.localeCompare(a, undefined, { numeric: true });
}

/** The releases that have a page, newest first. */
export function releases(): string[] {
  return markdownFiles(RELEASES_DIR)
    .map((file) => path.basename(file, '.md'))
    .sort(compareVersionsDescending);
}

export function releasePages(): NavPage[] {
  return releases().map((version) => ({ title: version, href: `${RELEASES_PATH}/${version}` }));
}

/**
 * The URL of every page that comes from Markdown, in navigation order.
 *
 * Also checks that the docs in the navigation tree and the files under `docs/` agree, so that a
 * page can't go missing from the navigation, nor the navigation point at a page that isn't there.
 */
export function pageUrls(): string[] {
  const docs = pagesOf(DOCS).map((page) => page.href);
  const files = markdownFiles(DOCS_DIR).map((file) => urlOfFile(file));
  for (const url of docs) {
    if (!files.includes(url)) {
      throw new Error(`The navigation lists ${url}, but there's no ${url.slice(1)}.md.`);
    }
  }
  for (const url of files) {
    if (url && !docs.includes(url)) {
      throw new Error(`${url.slice(1)}.md isn't listed in the navigation (site/src/lib/nav.ts).`);
    }
  }
  return ['/', ...releasePages().map((page) => page.href), ...docs];
}

// -------------------------------------------------------------------------------------------------
// Links
// -------------------------------------------------------------------------------------------------

const idCache = new Map<string, { modified: number; ids: Set<string> }>();

/** The ids of the headings in a Markdown file. */
function idsOfFile(file: string): Set<string> {
  const modified = statSync(repoPath(file)).mtimeMs;
  const cached = idCache.get(file);
  if (cached?.modified === modified) return cached.ids;
  const ids = headingIds(parseMarkdown(readFileSync(repoPath(file), 'utf8')));
  idCache.set(file, { modified, ids });
  return ids;
}

/**
 * Turns the URL of a link written in `file` into the URL to use on the site:
 *
 * - A link to another Markdown page becomes a link to that page.
 * - A link to any other file in the repository becomes a link to the file on GitHub.
 * - Anything else (an external URL, an absolute path within the site) is left alone.
 *
 * Throws if the link points at a file or a heading that doesn't exist. (Absolute paths within the
 * site aren't checked here, but `scripts/check-links.js` checks them in the built site.)
 */
function resolveLink(file: string, url: string, ids: Set<string>): string {
  if (/^[a-z][a-z0-9+.-]*:/i.test(url) || url.startsWith('/')) return url;
  const broken = (why: string) => new Error(`${file}: broken link \`${url}\`: ${why}`);
  const [target, fragment] = url.split('#', 2);
  if (target === '') {
    if (!ids.has(fragment)) throw broken('no heading on this page has that id.');
    return url;
  }
  const targetFile = path.posix.normalize(path.posix.join(path.posix.dirname(file), target));
  if (targetFile.startsWith('..')) throw broken('it leads outside of the repository.');
  if (!existsSync(repoPath(targetFile))) throw broken(`${targetFile} doesn't exist.`);
  const pageUrl = urlOfFile(targetFile);
  if (pageUrl === null) {
    const kind = statSync(repoPath(targetFile)).isDirectory() ? 'tree' : 'blob';
    return `${GITHUB_URL}/${kind}/main/${targetFile}${fragment ? `#${fragment}` : ''}`;
  }
  if (fragment === undefined) return pageUrl;
  if (!idsOfFile(targetFile).has(fragment)) {
    throw broken(`no heading in ${targetFile} has the id \`${fragment}\`.`);
  }
  return `${pageUrl}#${fragment}`;
}

// -------------------------------------------------------------------------------------------------
// Pages
// -------------------------------------------------------------------------------------------------

const pageCache = new Map<string, { stamp: string; page: Promise<Page> }>();

/**
 * A stamp that changes whenever anything a page is rendered from might have: its own file and its
 * stylesheet, the files it may link to (whose headings are checked), or the schema its examples
 * are compiled with.
 */
function contentStamp(): string {
  const pages = [HOME_FILE, ...markdownFiles(DOCS_DIR), ...markdownFiles(RELEASES_DIR)];
  return [SAMPLE_SCHEMA_FILE, ...pages, ...pages.map(stylesheetOf)]
    .map((file) => `${file}:${existsSync(repoPath(file)) ? statSync(repoPath(file)).mtimeMs : '-'}`)
    .join('|');
}

/** Renders the page at `url`, or returns `null` if no Markdown file is behind that URL. */
export async function getPage(url: string): Promise<Page | null> {
  const file = fileOfUrl(url);
  if (file === null) return null;
  const stamp = contentStamp();
  const cached = pageCache.get(url);
  if (cached?.stamp === stamp) return cached.page;
  const stylesheet = repoPath(stylesheetOf(file));
  const styles = existsSync(stylesheet) ? readFileSync(stylesheet, 'utf8') : null;
  const page = renderMarkdown(readFileSync(repoPath(file), 'utf8'), {
    file,
    resolveLink: (link, ids) => resolveLink(file, link, ids),
    playgroundUrl: PLAYGROUND.href,
  }).then((rendered) => ({ ...rendered, url, file, styles }));
  pageCache.set(url, { stamp, page });
  return page;
}

/** Every page that comes from Markdown, in navigation order. */
export async function allPages(): Promise<Page[]> {
  const pages = await Promise.all(pageUrls().map((url) => getPage(url)));
  return pages.filter((page) => page !== null);
}

/** The Markdown source of a page's file. */
export function sourceOf(page: Page): string {
  return readFileSync(repoPath(page.file), 'utf8');
}
