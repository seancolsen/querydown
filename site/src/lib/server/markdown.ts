/**
 * Renders the site's Markdown content, checking its examples as it goes.
 *
 * The result is mostly HTML, but each code block becomes the data for a Svelte component that shows
 * it (see `Blocks.svelte`), and so do the components that a page asks for with a marker comment.
 *
 * This is ordinary GitHub-flavored Markdown, so that the files also read well on GitHub. The
 * conventions that the site adds on top of it are documented for authors in `docs/README.md`; keep
 * that file in step with this one.
 */

import GithubSlugger from 'github-slugger';
import type {
  Element,
  Nodes as HastNodes,
  Root as HastRoot,
  RootContent as HastRootContent,
} from 'hast';
import type {
  Blockquote,
  Code,
  Heading,
  ListItem,
  Node as MdastNode,
  Nodes,
  Parent,
  Root,
  RootContent,
} from 'mdast';
import { find, html as htmlSchema } from 'property-information';
import { toString } from 'mdast-util-to-string';
import rehypeRaw from 'rehype-raw';
import rehypeStringify from 'rehype-stringify';
import remarkGfm from 'remark-gfm';
import remarkParse from 'remark-parse';
import remarkRehype from 'remark-rehype';
import type { Highlighter } from 'shiki';
import { unified } from 'unified';
import { visit } from 'unist-util-visit';

import type {
  Block,
  Feature,
  RenderedMarkdown,
  SearchSection,
  Segment,
  Slide,
  TocItem,
} from '../content-types';
import { playgroundFragment } from '../playground-url';
import { highlightCode, highlightQuerydown, loadHighlighter } from './highlight';
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

/** The element that a placeholder becomes in HTML, until `splitBlocks` swaps it for its block. */
const PLACEHOLDER_TAG = 'x-block';

/** Stands in for a code block in Markdown, so that the page shows a component there. */
interface Placeholder extends MdastNode {
  type: 'placeholder';
}

declare module 'mdast' {
  interface BlockContentMap {
    placeholder: Placeholder;
  }
  interface RootContentMap {
    placeholder: Placeholder;
  }
}

/** Adds a block to `blocks`, returning a placeholder that refers to it by its position. */
function placeholder(block: Block, blocks: Block[]): Placeholder {
  blocks.push(block);
  const properties = { dataBlock: blocks.length - 1 };
  return { type: 'placeholder', data: { hName: PLACEHOLDER_TAG, hProperties: properties } };
}

/** Whether a placeholder is within `node`, at any depth. */
function holdsPlaceholder(node: Nodes): boolean {
  return (
    'children' in node && node.children.some((c) => c.type === 'placeholder' || holdsPlaceholder(c))
  );
}

/**
 * Replaces the code blocks within `parent` (at any depth) with placeholders for the components
 * that show them, adding those to `blocks`.
 *
 * A `qd` block is an example. A blockquote directly before one is its plain-English description,
 * which the example shows above its code.
 */
function transformCodeBlocks(
  parent: Parent,
  context: RenderContext,
  shiki: Highlighter,
  blocks: Block[],
): void {
  const output: RootContent[] = [];
  const children = parent.children as RootContent[];
  children.forEach((node, index) => {
    if (node.type !== 'code') {
      if ('children' in node) transformCodeBlocks(node, context, shiki, blocks);
      output.push(node);
      return;
    }
    if (node.lang !== 'qd') {
      const html = highlightCode(shiki, node.value, node.lang ?? null);
      output.push(placeholder({ type: 'code', source: node.value, html }, blocks));
      return;
    }
    const example = compileCodeBlock(node, context);
    const previous = output.at(-1);
    let caption: string | null = null;
    if (previous?.type === 'blockquote') {
      if (holdsPlaceholder(previous)) {
        const where = `${context.file}, line ${node.position?.start.line}`;
        throw new Error(`${where}: an example's description can't hold a code block.`);
      }
      output.pop();
      caption = renderNodes(previous.children);
    }
    const showSql = !isOutputBlock(children[index + 1]);
    const block: Block = {
      type: 'example',
      caption,
      source: node.value,
      code: example.code,
      playground: example.playground,
      sql: showSql ? example.sql : null,
      error: example.error,
    };
    output.push(placeholder(block, blocks));
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

/** Renders Markdown as HTML. */
function renderNodes(nodes: RootContent[]): string {
  const root: Root = { type: 'root', children: nodes };
  return toHtml.stringify(toHtml.runSync(root));
}

/** Renders Markdown whose code blocks are placeholders for `components`, as blocks. */
function renderBlocks(nodes: RootContent[], components: Block[]): Block[] {
  const root: Root = { type: 'root', children: nodes };
  return splitBlocks(toHtml.runSync(root).children, components);
}

function hasPlaceholder(node: HastNodes): boolean {
  if (node.type === 'element' && node.tagName === PLACEHOLDER_TAG) return true;
  return 'children' in node && node.children.some(hasPlaceholder);
}

/**
 * Turns HTML into blocks, swapping each placeholder for its block from `components`. Any element
 * that holds a placeholder becomes a block of its own, and the HTML around them becomes HTML blocks.
 */
function splitBlocks(nodes: HastRootContent[], components: Block[]): Block[] {
  const blocks: Block[] = [];
  let pending: HastRootContent[] = [];
  const flush = () => {
    const html = toHtml.stringify({ type: 'root', children: pending });
    if (html.trim()) blocks.push({ type: 'html', html });
    pending = [];
  };
  for (const node of nodes) {
    if (node.type !== 'element' || !hasPlaceholder(node)) {
      pending.push(node);
      continue;
    }
    flush();
    blocks.push(
      node.tagName === PLACEHOLDER_TAG
        ? components[Number(node.properties.dataBlock)]
        : {
            type: 'element',
            tag: node.tagName,
            attributes: attributes(node),
            children: splitBlocks(node.children, components),
          },
    );
  }
  flush();
  return blocks;
}

/** An element's properties, as HTML attributes. */
function attributes(element: Element): Record<string, string> {
  const result: Record<string, string> = {};
  for (const [key, value] of Object.entries(element.properties)) {
    if (value == null || value === false) continue;
    const info = find(htmlSchema, key);
    result[info.attribute] =
      value === true
        ? ''
        : Array.isArray(value)
          ? value.join(info.commaSeparated ? ', ' : ' ')
          : String(value);
  }
  return result;
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

/** One item of the list within `<!-- features -->`: a feature's name, a description, an example. */
function feature(item: ListItem, context: RenderContext): Feature {
  const [title, caption, code, ...rest] = item.children;
  const shapeError = () =>
    new Error(
      `${context.file}: each feature must be a list item holding its name (which may be a link), then a blockquote describing an example, then the example as a \`qd\` code block.`,
    );
  if (
    title?.type !== 'paragraph' ||
    caption?.type !== 'blockquote' ||
    code?.type !== 'code' ||
    code.lang !== 'qd' ||
    rest.length > 0
  ) {
    throw shapeError();
  }
  const link = title.children.length === 1 ? title.children[0] : null;
  const example = compileCodeBlock(code, context);
  if (!example.playground || example.error) {
    throw new Error(`${context.file}: a feature's example must be one that compiles.`);
  }
  return {
    name: toString(title),
    href: link?.type === 'link' ? link.url : null,
    caption: renderNodes(caption.children),
    source: code.value,
    code: example.code,
    playground: example.playground,
  };
}

/**
 * The contents of `<!-- features -->` … `<!-- /features -->`: a heading, then a list of features,
 * each with an example.
 */
function features(
  nodes: RootContent[],
  context: RenderContext,
): { heading: string | null; features: Feature[] } {
  const [first, ...others] = nodes;
  const heading = first?.type === 'heading' ? first : null;
  const rest = heading ? others : nodes;
  const list = rest[0];
  if (rest.length !== 1 || list.type !== 'list') {
    throw new Error(`${context.file}: \`<!-- features -->\` must hold a heading and one list.`);
  }
  return {
    heading: heading ? renderNodes([heading]) : null,
    features: list.children.map((item) => feature(item, context)),
  };
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
    const components: Block[] = [];
    transformCodeBlocks(part, context, shiki, components);
    segments.push({ type: 'prose', blocks: renderBlocks(part.children, components) });
    pending = [];
  };
  for (let index = 0; index < root.children.length; index++) {
    const node = root.children[index];
    const name = marker(node);
    if (name === 'er-diagram') {
      flush();
      segments.push({ type: 'er-diagram' });
    } else if (name === 'features') {
      const end = root.children.findIndex((n, i) => i > index && marker(n) === '/features');
      if (end === -1) throw new Error(`${context.file}: \`<!-- features -->\` is never closed.`);
      flush();
      const contents = root.children.slice(index + 1, end);
      segments.push({ type: 'features', ...features(contents, context) });
      index = end;
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
