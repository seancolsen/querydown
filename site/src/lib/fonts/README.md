# Box-drawing font

`box-drawing.woff2` holds only the Box Drawing block (U+2500–257F), which the site's code font, Red Hat Mono, doesn't have. `app.css` declares it as `Querydown Box Drawing` and lists it right after Red Hat Mono in `--font-mono`, so the browser uses it for those characters alone.

Its characters come from [JetBrains Mono](https://github.com/JetBrains/JetBrainsMono) 2.211, whose characters are 0.6em wide like Red Hat Mono's. That keeps a drawing such as the "Query anatomy" diagram in `docs/language.md` lined up with the code above it. JetBrains Mono is licensed under the SIL Open Font License 1.1, copied in [`box-drawing-OFL.txt`](./box-drawing-OFL.txt).

## Regenerating it

The file was made from `ofl/jetbrainsmono/JetBrainsMono[wght].ttf` in the [google/fonts](https://github.com/google/fonts) repository, with [`subset-font`](https://www.npmjs.com/package/subset-font) 2.4.0 (HarfBuzz's subsetter). The variable font is pinned at weight 400:

```js
import { readFileSync, writeFileSync } from 'node:fs';
import subsetFont from 'subset-font';

let text = '';
for (let c = 0x2500; c <= 0x257f; c++) text += String.fromCodePoint(c);

const font = readFileSync('JetBrainsMono[wght].ttf');
const subset = await subsetFont(font, text, {
  targetFormat: 'woff2',
  variationAxes: { wght: 400 },
});
writeFileSync('box-drawing.woff2', subset);
```
