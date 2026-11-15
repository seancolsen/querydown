# Box-drawing font

`box-drawing.woff2` is a 2 KB font that holds only the Box Drawing block (U+2500–257F), taken from [JetBrains Mono](https://github.com/JetBrains/JetBrainsMono). [`app.css`](../../app.css) declares it as `Querydown Box Drawing` and lists it right after Red Hat Mono in `--font-mono`, so the browser uses it for those characters alone.

## Why it exists

The docs draw with box-drawing characters in code blocks. The "Query anatomy" diagram at the top of [`docs/language.md`](../../../../docs/language.md) brackets the parts of a query, and its labels sit beneath it:

```
 #issues   status:"open" due_date:<@now   $id $title $author.*   /*
╰───────╯ ╰────────────────────────────╯ ╰────────────────────╯   *
Base table          Conditions               Result columns       */
```

Red Hat Mono, the site's code font, has no box-drawing characters. That's true of the font itself, not only of the subsets that Fontsource and Google Fonts serve, so a newer release won't help. Without this font, the browser takes those characters from the visitor's system monospace font. Some of those fonts have characters of a different width (Consolas, on Windows, is 0.55em, not 0.6em), which pushes the brackets out of line with the code and the labels.

## Why JetBrains Mono

The font for these characters needed to:

- **have all 128 characters** of the block;
- **be as wide as Red Hat Mono**, whose characters are 0.6em wide (600 of its 1000 units per em), so each character takes the same space as the code around it;
- **have similar vertical metrics**, so that a line of box-drawing characters isn't taller than a line of code. JetBrains Mono's ascent and descent are 1020 and −300, against Red Hat Mono's 1018 and −305;
- **allow a modified copy to be shipped.** JetBrains Mono's license is the SIL Open Font License 1.1, copied in [`box-drawing-OFL.txt`](./box-drawing-OFL.txt). It names no Reserved Font Name, so a subset can be distributed under a name of its own.

## How it's made

[`site/scripts/subset-box-drawing.js`](../../../scripts/subset-box-drawing.js) makes the file. It:

1. downloads `JetBrainsMono[wght].ttf` (version 2.211) from the [google/fonts](https://github.com/google/fonts) repository, at the commit that added that version, and checks the file's SHA-256 hash;
2. keeps the box-drawing characters and drops the rest, using [`subset-font`](https://www.npmjs.com/package/subset-font), which runs HarfBuzz's subsetter. It fixes the variable font's weight at 400 (regular), which leaves a static font, and writes it as WOFF2;
3. checks the result with [`fontkit`](https://www.npmjs.com/package/fontkit): all 128 characters are there, and they're exactly as wide as Red Hat Mono's characters, read from the Red Hat Mono file the site ships;
4. writes `box-drawing.woff2`, and says whether it's identical to the file that was there.

The build doesn't run the script. The font is committed, and it only needs making again if the site's code font changes. Vite inlines it into the site's CSS, since it's smaller than Vite's 4 KB limit for inlining assets.

## Running the script

The script needs two packages that the site doesn't depend on. From `site/`, install them without adding them to `package.json`, and then run it:

```sh
npm install --no-save subset-font@2.4.0 fontkit@2.0.4
node scripts/subset-box-drawing.js
```

With those versions, it makes a file identical to the one committed. It needs network access to download the source font. The next `npm install` (or `npm prune`) removes the two packages again.

To check the result on the page, run `npm run build` and `npm run preview`, open `/docs/language`, and look at the "Query anatomy" diagram: each bracket should span the part of the query above it. To measure it, run this in the browser's console. It prints the width per character of each line of the diagram, which should be the same for every line:

```js
const node = [...document.querySelectorAll('pre.qd .qd-comment')].find((e) =>
  e.textContent.includes('╰'),
).firstChild;
let start = 0;
for (const line of node.textContent.split('\n')) {
  const range = document.createRange();
  range.setStart(node, start);
  range.setEnd(node, start + line.length);
  console.log((range.getBoundingClientRect().width / line.length).toFixed(2), line);
  start += line.length + 1;
}
```

## If the code font changes

First check whether the new font has box-drawing characters of its own. If it does, delete this font and its `@font-face` in `app.css`. If it doesn't, but its characters are still 0.6em wide, this font still fits. Otherwise, choose a source font that meets the conditions above for the new font, and change `SOURCE_URL` and `SOURCE_SHA256` in the script. The script's width check compares against the Red Hat Mono file, so that path needs changing too.
