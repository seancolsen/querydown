/**
 * The site's navigation tree.
 *
 * Navigation is a strict tree: an item either is a page (it has an `href`) or has children, never
 * both. The URL of a group (e.g. `/docs/setup`) redirects to its first page.
 *
 * The docs part of the tree is defined here, and is the source of truth for which Markdown files
 * under `docs/` become pages: a page listed here must have a file, and a file must be listed here.
 * The releases are discovered from `site/content/releases`.
 */

export interface NavPage {
  title: string;
  href: string;
}

export interface NavGroup {
  title: string;
  /** The URL prefix that the group's pages share. Visiting it redirects to the first page. */
  path: string;
  children: NavNode[];
}

export type NavNode = NavPage | NavGroup;

export function isGroup(node: NavNode): node is NavGroup {
  return 'children' in node;
}

const page = (title: string, href: string): NavPage => ({ title, href });

export const DOCS: NavGroup = {
  title: 'Docs',
  path: '/docs',
  children: [
    {
      title: 'Setup',
      path: '/docs/setup',
      children: [
        page('Installation', '/docs/setup/installation'),
        page('CLI', '/docs/setup/cli'),
        page('Rust', '/docs/setup/rust'),
        page('JavaScript', '/docs/setup/javascript'),
        page('Introspection', '/docs/setup/introspection'),
        page('CodeMirror', '/docs/setup/codemirror'),
      ],
    },
    page('Language', '/docs/language'),
    page('Functions', '/docs/functions'),
    page('Errors', '/docs/errors'),
    page('Cheat sheet', '/docs/cheat-sheet'),
    page('Sample schema', '/docs/sample-schema'),
  ],
};

export const HOME = page('Home', '/');
export const PLAYGROUND = page('Playground', '/playground');
export const RELEASES_PATH = '/releases';

/** Builds the whole tree, given the release pages (newest first). */
export function buildNav(releases: NavPage[]): NavNode[] {
  return [HOME, DOCS, { title: 'Releases', path: RELEASES_PATH, children: releases }, PLAYGROUND];
}

/** Every page within a node, in order. */
export function pagesOf(node: NavNode): NavPage[] {
  return isGroup(node) ? node.children.flatMap(pagesOf) : [node];
}

/** Every group within a node (including the node itself), in order. */
export function groupsOf(node: NavNode): NavGroup[] {
  return isGroup(node) ? [node, ...node.children.flatMap(groupsOf)] : [];
}

/** The first page within a node, which is where a group's URL leads. */
export function firstPage(node: NavNode): NavPage | undefined {
  return pagesOf(node)[0];
}

/** Whether `pathname` is the node's page, or a page within the node's group. */
export function contains(node: NavNode, pathname: string): boolean {
  return pagesOf(node).some((p) => p.href === pathname);
}

/** The titles of the groups leading to the page at `pathname`, outermost first. */
export function breadcrumbs(nodes: NavNode[], pathname: string): string[] {
  for (const node of nodes) {
    if (!isGroup(node)) continue;
    if (contains(node, pathname)) return [node.title, ...breadcrumbs(node.children, pathname)];
  }
  return [];
}
