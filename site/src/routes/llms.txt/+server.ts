import { breadcrumbs, buildNav } from '#lib/nav.ts';
import { allPages, releasePages } from '#lib/server/content.ts';
import { GITHUB_URL, SITE_NAME } from '#lib/site.ts';

import type { RequestHandler } from './$types';

export const prerender = true;

/**
 * An index of the site for AI agents, in the llms.txt format (https://llmstxt.org): what Querydown
 * is, and a link to each page with a one-line description.
 */
export const GET: RequestHandler = async ({ url }) => {
  const nav = buildNav(releasePages());
  const pages = await allPages();
  const home = pages.find((page) => page.url === '/');
  const sections = new Map<string, string[]>();
  for (const page of pages) {
    if (page.url === '/') continue;
    const [section = 'Pages', ...groups] = breadcrumbs(nav, page.url);
    const title = [...groups, page.title].join(': ');
    const line = `- [${title}](${url.origin}${page.url}): ${page.description}`;
    sections.set(section, [...(sections.get(section) ?? []), line]);
  }
  const body = [
    `# ${SITE_NAME}`,
    '',
    `> ${home?.description ?? ''}`,
    '',
    `The whole documentation is also available as one Markdown file, at ${url.origin}/llms-full.txt.`,
    `The source code is at ${GITHUB_URL}.`,
    '',
    ...[...sections].flatMap(([section, lines]) => [`## ${section}`, '', ...lines, '']),
  ].join('\n');
  return new Response(body, { headers: { 'content-type': 'text/plain; charset=utf-8' } });
};
