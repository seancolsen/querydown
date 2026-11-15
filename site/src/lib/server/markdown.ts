/**
 * Renders the site's Markdown content to HTML.
 *
 * This is ordinary GitHub-flavored Markdown, so that the files also read well on GitHub. The
 * conventions that the site adds on top of it are documented for authors in `docs/README.md`; keep
 * that file in step with this one.
 */

import GithubSlugger from 'github-slugger';
import type { Element, Root as HastRoot } from 'hast';
import type { Blockquote, Code, Heading, Html, Nodes, Parent, Root, RootContent } from 'mdast';
import { toString } from 'mdast-util-to-string';
import rehypeRaw from 'rehype-raw';
import rehypeStringify from 'rehype-stringify';
import remarkGfm from 'remark-gfm';
import remarkParse from 'remark-parse';
import remarkRehype from 'remark-rehype';
import type { Highlighter } from 'shiki';
import { unified } from 'unified';
import { visit } from 'unist-util-visit';

import type { RenderedMarkdown, SearchSection, Segment, Slide, TocItem } from '../content-types';
import { playgroundFragment } from '../playground-url';
import { escapeHtml, highlightCode, highlightQuerydown, loadHighlighter } from './highlight';
import { compileExample, loadQuerydown, type CompileOptions } from './querydown';

export interface RenderContext {
  /** The file being rendered, relative to the repository root. Used in error messages. */
  file: string;
  /**
   * Turns the URL of a Markdown link into the URL to use on the site, throwing if it's broken.
   * `ids` holds the ids of the headings on the page being rendered.
   */
  resolveLink: (url: string, ids: Set<string>) => string;
  /** The URL of the playground. */
  playgroundUrl: string;
}

const parser = unified().use(remarkParse).use(remarkGfm);

export function parseMarkdown(source: string): Root {
  return parser.parse(source);
}

/** The ids that a page's headings get, which match the anchors GitHub generates. */
export function headingIds(root: Root): Set<string> {
  const slugger = new GithubSlugger();
  const ids = new Set<string>();
  visit(root, 'heading', (heading) => {
    ids.add(slugger.slug(toString(heading)));
  });
  return ids;
}

// -------------------------------------------------------------------------------------------------
// Code blocks
// -------------------------------------------------------------------------------------------------

/** The flags that may follow `qd` in the info string of a fenced code block. */
interface ExampleFlags {
  /** The example is expected to fail to compile. */
  error: boolean;
  /** The code isn't a whole query, so it's only highlighted. */
  fragment: boolean;
  options: CompileOptions;
}

function parseExampleFlags(meta: string | null | undefined, context: RenderContext): ExampleFlags {
  const flags: ExampleFlags = { error: false, fragment: false, options: {} };
  for (const word of (meta ?? '').split(/\s+/).filter(Boolean)) {
    const option = /^(limit|offset)=(\d+)$/.exec(word);
    if (word === 'error') flags.error = true;
    else if (word === 'fragment') flags.fragment = true;
    else if (option) flags.options[option[1] as 'limit' | 'offset'] = Number(option[2]);
    else throw new Error(`${context.file}: unknown flag \`${word}\` on a \`qd\` code block.`);
  }
  return flags;
}

const COPY_BUTTON =
  '<button type="button" class="code-action" data-copy aria-label="Copy" title="Copy">' +
  '<svg viewBox="0 0 16 16" aria-hidden="true"><path d="M5.5 1.5h7a1 1 0 0 1 1 1v8.5M3.5 4.5h7a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1h-7a1 1 0 0 1-1-1v-8a1 1 0 0 1 1-1z"/></svg>' +
  '</button>';

const PLAYGROUND_ICON =
  '<svg viewBox="0 0 16 16" aria-hidden="true"><path d="M5 3.5v9l7.5-4.5z"/></svg>';

interface CompiledExample {
  /** The highlighted source. */
  code: string;
  /** A link that opens the example in the playground, unless it's a fragment. */
  playground: string | null;
  /** What the example compiles to, unless it's a fragment. */
  sql: string | null;
  /** Why the example doesn't compile, if it's expected not to. */
  error: { message: string; hint: string | null } | null;
}

/** Highlights and compiles an example, checking that it compiles (or fails to) as it declares. */
function compileCodeBlock(node: Code, context: RenderContext): CompiledExample {
  const flags = parseExampleFlags(node.meta, context);
  const code = highlightQuerydown(node.value);
  if (flags.fragment) return { code, playground: null, sql: null, error: null };

  const fragment = playgroundFragment({ query: node.value, ...flags.options });
  const playground = `${context.playgroundUrl}#${fragment}`;
  const outcome = compileExample(node.value, flags.options);
  const where = `${context.file}, line ${node.position?.start.line}`;
  if (outcome.ok && flags.error) {
    throw new Error(`${where}: this example is marked \`error\`, but it compiles.`);
  }
  if (!outcome.ok && !flags.error) {
    throw new Error(`${where}: this example doesn't compile: ${outcome.message}`);
  }
  return outcome.ok
    ? { code, playground, sql: outcome.sql, error: null }
    : { code, playground, sql: null, error: outcome };
}

/** Whether the code block after an example shows the example's output, as checked by the tests. */
function isOutputBlock(node: RootContent | undefined): boolean {
  return node?.type === 'code' && /(^|\s)output(\s|$)/.test(node.meta ?? '');
}

/** A compiler message as HTML. The compiler quotes code with backticks, as Markdown does. */
function messageHtml(message: string): string {
  return escapeHtml(message).replace(/`([^`]+)`/g, '<code>$1</code>');
}

function exampleHtml(example: CompiledExample, showSql: boolean): string {
  const playground = example.playground
    ? `<a class="code-action" href="${escapeHtml(example.playground)}" target="_blank" rel="noopener" aria-label="Open in the playground" title="Open in the playground">${PLAYGROUND_ICON}</a>`
    : '';
  let html =
    `<div class="code-block">` +
    `<pre class="qd"><code>${example.code}</code></pre>` +
    `<div class="code-actions">${COPY_BUTTON}${playground}</div>` +
    `</div>`;
  if (example.error) {
    const hint = example.error.hint
      ? ` <span class="example-hint">${messageHtml(example.error.hint)}</span>`
      : '';
    html += `<p class="example-error"><strong>Error:</strong> ${messageHtml(example.error.message)}${hint}</p>`;
  } else if (example.sql && showSql) {
    html +=
      `<details class="example-sql"><summary>SQL</summary>` +
      `<pre><code>${escapeHtml(example.sql)}</code></pre></details>`;
  }
  return html;
}

function plainCodeHtml(node: Code, shiki: Highlighter): string {
  return `<div class="code-block">${highlightCode(shiki, node.value, node.lang ?? null)}<div class="code-actions">${COPY_BUTTON}</div></div>`;
}

const html = (value: string): Html => ({ type: 'html', value });

/**
 * Replaces the code blocks within `parent` (at any depth) with highlighted HTML.
 *
 * A `qd` block is an example. A blockquote directly before one is its plain-English description,
 * and the two are wrapped up together.
 */
function transformCodeBlocks(parent: Parent, context: RenderContext, shiki: Highlighter): void {
  const output: RootContent[] = [];
  const children = parent.children as RootContent[];
  children.forEach((node, index) => {
    if (node.type !== 'code') {
      if ('children' in node) transformCodeBlocks(node, context, shiki);
      output.push(node);
      return;
    }
    if (node.lang !== 'qd') {
      output.push(html(plainCodeHtml(node, shiki)));
      return;
    }
    const example = compileCodeBlock(node, context);
    const body = exampleHtml(example, !isOutputBlock(children[index + 1]));
    const classes = example.error ? 'example example-fails' : 'example';
    const previous = output.at(-1);
    if (previous?.type === 'blockquote') {
      output.pop();
      output.push(
        html(`<div class="${classes}"><div class="example-caption">`),
        ...previous.children,
        html(`</div>${body}</div>`),
      );
    } else {
      output.push(html(`<div class="${classes}">${body}</div>`));
    }
  });
  parent.children = output as Parent['children'];
}

// -------------------------------------------------------------------------------------------------
// HTML
// -------------------------------------------------------------------------------------------------

/** Wraps tables so that wide ones scroll by themselves, and adds a link to each heading. */
function rehypePolish() {
  return (tree: HastRoot) => {
    visit(tree, 'element', (element, index, parent) => {
      if (element.tagName === 'table' && parent && index !== undefined) {
        const wrapper: Element = {
          type: 'element',
          tagName: 'div',
          properties: { className: ['table-wrap'] },
          children: [element],
        };
        parent.children[index] = wrapper;
        return;
      }
      const id = element.properties.id;
      if (/^h[2-4]$/.test(element.tagName) && typeof id === 'string') {
        element.children.push({
          type: 'element',
          tagName: 'a',
          properties: {
            className: ['heading-anchor'],
            href: `#${id}`,
            ariaLabel: 'Link to this section',
          },
          children: [{ type: 'text', value: '#' }],
        });
      }
    });
  };
}

const toHtml = unified()
  .use(remarkRehype, { allowDangerousHtml: true })
  .use(rehypeRaw)
  .use(rehypePolish)
  .use(rehypeStringify);

function renderNodes(nodes: RootContent[]): string {
  const root: Root = { type: 'root', children: nodes };
  return toHtml.stringify(toHtml.runSync(root));
}

// -------------------------------------------------------------------------------------------------
// Components
// -------------------------------------------------------------------------------------------------

const marker = (node: RootContent): string | null =>
  node.type === 'html'
    ? (/^<!--\s*(\/?[a-z-]+)\s*-->$/.exec(node.value.trim())?.[1] ?? null)
    : null;

/**
 * The examples between `<!-- slideshow -->` and `<!-- /slideshow -->`, as slides, and the note
 * (paragraphs after the last example) about all of them.
 */
function slideshow(
  nodes: RootContent[],
  context: RenderContext,
  shiki: Highlighter,
): { slides: Slide[]; note: string | null } {
  const result: Slide[] = [];
  const note: RootContent[] = [];
  let caption: Blockquote | undefined;
  for (const node of nodes) {
    if (node.type === 'paragraph' && !caption) {
      note.push(node);
    } else if (note.length > 0) {
      throw new Error(`${context.file}: a slideshow's note must come after its last example.`);
    } else if (node.type === 'blockquote') {
      caption = node;
    } else if (node.type === 'code' && node.lang === 'qd' && caption) {
      const example = compileCodeBlock(node, context);
      if (!example.sql || !example.playground) {
        throw new Error(`${context.file}: a slide must be an example that compiles.`);
      }
      result.push({
        caption: renderNodes(caption.children),
        code: example.code,
        sql: highlightCode(shiki, example.sql, 'sql'),
        playground: example.playground,
      });
      caption = undefined;
    } else {
      throw new Error(
        `${context.file}: a slideshow may only contain examples, each a blockquote followed by a \`qd\` code block, and then a note.`,
      );
    }
  }
  if (caption) throw new Error(`${context.file}: a slideshow's last description has no example.`);
  return { slides: result, note: note.length > 0 ? renderNodes(note) : null };
}

// -------------------------------------------------------------------------------------------------
// The search index
// -------------------------------------------------------------------------------------------------

/** The text within a block, with a space after each paragraph, table cell, and so on. */
function blockText(node: RootContent): string {
  let text = '';
  visit(node, (inner: Nodes) => {
    if (inner.type === 'paragraph' || inner.type === 'tableCell' || inner.type === 'code') {
      text += `${toString(inner)} `;
    }
  });
  return text;
}

function searchSections(root: Root): SearchSection[] {
  const slugger = new GithubSlugger();
  const sections: SearchSection[] = [];
  /** The headings that the current section is nested within, by level. */
  const trail: (string | undefined)[] = [];
  let current: SearchSection = { id: null, heading: '', parents: [], text: '' };
  for (const node of root.children) {
    if (node.type === 'heading') {
      sections.push(current);
      const heading = toString(node);
      trail.length = node.depth;
      trail[node.depth - 1] = heading;
      current = {
        id: slugger.slug(heading),
        heading,
        parents: trail.slice(0, node.depth - 1).filter((h) => h !== undefined),
        text: '',
      };
    } else {
      current.text += blockText(node);
    }
  }
  sections.push(current);
  return sections.filter((section) => section.heading || section.text.trim());
}

// -------------------------------------------------------------------------------------------------
// Pages
// -------------------------------------------------------------------------------------------------

function firstParagraph(root: Root): string {
  const paragraph = root.children.find((node) => node.type === 'paragraph');
  return paragraph ? toString(paragraph).replace(/\s+/g, ' ').trim() : '';
}

export async function renderMarkdown(
  source: string,
  context: RenderContext,
): Promise<RenderedMarkdown> {
  const [shiki] = await Promise.all([loadHighlighter(), loadQuerydown()]);
  const root = parseMarkdown(source);
  const sections = searchSections(root);
  const description = firstParagraph(root);

  // Headings: give each an id, and collect the table of contents.
  const slugger = new GithubSlugger();
  const toc: TocItem[] = [];
  const ids = new Set<string>();
  let title = '';
  visit(root, 'heading', (heading: Heading) => {
    const text = toString(heading);
    const id = slugger.slug(text);
    ids.add(id);
    heading.data = { ...heading.data, hProperties: { id } };
    if (heading.depth === 1) title ||= text;
    else if (heading.depth <= 4) toc.push({ id, text, depth: heading.depth });
  });
  if (!title) throw new Error(`${context.file}: the page needs a level-1 heading for its title.`);

  // Links.
  visit(root, (node: Nodes) => {
    if (node.type === 'link' || node.type === 'definition') {
      node.url = context.resolveLink(node.url, ids);
    } else if (node.type === 'image') {
      throw new Error(`${context.file}: images aren't supported in content yet (${node.url}).`);
    }
  });

  // Split the page into segments at the component markers, rendering what's between them.
  const segments: Segment[] = [];
  let pending: RootContent[] = [];
  const flush = () => {
    if (pending.length === 0) return;
    const part: Root = { type: 'root', children: pending };
    transformCodeBlocks(part, context, shiki);
    segments.push({ type: 'html', html: renderNodes(part.children) });
    pending = [];
  };
  for (let index = 0; index < root.children.length; index++) {
    const node = root.children[index];
    const name = marker(node);
    if (name === 'er-diagram') {
      flush();
      segments.push({ type: 'er-diagram' });
    } else if (name === 'slideshow') {
      const end = root.children.findIndex((n, i) => i > index && marker(n) === '/slideshow');
      if (end === -1) throw new Error(`${context.file}: \`<!-- slideshow -->\` is never closed.`);
      flush();
      const contents = root.children.slice(index + 1, end);
      segments.push({ type: 'slideshow', ...slideshow(contents, context, shiki) });
      index = end;
    } else {
      pending.push(node);
    }
  }
  flush();

  return { title, description, segments, toc, sections };
}
