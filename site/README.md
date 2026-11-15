# The Querydown website

This directory holds the code for [querydown.org](https://querydown.org): the home page, the documentation, the release notes, and the playground. It's a [SvelteKit](https://svelte.dev/docs/kit) app that's built into a static site.

Most of what the site shows isn't in this directory. The documentation is Markdown in [`docs/`](../docs), and [`docs/README.md`](../docs/README.md) explains how to write it. This file is about the site's code.

## Running it

You need Node.js 22.17 or later, and (to build the compiler to WebAssembly) Rust and [`wasm-pack`](https://rustwasm.github.io/wasm-pack/). The [dev container](../DEVELOPMENT.md) provides all three.

```sh
npm install
npm run dev
```

`npm run dev` builds the WebAssembly bindings if they haven't been built, and starts a server that reloads the browser as you edit code or Markdown.

| Command               | What it does                                                                                          |
| --------------------- | ----------------------------------------------------------------------------------------------------- |
| `npm run dev`         | Serves the site for development                                                                       |
| `npm run build`       | Builds the bindings and then the site into `build/`, and checks the result for broken links           |
| `npm run preview`     | Serves `build/`, to see the site as it will be deployed                                               |
| `npm run wasm`        | Rebuilds the WebAssembly bindings. **Run this after changing Rust code**, then restart `npm run dev`. |
| `npm run check`       | Type-checks the code                                                                                  |
| `npm run check:links` | Checks `build/` for broken internal links                                                             |
| `npm run format`      | Formats the code with Prettier                                                                        |

To reach the dev server from outside the container, map its port in `docker-compose.yml` (e.g. `ports: ["5173:5173"]`) and run `npm run dev -- --host`.

## Deploying it

`npm run build` writes a static site to `build/`, which any static file host can serve. It expects to be served from the root of its domain, and to have `/docs/language` serve `build/docs/language.html` (as Netlify, Cloudflare Pages, and GitHub Pages do). `build/404.html` is the page for URLs that don't exist.

## How it works

### Content

A page of content is a Markdown file. [`src/lib/server/content.ts`](./src/lib/server/content.ts) maps files to URLs, and [`src/lib/server/markdown.ts`](./src/lib/server/markdown.ts) renders them to HTML, all at build time:

- **Links** between Markdown files are rewritten to the URLs of pages, and a broken link fails the build.
- **Querydown examples** are compiled by the real compiler (to show their SQL, or their error) and highlighted by its tokenizer. Other languages are highlighted by [Shiki](https://shiki.style/). So pages need no JavaScript to show code.
- **Headings** get the same ids as on GitHub, and become the page's table of contents and its entries in the search index.
- **Marker comments** such as `<!-- er-diagram -->` are replaced by Svelte components.
- **A page's stylesheet**, the CSS file beside its Markdown file (such as `content/home.css`), is included in the page.

The navigation tree is defined in [`src/lib/nav.ts`](./src/lib/nav.ts), which is also what decides which files under `docs/` are pages.

### The compiler in the site

The site uses the compiler as WebAssembly, built from [`bindings/js`](../bindings/js) into `site/wasm/` (which Git ignores). The same build is used in two places: on the server while rendering pages, and in the browser by the playground. The [CodeMirror package](../bindings/codemirror) is imported straight from its TypeScript source. Both are wired up as aliases in [`vite.config.ts`](./vite.config.ts).

The colors of highlighted Querydown come from the CodeMirror package's palettes, by way of a generated stylesheet (`/querydown-theme.css`), so that an example on a docs page looks the same as in the playground's editor.

### Theme

The site follows the browser's color scheme, unless the visitor picks a theme with the button in the header. Colors are CSS variables defined once in [`src/app.css`](./src/app.css) with `light-dark()`, and [`src/lib/theme.svelte.ts`](./src/lib/theme.svelte.ts) holds the visitor's choice.

### Fonts

The site uses Red Hat Text for prose, Red Hat Display for headings, and Red Hat Mono for code. Their files come from [Fontsource](https://fontsource.org/) packages, which [`src/routes/+layout.svelte`](./src/routes/+layout.svelte) imports, so Vite copies them into the build and nothing is fetched from a font service. The layout also preloads the upright Latin files, and [`src/app.css`](./src/app.css) names each font in a CSS variable (`--font-sans`, `--font-display`, and `--font-mono`).

Red Hat Mono has no box-drawing characters, so a small font in [`src/lib/fonts/`](./src/lib/fonts) supplies them. Its README says where it came from.

### Search

[`/search.json`](./src/routes/search.json/+server.ts) is an index of every section of every page, built with the site. The search dialog fetches it (and [MiniSearch](https://lucaong.github.io/minisearch/)) when it's first opened. Runs of symbols are indexed as terms, so that searching for `++` or `%%` finds the syntax.

### The playground

[`src/routes/playground/`](./src/routes/playground) is the only part of the site that's an application rather than a document. Its state is kept in the URL's fragment, in the format that [`src/lib/playground-url.ts`](./src/lib/playground-url.ts) describes, which is how every example in the docs can link to itself in the playground. A schema is only put in the URL if it has been edited.

### For AI agents

[`/llms.txt`](./src/routes/llms.txt/+server.ts) lists the site's pages, and [`/llms-full.txt`](./src/routes/llms-full.txt/+server.ts) is all of the documentation's Markdown in one file.

### Checks

- The build fails on a broken link between Markdown files, on an example that doesn't compile (or one marked `error` that does), and on a docs page that's missing from the navigation.
- [`scripts/check-links.js`](./scripts/check-links.js) then checks every internal link and fragment in the built HTML.
- `cargo test` (from the repository root) tests every example in the docs. See [`docs/README.md`](../docs/README.md#how-examples-are-tested).
