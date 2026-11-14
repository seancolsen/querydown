// Checks the built site (`build/`) for broken internal links.
//
// For every page, each link, image, script and stylesheet that points within the site must lead to
// a file that was built, and each fragment (`#section`) must match the id of an element on the
// page it points at. External links aren't followed.
//
// This runs at the end of `npm run build`. It overlaps with the checks that SvelteKit makes while
// prerendering, and with those the Markdown renderer makes of links between Markdown files, but it
// looks at the site as it was actually built, whatever the links came from.

import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import rehypeParse from 'rehype-parse';
import { unified } from 'unified';
import { visit } from 'unist-util-visit';

const BUILD = fileURLToPath(new URL('../build', import.meta.url));
const ORIGIN = 'https://querydown.org';
/** Pages whose fragment isn't the id of an element. The playground keeps its state there. */
const FREE_FRAGMENTS = new Set(['/playground']);

if (!existsSync(BUILD)) {
  console.error('There is no build to check. Run `npm run build` first.');
  process.exit(1);
}

function htmlFiles(dir) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const file = path.join(dir, entry.name);
    if (entry.isDirectory()) return htmlFiles(file);
    return entry.name.endsWith('.html') ? [file] : [];
  });
}

/** The URL path that a built HTML file is served at. */
function urlOf(file) {
  const relative = path.relative(BUILD, file).split(path.sep).join('/');
  if (relative === 'index.html') return '/';
  return `/${relative.replace(/(\/index)?\.html$/, '')}`;
}

/** The built file that serves a URL path, if there is one. */
function fileOf(pathname) {
  const clean = decodeURIComponent(pathname).replace(/\/$/, '');
  const candidates = [clean, `${clean}.html`, `${clean}/index.html`].map((p) =>
    path.join(BUILD, p),
  );
  return candidates.find((file) => existsSync(file) && statSync(file).isFile());
}

const parser = unified().use(rehypeParse);

/** The ids and the references of each page, read once. */
const pages = new Map();
for (const file of htmlFiles(BUILD)) {
  const tree = parser.parse(readFileSync(file, 'utf8'));
  const ids = new Set();
  const references = [];
  let redirect = null;
  visit(tree, 'element', (element) => {
    const { id, href, src, httpEquiv, content } = element.properties;
    if (typeof id === 'string') ids.add(id);
    if (typeof href === 'string') references.push(href);
    if (typeof src === 'string') references.push(src);
    if (String(httpEquiv).toLowerCase() === 'refresh' && typeof content === 'string') {
      redirect = /url=(.*)$/i.exec(content)?.[1] ?? null;
    }
  });
  if (redirect) references.push(redirect);
  pages.set(file, { url: urlOf(file), ids, references });
}

const problems = [];
let checked = 0;

for (const [, page] of pages) {
  for (const reference of new Set(page.references)) {
    const target = new URL(reference, ORIGIN + page.url);
    if (target.origin !== ORIGIN) continue;
    checked += 1;
    const file = fileOf(target.pathname);
    if (!file) {
      problems.push(`${page.url}: \`${reference}\` leads to a page that doesn't exist.`);
      continue;
    }
    const fragment = decodeURIComponent(target.hash.slice(1));
    const destination = pages.get(file);
    if (!fragment || !destination || FREE_FRAGMENTS.has(destination.url)) continue;
    if (!destination.ids.has(fragment)) {
      problems.push(
        `${page.url}: \`${reference}\` leads to \`#${fragment}\`, which isn't on ${destination.url}.`,
      );
    }
  }
}

if (problems.length > 0) {
  console.error(`Found ${problems.length} broken link(s):\n`);
  for (const problem of problems) console.error(`  ${problem}`);
  process.exit(1);
}
console.log(`Checked ${checked} internal links on ${pages.size} pages. None are broken.`);
