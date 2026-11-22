# The Querydown website

This directory holds the code for [querydown.org](https://querydown.org): the home page, the documentation, the release notes, and the playground. It's a [SvelteKit](https://svelte.dev/docs/kit) app that's built into a static site.

Most of what the site shows isn't in this directory. The documentation is Markdown in [`docs/`](../docs), and [`docs/README.md`](../docs/README.md) explains how to write it. This file is about the site's code.

## Running it

You need Node.js 22.17 or later, and (to build the compiler to WebAssembly) Rust and [`wasm-pack`](https://rustwasm.github.io/wasm-pack/). The [dev container](../DEVELOPMENT.md) provides all three.

```sh
npm install
npm run dev
```

To start the dev server _outside the container_ run:

```
docker compose run --rm -p 5173:5173 -w /workspace/site dev sh -c "npm install && npm run dev -- --host"
```

`npm run dev` builds the WebAssembly bindings if they haven't been built, and starts a server that reloads the browser as you edit code or Markdown.

| Command               | What it does                                                                                          |
| --------------------- | ----------------------------------------------------------------------------------------------------- |
| `npm run dev`         | Serves the site for development                                                                       |
| `npm run build`       | Builds the bindings and then the site into `build/`, and checks the result (the two checks below)     |
| `npm run preview`     | Serves `build/`, to see the site as it will be deployed                                               |
| `npm run wasm`        | Rebuilds the WebAssembly bindings. **Run this after changing Rust code**, then restart `npm run dev`. |
| `npm run check`       | Type-checks the code                                                                                  |
| `npm run check:links` | Checks `build/` for broken internal links                                                             |
| `npm run check:seo`   | Checks what the pages in `build/` tell search engines, and the sitemap                                |
| `npm run format`      | Formats the code with Prettier                                                                        |

To reach the dev server from outside the container, map its port in `docker-compose.yml` (e.g. `ports: ["5173:5173"]`) and run `npm run dev -- --host`.

## Deploying it

`npm run build` writes a static site to `build/`, which any static file host can serve. It expects to be served from the root of its domain, and to have `/docs/language` serve `build/docs/language.html` (as Netlify, Cloudflare Pages, and GitHub Pages do). `build/404.html` is the page for URLs that don't exist.

querydown.org is built by Netlify from the `stable` branch, on every push to it, so that the site matches the latest release. [DEVELOPMENT.md](../DEVELOPMENT.md#branches) explains which changes go on `stable` and which on `main`.

## How it works

### Content

A page of content is a Markdown file. [`src/lib/server/content.ts`](./src/lib/server/content.ts) maps files to URLs, and [`src/lib/server/markdown.ts`](./src/lib/server/markdown.ts) renders them to HTML, all at build time:

- **Links** between Markdown files are rewritten to the URLs of pages, and a broken link fails the build.
- **Querydown examples** are compiled by the real compiler (to show their SQL, or their error) and highlighted by its tokenizer. Other languages are highlighted by [Shiki](https://shiki.style/). So pages need no JavaScript to show code.
- **Headings** get the same ids as on GitHub, and become the page's table of contents and its entries in the search index.
- **Marker comments** such as `<!-- er-diagram -->` are replaced by Svelte components.
- **A page's stylesheet**, the CSS file beside its Markdown file (such as `content/home.css`), is included in the page.

The navigation tree is defined in [`src/lib/nav.ts`](./src/lib/nav.ts), which is also what decides which files under `docs/` are pages. Beneath the current page, the tree lists that page's table of contents, and marks the section being read (which [`src/lib/reading.svelte.ts`](./src/lib/reading.svelte.ts) keeps track of). On a narrow screen, the tree is in a drawer that the header's menu button opens.

### The compiler in the site

The site uses the compiler as WebAssembly, built from [`bindings/js`](../bindings/js) into `site/wasm/` (which Git ignores). The same build is used in two places: on the server while rendering pages, and in the browser by the playground. The [CodeMirror package](../bindings/codemirror) is imported straight from its TypeScript source. Both are wired up as aliases in [`vite.config.ts`](./vite.config.ts).

The colors of highlighted Querydown come from the CodeMirror package's palettes, by way of a generated stylesheet (`/querydown-theme.css`), so that an example on a docs page looks the same as in the playground's editor.

### Theme

The site follows the browser's color scheme, unless the visitor picks a theme with the button in the header. Colors are CSS variables defined once in [`src/app.css`](./src/app.css) with `light-dark()`, and [`src/lib/theme.svelte.ts`](./src/lib/theme.svelte.ts) holds the visitor's choice.

### Fonts

The site uses Red Hat Text for prose, Red Hat Display for headings, and Red Hat Mono for code. They were chosen over other warm, humanist sans-serifs (such as Source Sans, Fira Sans, and Recursive) because they're a family designed together: a text cut drawn for reading sizes, a display cut for headings, and a mono that matches them.

The font files come from [Fontsource](https://fontsource.org/) packages, which [`src/routes/+layout.svelte`](./src/routes/+layout.svelte) imports, so Vite copies them into the build and nothing is fetched from a font service. The layout also preloads the upright Latin files, and [`src/app.css`](./src/app.css) names each font in a CSS variable (`--font-sans`, `--font-display`, and `--font-mono`).

- Headings from h1 to h3 use the display cut. An h4 is the size of the text, so it keeps the text cut.
- Code is shown without ligatures, so that the syntax looks as it's typed: `:=` is two characters, not one glyph.
- Red Hat Mono has no box-drawing characters, so a small font in [`src/lib/fonts/`](./src/lib/fonts) supplies them, made by [`scripts/subset-box-drawing.js`](./scripts/subset-box-drawing.js). The README in that folder explains why it's needed, how it was made, and how to make it again.

### Search

[`/search.json`](./src/routes/search.json/+server.ts) is an index of every section of every page, built with the site. The search dialog fetches it (and [MiniSearch](https://lucaong.github.io/minisearch/)) when it's first opened. Runs of symbols are indexed as terms, so that searching for `++` or `%%` finds the syntax.

### The playground

[`src/routes/playground/`](./src/routes/playground) is the only part of the site that's an application rather than a document. Its state is kept in the URL's fragment, in the format that [`src/lib/playground-url.ts`](./src/lib/playground-url.ts) describes, which is how every example in the docs can link to itself in the playground. A schema is only put in the URL if it has been edited.

### For search engines

Every page tells search engines and link previews about itself with [`PageMeta.svelte`](./src/lib/components/PageMeta.svelte), which writes these into the page's `<head>`:

- **A title.** For a page in the navigation tree, it's the page's own title, then the groups that the page is in, then the site: "CLI · Setup · Docs · Querydown".
- **A description**: the page's first paragraph, shortened to the sentences that a search engine has room to show (see [`src/lib/seo.ts`](./src/lib/seo.ts)).
- **A canonical URL**, because a static host serves a page at more than one address (`/docs/language`, `/docs/language.html`, `/docs/language/`), and search engines should count them as one.
- **Open Graph tags**, which chat apps and social networks make a link's preview from. The preview's picture is the same for every page: `static/social-card.png`, made by [`scripts/make-social-card.js`](./scripts/make-social-card.js). It's committed, and needs making again only if the branding, the tagline, or the fonts change.

The home page also says what the site is called, as schema.org data. The page for URLs that don't exist asks not to be indexed.

[`/sitemap.xml`](./src/routes/sitemap.xml/+server.ts) lists every page by its canonical URL, and `robots.txt` says where it is. URLs that only redirect (`/docs`, `/releases`) are left out of it.

A new kind of page needs a `PageMeta` of its own, and a place in the sitemap. The build fails without them.

### For AI agents

[`/llms.txt`](./src/routes/llms.txt/+server.ts) lists the site's pages, and [`/llms-full.txt`](./src/routes/llms-full.txt/+server.ts) is all of the documentation's Markdown in one file.

### Checks

- The build fails on a broken link between Markdown files, on an example that doesn't compile (or one marked `error` that does), and on a docs page that's missing from the navigation.
- [`scripts/check-links.js`](./scripts/check-links.js) then checks every internal link and fragment in the built HTML.
- [`scripts/check-seo.js`](./scripts/check-seo.js) checks that every built page has one title, description, canonical URL, and top-level heading, that no two pages share a title or a description, and that the sitemap lists exactly the pages that were built.
- `cargo test` (from the repository root) tests every example in the docs. See [`docs/README.md`](../docs/README.md#how-examples-are-tested).
