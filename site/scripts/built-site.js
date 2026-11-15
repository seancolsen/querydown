// What the scripts that check the built site (`build/`) have in common: where it is, and how its
// files map to URLs.

import { existsSync, readdirSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export const BUILD = fileURLToPath(new URL('../build', import.meta.url));
export const ORIGIN = 'https://querydown.org';

if (!existsSync(BUILD)) {
  console.error('There is no build to check. Run `npm run build` first.');
  process.exit(1);
}

/** Every HTML file within a directory of the build. */
export function htmlFiles(dir = BUILD) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const file = path.join(dir, entry.name);
    if (entry.isDirectory()) return htmlFiles(file);
    return entry.name.endsWith('.html') ? [file] : [];
  });
}

/** The URL path that a built HTML file is served at. */
export function urlOf(file) {
  const relative = path.relative(BUILD, file).split(path.sep).join('/');
  if (relative === 'index.html') return '/';
  return `/${relative.replace(/(\/index)?\.html$/, '')}`;
}
