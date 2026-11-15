// Checks that the built site (`build/`) tells search engines what they need to know about it.
//
// Every page must have exactly one title, description, canonical URL and top-level heading, and
// the Open Graph tags that link previews are made from. No two pages may share a title or a
// description, the canonical URL must be the page's own, and the sitemap must list exactly the
// pages that were built.
//
// This runs at the end of `npm run build`. The tags come from `src/lib/components/PageMeta.svelte`
// and the sitemap from `src/routes/sitemap.xml`, so what this mostly catches is a new page that
// was built without them.

import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';

import rehypeParse from 'rehype-parse';
import { unified } from 'unified';
import { visit } from 'unist-util-visit';
import { toString } from 'hast-util-to-string';

import { BUILD, htmlFiles, ORIGIN, urlOf } from './built-site.js';

/** The page that hosts serve for a URL that isn't a page. It's an empty shell until it loads. */
const NOT_PAGES = new Set(['/404']);
const OPEN_GRAPH = ['og:site_name', 'og:type', 'og:title', 'og:description', 'og:url', 'og:image'];

const parser = unified().use(rehypeParse);
const problems = [];

/** What each page says about itself, by its URL path. Pages that only redirect are left out. */
const pages = new Map();
for (const file of htmlFiles()) {
  const url = urlOf(file);
  if (NOT_PAGES.has(url)) continue;
  const tree = parser.parse(readFileSync(file, 'utf8'));
  const found = { title: [], description: [], canonical: [], h1: [], property: new Map() };
  let redirect = false;
  visit(tree, 'element', (element, _index, parent) => {
    const { name, property, rel, href, httpEquiv, content } = element.properties;
    if (String(httpEquiv).toLowerCase() === 'refresh') redirect = true;
    // The page's own title, as opposed to the title of a drawing within it.
    if (element.tagName === 'title' && parent?.tagName === 'head') {
      found.title.push(toString(element).trim());
    }
    if (element.tagName === 'h1') found.h1.push(toString(element).trim());
    if (element.tagName === 'meta' && name === 'description') found.description.push(content);
    if (element.tagName === 'meta' && typeof property === 'string') {
      found.property.set(property, content);
    }
    if (element.tagName === 'link' && rel?.includes('canonical')) found.canonical.push(href);
  });
  if (!redirect) pages.set(url, found);
}

/** Checks that a page has one of something, and returns it. */
function one(url, what, values) {
  const present = values.filter(Boolean);
  if (present.length !== 1) problems.push(`${url}: has ${present.length} ${what}s, not one.`);
  return present[0];
}

const seen = { title: new Map(), description: new Map() };
for (const [url, found] of pages) {
  one(url, 'top-level heading', found.h1);
  const canonical = one(url, 'canonical URL', found.canonical);
  if (canonical && canonical !== ORIGIN + url) {
    problems.push(`${url}: its canonical URL is ${canonical}, not ${ORIGIN + url}.`);
  }
  for (const what of ['title', 'description']) {
    const value = one(url, what, found[what]);
    const other = seen[what].get(value);
    if (value && other) problems.push(`${url}: has the same ${what} as ${other}.`);
    else seen[what].set(value, url);
  }
  for (const property of OPEN_GRAPH) {
    if (!found.property.get(property)) problems.push(`${url}: has no \`${property}\` tag.`);
  }
  const image = found.property.get('og:image');
  if (
    image &&
    !(image.startsWith(ORIGIN) && existsSync(path.join(BUILD, new URL(image).pathname)))
  ) {
    problems.push(`${url}: its preview image, ${image}, isn't a file of the site.`);
  }
}

// The sitemap, and where search engines learn of it.
const sitemapUrl = `${ORIGIN}/sitemap.xml`;
const sitemapFile = path.join(BUILD, 'sitemap.xml');
if (!existsSync(sitemapFile)) {
  problems.push('There is no sitemap.xml.');
} else {
  const listed = new Set(
    [...readFileSync(sitemapFile, 'utf8').matchAll(/<loc>([^<]*)<\/loc>/g)].map(
      (match) => match[1],
    ),
  );
  const built = new Set([...pages.keys()].map((url) => ORIGIN + url));
  for (const url of built.difference(listed)) problems.push(`sitemap.xml doesn't list ${url}.`);
  for (const url of listed.difference(built)) {
    problems.push(`sitemap.xml lists ${url}, which isn't a page that was built.`);
  }
}
const robots = readFileSync(path.join(BUILD, 'robots.txt'), 'utf8');
if (!robots.split(/\r?\n/).includes(`Sitemap: ${sitemapUrl}`)) {
  problems.push(`robots.txt doesn't name the sitemap (\`Sitemap: ${sitemapUrl}\`).`);
}

if (problems.length > 0) {
  console.error(`Found ${problems.length} problem(s) with what the site tells search engines:\n`);
  for (const problem of problems) console.error(`  ${problem}`);
  process.exit(1);
}
console.log(
  `Checked the titles, descriptions and canonical URLs of ${pages.size} pages, and the sitemap.`,
);
