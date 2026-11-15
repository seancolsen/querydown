<!--
  The site's navigation, as a tree. A group is a collapsible label, and only pages are links. Only
  the groups that hold the current page start open, and moving to another page opens the groups
  that hold it and closes the ones that don't.

  The current page's table of contents (its headings) is listed beneath the page, with the section
  being read marked, and with the subsections of the other sections collapsed.
-->
<script lang="ts">
  import { tick } from 'svelte';

  import { afterNavigate, beforeNavigate } from '$app/navigation';
  import { page } from '$app/state';

  import { contains, isGroup, type NavNode } from '#lib/nav.ts';
  import { follow, reading } from '#lib/reading.svelte.ts';

  let { nodes }: { nodes: NavNode[] } = $props();

  const toc = $derived(page.data.toc ?? []);

  /** The index of each heading's parent: the closest heading before it that's shallower. */
  const parents = $derived(
    toc.map((item, index) => {
      for (let i = index - 1; i >= 0; i--) if (toc[i].depth < item.depth) return i;
      return null;
    }),
  );

  /**
   * The headings to list: the top-level sections, and the subsections only of the sections that the
   * reader is within. The rest are collapsed.
   */
  const sections = $derived.by(() => {
    const within = new Set<number>();
    const index = toc.findIndex((item) => item.id === reading.section);
    for (let i = index < 0 ? null : index; i !== null; i = parents[i]) within.add(i);
    return toc.filter((_, index) => {
      const parent = parents[index];
      return parent === null || within.has(parent);
    });
  });

  let tree = $state<HTMLElement>();

  /** The element that the tree scrolls within (the sidebar, or the drawer), while it's shown. */
  function scroller(): HTMLElement | null {
    if (!tree || tree.offsetParent === null) return null;
    for (let element = tree.parentElement; element; element = element.parentElement) {
      if (/auto|scroll/.test(getComputedStyle(element).overflowY)) return element;
    }
    return null;
  }

  /**
   * Notes where a link is, and returns a function that scrolls the tree to put it back there, for
   * once the tree has changed around it. (Following a link opens and closes the lists above it,
   * which would otherwise move the link from under the pointer.)
   */
  function pin(link: Element): () => void {
    const before = link.getBoundingClientRect().top;
    return () => {
      const within = scroller();
      if (within && link.isConnected) within.scrollTop += link.getBoundingClientRect().top - before;
    };
  }

  /** Scrolls the tree, if need be, to show the section being read (or else the current page). */
  export function reveal() {
    const within = scroller();
    const link =
      tree?.querySelector('[aria-current="location"]') ??
      tree?.querySelector('[aria-current="page"]');
    if (!within || !link) return;
    const top = link.getBoundingClientRect().top - within.getBoundingClientRect().top;
    if (top < 16 || top > within.clientHeight - 48) {
      within.scrollTop += top - within.clientHeight / 2;
    }
  }

  async function onSectionClick(event: MouseEvent, id: string) {
    const restore = pin(event.currentTarget as Element);
    follow(id);
    await tick();
    restore();
  }

  /** Puts back the link that the reader followed to another page, once that page is shown. */
  let restore: (() => void) | null = null;

  beforeNavigate((navigation) => {
    const link = navigation.type === 'link' && (navigation.event.target as Element).closest('a');
    restore = link && tree?.contains(link) ? pin(link) : null;
  });

  afterNavigate(() => {
    if (restore) restore();
    else reveal();
    restore = null;
  });

  $effect(() => {
    // Keep the section being read in view, as the reader scrolls through the page.
    void reading.section;
    if (!reading.followed) reveal();
  });
</script>

{#snippet list(nodes: NavNode[], nested: boolean)}
  <ul class:nested>
    {#each nodes as node (isGroup(node) ? node.path : node.href)}
      <li>
        {#if isGroup(node)}
          <details open={contains(node, page.url.pathname)}>
            <summary>{node.title}</summary>
            {@render list(node.children, true)}
          </details>
        {:else}
          {@const current = page.url.pathname === node.href}
          <a href={node.href} aria-current={current ? 'page' : undefined}>{node.title}</a>
          {#if current && sections.length > 0}
            <ul class="nested sections" aria-label="On this page">
              {#each sections as item (item.id)}
                <li>
                  <a
                    href="#{item.id}"
                    style:--depth={item.depth - 2}
                    aria-current={reading.section === item.id ? 'location' : undefined}
                    onclick={(event) => onSectionClick(event, item.id)}
                  >
                    {item.text}
                  </a>
                </li>
              {/each}
            </ul>
          {/if}
        {/if}
      </li>
    {/each}
  </ul>
{/snippet}

<div bind:this={tree}>
  {@render list(nodes, false)}
</div>

<style>
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  ul.nested {
    margin-left: 0.6rem;
    padding-left: 0.5rem;
    border-left: 1px solid var(--border);
  }

  a,
  summary {
    display: block;
    padding: 0.3rem 0.6rem;
    border-radius: var(--radius);
    color: var(--text-muted);
    line-height: 1.35;
  }

  a:hover,
  summary:hover {
    background: var(--bg-inset);
    color: var(--text);
    text-decoration: none;
  }

  /* The current page is a filled block. */
  a[aria-current='page'] {
    background: var(--accent-subtle);
    color: var(--accent);
    font-weight: 600;
  }

  summary {
    display: flex;
    align-items: center;
    justify-content: space-between;
    color: var(--text);
    font-weight: 600;
    cursor: pointer;
    list-style: none;
    user-select: none;
  }

  summary::-webkit-details-marker {
    display: none;
  }

  summary::after {
    content: '';
    width: 0.4rem;
    height: 0.4rem;
    margin-right: 0.2rem;
    border: solid var(--text-muted);
    border-width: 0 1.5px 1.5px 0;
    transform: rotate(-45deg);
    transition: transform 0.1s;
  }

  details[open] > summary::after {
    transform: rotate(45deg);
  }

  li {
    margin: 1px 0;
  }

  /* The current page's sections are smaller than the pages, and aren't filled when hovered over. */
  ul.sections {
    margin-block: 0.25rem 0.5rem;
    padding-left: 0;
    font-size: 0.875rem;
  }

  .sections li {
    margin: 0;
  }

  .sections a {
    /* Over the list's line, which the section being read marks. */
    margin-left: -1px;
    padding: 0.2rem 0.6rem;
    padding-left: calc(0.6rem + var(--depth) * 0.8rem);
    border-left: 2px solid transparent;
    border-radius: 0;
  }

  .sections a:hover {
    background: none;
  }

  /* The section being read is a mark on the line beside it, with no fill. */
  .sections a[aria-current='location'] {
    border-left-color: var(--accent);
    color: var(--accent);
  }
</style>
