<!--
  A page of documentation: the content, and links to its neighbors. (Its table of contents is in the
  navigation: see NavTree.)
-->
<script lang="ts">
  import type { Page } from '#lib/content-types.ts';
  import { contains, isGroup, pagesOf, type NavNode } from '#lib/nav.ts';
  import { githubFileUrl, SITE_NAME } from '#lib/site.ts';

  import Content from './Content.svelte';
  import Footer from './Footer.svelte';

  interface Props {
    page: Omit<Page, 'sections'>;
    nav: NavNode[];
  }

  let { page, nav }: Props = $props();

  /** The pages before and after this one, within its top-level section of the navigation. */
  const neighbors = $derived.by(() => {
    const section = nav.find((node) => isGroup(node) && contains(node, page.url));
    const pages = section ? pagesOf(section) : [];
    const index = pages.findIndex((p) => p.href === page.url);
    return { previous: pages[index - 1], next: pages[index + 1] };
  });
</script>

<svelte:head>
  <title>{page.title} · {SITE_NAME}</title>
  <meta name="description" content={page.description} />
</svelte:head>

<main>
  <Content segments={page.segments} styles={page.styles} />

  <nav class="pager" aria-label="Neighboring pages">
    {#if neighbors.previous}
      <a class="previous" href={neighbors.previous.href}>
        <span>Previous</span>
        {neighbors.previous.title}
      </a>
    {/if}
    {#if neighbors.next}
      <a class="next" href={neighbors.next.href}>
        <span>Next</span>
        {neighbors.next.title}
      </a>
    {/if}
  </nav>

  <p class="source">
    <a href={githubFileUrl(page.file)}>Edit this page on GitHub</a>
  </p>
  <Footer />
</main>

<style>
  main {
    min-width: 0;
    padding: 2rem 0 0;
  }

  .pager {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
    margin-top: 3.5rem;
  }

  .pager a {
    display: grid;
    gap: 0.1rem;
    padding: 0.7rem 1rem;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    font-weight: 600;
  }

  .pager a:hover {
    border-color: var(--accent);
    text-decoration: none;
  }

  .pager span {
    color: var(--text-muted);
    font-size: 0.8125rem;
    font-weight: 400;
  }

  .pager .next {
    grid-column: 2;
    text-align: right;
  }

  .source {
    margin: 1.5rem 0 0;
    font-size: 0.875rem;
  }

  .source + :global(footer) {
    margin-top: 1.5rem;
  }
</style>
