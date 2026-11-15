# Querydown documentation content

This directory holds the documentation content that's published at [querydown.org](https://querydown.org), as Markdown. This file is for the people (and the AI agents) who maintain it: it explains how the content is organized, the conventions it follows, and how it's tested. It isn't a page of the site.

To build and preview the site locally, see [site/README.md](../site/README.md).

To read the docs, start at [language.md](./language.md), or visit the site.

## Where things are

| Content | Lives in | Becomes |
| -- | -- | -- |
| The docs | `docs/**/*.md` (this directory) | `/docs/…` |
| The home page | [`site/content/home.md`](../site/content/home.md) | `/` |
| Release notes | [`site/content/releases/`](../site/content/releases), one file per release | `/releases/…` |
| The playground | [`site/src/routes/playground/`](../site/src/routes/playground) (not Markdown) | `/playground` |

A Markdown file's path is its URL: `docs/setup/cli.md` is the page `/docs/setup/cli`. Files named `README.md` are never pages.

The site itself (a SvelteKit app that renders these files) is in [`site/`](../site), and its [README](../site/README.md) explains how to run it. While `npm run dev` is running there, the browser reloads a page whenever you save its Markdown file.

### Adding, moving, and removing pages

The site's navigation is a tree, defined in [`site/src/lib/nav.ts`](../site/src/lib/nav.ts). An item of the tree is either a page or a group of items, never both. (The URL of a group, like `/docs/setup`, redirects to the group's first page.)

- **To add a docs page**, create the Markdown file, and list it in `nav.ts`. The build fails if the two disagree.
- **To add a release**, create `site/content/releases/<version>.md`. Releases are found by themselves, and are listed newest first.
- **To move or rename a page**, also update the links to it. The build fails on the ones you miss.

## Who you're writing for

- **People evaluating Querydown**: application developers, data engineers, and BI analysts deciding whether it suits them. They need to see quickly what Querydown is, what it isn't, and how to try it.
- **People installing and embedding it**: technical, with varying commitment. They want steps that work.
- **People learning the language**, some of them not very technical. They want one feature at a time, shown by example.
- **People who know the language** and want a quick answer about a piece of syntax or a function.
- **AI agents** answering questions for any of the above. They read the same pages (and `/llms.txt`), so what's clear for a person is clear for them.
- **Skeptics**, from Hacker News and the like. They're won over by candor, not by claims.

## Tone

- **Understated and matter-of-fact.** We're not selling anything. Boring is fine. No marketing fluff, no superlatives, no exclamation marks to drum up excitement.
- **Upfront about limitations.** Querydown is a young project with real gaps. Say what it can't do where a reader would run into it. A setup page ends with a "What's missing" section when there's something to say.
- **Second person.** Speak to the reader as "you". Say what _you_ can do, rather than what "can be done" or what "the user" does.
- **Concise.** Prefer a short sentence and an example to a long explanation.
- **Plain words.** Explain a term the first time a page uses it, or link to where it's explained.

## How each kind of page is written

### Language and function reference (`language.md`, `functions.md`)

Organized by feature, with **one short section per feature**, so that each feature has a heading of its own in the table of contents. A section follows this pattern:

1. One or two sentences that say what the feature is.
1. An example, with its plain-English description.
1. The details: edge cases, rules, and limits, usually as a list. Enough for a full understanding.

When a section grows long, or needs several examples of different things, split it into sections (or subsections) with headings of their own. Don't number or reorder headings casually: other pages link to them.

### Setup pages (`setup/*.md`)

Written **imperatively**, as things to do. Headings are actions ("Compile a query", not "Compilation"), and steps go in ordered lists, one action per step, with the command or code to use. Reference material (tables of options, function signatures) comes after the steps.

Until Querydown is published to crates.io and npm, installation instructions pin a Git commit.

### The cheat sheet

Tables of syntax, each row linking to the section of `language.md` that explains it. When you add syntax to the language, add a row.

### The home page

It has to make sense to someone who knows nothing yet. Keep it honest about what Querydown is _not_. The `__TODO__` placeholders there are Sean's to fill in. Leave them.

## Examples

Examples are the heart of the docs, and **every one is tested**. A fenced code block whose language is `qd` is an example:

````md
> Find issues where the title contains "performance"

```qd
#issues title:"performance"
```
````

- **The blockquote directly before an example is its plain-English description.** The site shows the two together. Describe what the query finds ("Issues that…"), not the syntax it uses.
- **Every example uses the [sample schema](./sample-schema.md)**, the issue tracker. Don't invent tables or columns, even in passing. If the schema lacks something that an example needs, mention the hypothetical in prose (with inline code) rather than in an example.
- **Every example is a whole query**, which compiles by itself. If it needs a definition from an earlier example, repeat the definition.
- On the site, each example gets a button to copy it and one that shows the SQL it compiles to, with a link that opens it in the playground. You don't write any of that.

### Flags

Words after `qd` on the opening line of the block change what's expected of the example:

| Opening line | Meaning |
| -- | -- |
| ```` ```qd ```` | The example must compile, for every dialect. |
| ```` ```qd error ```` | The example must **fail** to compile. The site underlines the part of the code that the error is about, and shows the compiler's actual error message beneath it (after "This query fails to compile."), so don't write the message out, and don't mark the mistake with a comment. Give the example a description that makes clear the failure is the point, such as "Attempt to show…". |
| ```` ```qd fragment ```` | The code isn't a whole query, so it's only highlighted, and isn't tested. Use this rarely: prefer inline code for a piece of syntax. |
| ```` ```qd limit=50 offset=100 ```` | The example is compiled with those [compiler options](./language.md#limit-and-offset). |

### Stating the output

A code block _after_ an example can state what the example compiles to. It's then checked against the compiler's real output (ignoring whitespace), so it can't go stale:

| Opening line | Must match the example's… |
| -- | -- |
| ```` ```sql output ```` | SQL for PostgreSQL |
| ```` ```sql output duckdb ```` | SQL for DuckDB |
| ```` ```json output ```` | annotations. Only the keys that the block has (`columnAnnotations`, `queryAnnotation`) are compared. |

A plain ```` ```sql ```` block isn't checked. Use `output` whenever a block claims to be what an example compiles to.

### How examples are tested

- `cargo test` runs [`compiler/src/tests/docs_examples.rs`](../compiler/src/tests/docs_examples.rs), which finds every example in `README.md`, `docs/`, and `site/content/`, and compiles it for PostgreSQL and for DuckDB. It also compiles every `.qd` file in those places and in `site/src/`, each of which holds one whole query. That's how the query that the playground opens with ([`starting-query.qd`](../site/src/routes/playground/starting-query.qd)) is tested.
- `cargo test --features db-tests` also has real PostgreSQL and DuckDB databases plan each example's SQL.
- The site's build compiles each example too (to show its SQL, or its error), and fails if an example doesn't do what its flags say.

To show Markdown source that contains an example without it being tested, as this file does, put it in a fence of four backticks.

## Links

Cross-link generously. A reader who lands in the middle of a page should be able to reach the explanation of every term it uses.

- **Work links into the prose**, by wrapping the words that name the thing: "see the list of [all aggregate functions](./functions.md#aggregate-functions)". Avoid "click here" and bare URLs.
- **Link to another page by the relative path of its Markdown file**: `[ranges](./language.md#ranges)`, `[the CLI](./setup/cli.md)`. Such links work on GitHub as they are, and the site rewrites them to the page's URL.
- **Link to a section** with a fragment, which is the heading's text in lowercase, with spaces as hyphens and punctuation dropped (as on GitHub): `## Flagged strings` is `#flagged-strings`.
- **Link to any other file in the repository** by its relative path too. The site turns that into a link to the file on GitHub.
- **Link to a page that has no Markdown file** (the playground) by its absolute path: `[playground](/playground)`.

A link to a file or a heading that doesn't exist fails the site's build, with a message naming the file and the link. After the build, `npm run check:links` checks every internal link of the built site again.

## Formatting

- **One `#` heading per page**, which is the page's title. It should match the page's title in the navigation.
- **The paragraph after the title** says what the page is for. It's also the page's description in search engines and in `/llms.txt`.
- **No table of contents in the Markdown.** The site generates one from the headings.
- **No images yet.** The renderer doesn't support them. (The ER diagram is a component, drawn from the schema's JSON.)
- **Raw HTML is allowed**, and is how you give part of a page a class to [style](#styling-a-page). Use it sparingly, so that the file still reads well on GitHub.
- **Tables** for anything with two or more attributes per item. **Lists** for rules and details.
- **Code formatting** for anything that the reader might type or see in code: syntax, names of tables, columns, and functions, file names, commands.
- Give other code blocks their language (`sql`, `rust`, `ts`, `json`, `sh`, `toml`), so that they're highlighted. Use `text` for terminal output.
- Sentence case for headings ("Query anatomy", not "Query Anatomy").

### Components

A few things on the site aren't Markdown. A page asks for one with an HTML comment on a line of its own, which GitHub doesn't show:

| Marker | What appears |
| -- | -- |
| `<!-- er-diagram -->` | The entity-relationship diagram of the sample schema |
| `<!-- showcase -->` … `<!-- /showcase -->` | One `qd` block, without a description, shown beside the SQL it compiles to. The SQL is set small, to show how much of it the Querydown stands for. |
| `<!-- features -->` … `<!-- /features -->` | A heading, then a list of features, each item holding the feature's name (usually a link to the docs), a blockquote, and a `qd` block. It's shown as a box with the list of features beside the Querydown of the chosen one, which is followed by its description and a link to the docs. No SQL is shown. Here the description comes _after_ the code, so write it as a whole sentence about the query ("This query shows issues that…"). Paragraphs after the list are a note about all of the examples, shown at the foot of the box. |

### Styling a page

A page can have a stylesheet of its own: a CSS file beside its Markdown file, with the same name. The home page (`site/content/home.md`) is styled by [`site/content/home.css`](../site/content/home.css), for example, and a `docs/foo.css` would style `docs/foo.md`. The page includes it, and the dev server reloads the page when you save it.

To style part of a page, wrap it in an HTML element with a class, and leave a blank line inside each tag so that what's between them is still read as Markdown:

````md
<div class="features">

- [Intuitive joins](./language.md#referencing-single-related-records)
- [Math](./language.md#arithmetic)

</div>
````

Then style the class in the page's stylesheet, starting each selector with `.prose` (as in `.prose .features ul`) so that it outranks the site's own styles for Markdown, and using the [colors and other variables](../site/src/app.css) that the rest of the site uses, so that the page suits both themes. GitHub ignores the class, and shows the Markdown inside as usual.

The stylesheet applies to the whole site while its page is shown, so choose class names that the site's own styles don't use. Styles that more than one page needs belong in [`site/src/app.css`](../site/src/app.css) instead.

## Before you commit

1. Run `cargo test`, which tests every example.
1. Run `npm run build` in `site/`, which renders every page and checks every link.
1. Read your change on the site (`npm run dev`), in both a wide and a narrow window.
1. If you changed the language, check whether the [cheat sheet](./cheat-sheet.md), the [functions](./functions.md) page, and the home page's list of features need to follow.
1. If you changed these conventions, or how the site renders Markdown ([`site/src/lib/server/markdown.ts`](../site/src/lib/server/markdown.ts)), update this file.
