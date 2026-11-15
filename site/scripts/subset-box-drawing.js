// Makes `src/lib/fonts/box-drawing.woff2`: the box-drawing characters (U+2500–257F) of JetBrains
// Mono, as a font of their own. The site lists it right after Red Hat Mono, which has none of those
// characters. `src/lib/fonts/README.md` explains why, and how to run this.
//
// This isn't part of the build. The font it makes is committed, and only needs making again if the
// site's code font changes. It needs two packages that the site doesn't depend on:
//
//     npm install --no-save subset-font@2.4.0 fontkit@2.0.4
//     node scripts/subset-box-drawing.js

import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';

/** JetBrains Mono 2.211, as Google Fonts publishes it, at the commit that added that version. */
const SOURCE_URL =
  'https://raw.githubusercontent.com/google/fonts/2e05c1cf00a6e4f40a4b931600a90881c26e15cd/ofl/jetbrainsmono/JetBrainsMono%5Bwght%5D.ttf';
const SOURCE_SHA256 = '48715a42ec242c21e9f02692891e147d022299a52e48d5e413e1a942193ffeda';

/** The Box Drawing block. */
const FIRST = 0x2500;
const LAST = 0x257f;

/** The weight to fix the variable font at: regular, like the code it sits beside. */
const WEIGHT = 400;

const OUTPUT = fileURLToPath(new URL('../src/lib/fonts/box-drawing.woff2', import.meta.url));

const require = createRequire(import.meta.url);
/** The site's code font, whose character width the box-drawing characters must match. */
const RED_HAT_MONO =
  require.resolve('@fontsource-variable/red-hat-mono/files/red-hat-mono-latin-wght-normal.woff2');

let subsetFont, fontkit;
try {
  subsetFont = (await import('subset-font')).default;
  fontkit = await import('fontkit');
} catch {
  console.error(
    'This script needs packages that the site does not depend on. Install them with:\n\n' +
      '    npm install --no-save subset-font@2.4.0 fontkit@2.0.4\n',
  );
  process.exit(1);
}

/** Stops with a message, for a check that failed. */
function fail(message) {
  console.error(`Error: ${message}`);
  process.exit(1);
}

/** The width of a font's `0`, as a fraction of an em. Every character of a monospaced font has it. */
function charWidth(font) {
  return font.glyphForCodePoint(0x30).advanceWidth / font.unitsPerEm;
}

/** The code points in the range that the font has glyphs for. */
function covered(font) {
  const points = [];
  for (let c = FIRST; c <= LAST; c++) if (font.hasGlyphForCodePoint(c)) points.push(c);
  return points;
}

// Download the source font, and make sure it's the file this was written for.
console.log(`Downloading ${SOURCE_URL}`);
const response = await fetch(SOURCE_URL);
if (!response.ok) fail(`the download failed: ${response.status} ${response.statusText}`);
const source = Buffer.from(await response.arrayBuffer());
const sha256 = createHash('sha256').update(source).digest('hex');
if (sha256 !== SOURCE_SHA256) {
  fail(`the downloaded font isn't the expected file (its SHA-256 is ${sha256}).`);
}

const sourceFont = fontkit.create(source);
const redHatMono = fontkit.create(readFileSync(RED_HAT_MONO));
const total = LAST - FIRST + 1;
if (covered(sourceFont).length !== total)
  fail('the source font lacks some box-drawing characters.');

// Keep only the box-drawing characters, at one weight, as WOFF2.
let text = '';
for (let c = FIRST; c <= LAST; c++) text += String.fromCodePoint(c);
const subset = await subsetFont(source, text, {
  targetFormat: 'woff2',
  variationAxes: { wght: WEIGHT },
});

// Check the result: every character, each as wide as Red Hat Mono's characters.
const font = fontkit.create(subset);
const count = covered(font).length;
if (count !== total) fail(`the subset has ${count} of the ${total} box-drawing characters.`);
const width = font.glyphForCodePoint(FIRST).advanceWidth / font.unitsPerEm;
if (width !== charWidth(redHatMono)) {
  fail(`its characters are ${width}em wide, but Red Hat Mono's are ${charWidth(redHatMono)}em.`);
}

let previous = null;
try {
  previous = readFileSync(OUTPUT);
} catch {}
writeFileSync(OUTPUT, subset);

console.log(
  `Wrote ${OUTPUT}: ${subset.length} bytes, ${count} characters, each ${width}em wide.` +
    (previous?.equals(subset) ? ' It is identical to the file that was there.' : ''),
);
