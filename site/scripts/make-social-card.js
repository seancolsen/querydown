// Makes `static/social-card.png`: the picture that a link to the site is previewed with, in chat
// apps and on social networks (see `SOCIAL_CARD` in `src/lib/site.ts`, and the `og:image` tag in
// `src/lib/components/PageMeta.svelte`). It shows the logo and the wordmark, as at the top of the
// home page, above the site's tagline.
//
// This isn't part of the build. The picture it makes is committed, and only needs making again if
// the branding, the tagline or the site's fonts change. It draws the card as a web page and has a
// browser take a screenshot of it, so it needs a Chromium or Chrome to run:
//
//     CHROMIUM=/path/to/chrome node scripts/make-social-card.js

import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

/** The size that link previews are designed around: 1.91 to 1, and sharp on dense screens. */
const WIDTH = 1200;
const HEIGHT = 630;

/** The site's light theme (see `src/app.css`), which suits a preview wherever it's shown. */
const BACKGROUND = '#ffffff';
const TEXT = '#1b1d21';
const TEXT_MUTED = '#5a6270';

const repo = (file) => fileURLToPath(new URL(`../../${file}`, import.meta.url));
const OUTPUT = repo('site/static/social-card.png');

const chromium = process.env.CHROMIUM;
if (!chromium) {
  console.error(
    'This script needs a browser to draw with. Give it the path of a Chromium or Chrome:\n\n' +
      '    CHROMIUM=/path/to/chrome node scripts/make-social-card.js\n',
  );
  process.exit(1);
}

/** The value of a string constant in `src/lib/site.ts`, so that the card says what the site does. */
function siteConstant(name) {
  const source = readFileSync(repo('site/src/lib/site.ts'), 'utf8');
  const match = new RegExp(`export const ${name} = '([^']*)';`).exec(source);
  if (!match) throw new Error(`Couldn't find ${name} in src/lib/site.ts.`);
  return match[1];
}

/** A branding SVG, to put inline in the page. The logo's white backing is left as it is. */
const branding = (name) =>
  readFileSync(repo(`branding/${name}`), 'utf8').replace(/<\?xml[^>]*\?>/, '');

const require = createRequire(import.meta.url);
/** The font of the site's headings, which the tagline is set in. */
const displayFont = pathToFileURL(
  require.resolve('@fontsource-variable/red-hat-display/files/red-hat-display-latin-wght-normal.woff2'),
);

const html = `<!doctype html>
<meta charset="utf-8" />
<style>
  @font-face {
    font-family: 'Red Hat Display';
    font-weight: 300 900;
    src: url('${displayFont}') format('woff2');
  }
  html, body { margin: 0; }
  body {
    width: ${WIDTH}px;
    height: ${HEIGHT}px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 56px;
    background: ${BACKGROUND};
    color: ${TEXT};
    font-family: 'Red Hat Display';
  }
  /* The logo beside the wordmark, in the same proportion as in the site's header. */
  .hero { --size: 150px; display: flex; align-items: center; gap: calc(var(--size) * 0.3); }
  .hero svg { width: auto; }
  .hero svg:first-child { height: var(--size); }
  /* The logo's own black, brought into line with the wordmark's. */
  .hero svg:first-child path:last-child { fill: currentColor !important; }
  .hero svg:last-child { height: calc(var(--size) * 0.66); fill: currentColor; }
  p { margin: 0; color: ${TEXT_MUTED}; font-size: 46px; font-weight: 500; }
</style>
<div class="hero">${branding('logo-optimized.svg')}${branding('name-optimized.svg')}</div>
<p>${siteConstant('SITE_TAGLINE')}</p>
`;

const dir = mkdtempSync(path.join(tmpdir(), 'social-card-'));
try {
  const page = path.join(dir, 'card.html');
  writeFileSync(page, html);
  execFileSync(
    chromium,
    [
      '--headless',
      '--no-sandbox',
      '--disable-gpu',
      '--hide-scrollbars',
      '--force-device-scale-factor=1',
      `--window-size=${WIDTH},${HEIGHT}`,
      // Gives the font time to load before the screenshot is taken.
      '--virtual-time-budget=5000',
      `--screenshot=${OUTPUT}`,
      pathToFileURL(page).href,
    ],
    // A headless browser grumbles a good deal as it starts. What it says is only shown if it fails.
    { stdio: 'pipe' },
  );
} finally {
  rmSync(dir, { recursive: true, force: true });
}
console.log(`Wrote ${path.relative(process.cwd(), OUTPUT)}.`);
