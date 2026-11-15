<!--
  The headings of the current page. On a wide screen the `side` variant sits beside the content,
  marks the section being read, and collapses the sections that the reader isn't within. Otherwise
  the `inline` variant is a collapsed list above the content. A page renders both, and CSS shows
  the one that suits the screen.
-->
<script lang="ts">
  import { tick } from 'svelte';

  import type { TocItem } from '#lib/content-types.ts';

  interface Props {
    items: TocItem[];
    /** `inline` is a collapsed list for narrow screens, and `side` a sidebar for wide ones. */
    variant: 'inline' | 'side';
  }

  let { items, variant }: Props = $props();

  let active = $state<string | null>(null);

  /**
   * The heading whose link was last clicked, which stays marked until the reader scrolls by
   * themselves. (A heading near the end of a page can't reach the top of the window, so the scroll
   * position alone wouldn't mark it.)
   */
  let clicked: string | null = null;

  $effect(() => {
    if (variant !== 'side') return;
    const headings = items
      .map((item) => document.getElementById(item.id))
      .filter((element) => element !== null);
    if (headings.length === 0) return;

    // The section being read is the last one whose heading has passed the top of the viewport.
    const update = () => {
      if (clicked) return;
      const threshold = 96;
      let current: string | null = null;
      for (const heading of headings) {
        if (heading.getBoundingClientRect().top > threshold) break;
        current = heading.id;
      }
      active = current;
    };
    const release = () => {
      if (clicked) {
        clicked = null;
        update();
      }
    };
    update();
    const userInput = ['wheel', 'touchstart', 'keydown', 'pointerdown'] as const;
    window.addEventListener('scroll', update, { passive: true });
    for (const type of userInput) window.addEventListener(type, release, { passive: true });
    return () => {
      window.removeEventListener('scroll', update);
      for (const type of userInput) window.removeEventListener(type, release);
    };
  });

  /** The index of each item's parent: the closest item before it with a shallower heading. */
  const parents = $derived(
    items.map((item, index) => {
      for (let i = index - 1; i >= 0; i--) if (items[i].depth < item.depth) return i;
      return null;
    }),
  );

  /**
   * The items to list. The side variant lists the top-level sections, and the subsections only of
   * the sections that the reader is within: the rest are collapsed.
   */
  const shown = $derived.by(() => {
    if (variant !== 'side') return items;
    const within = new Set<number>();
    const index = items.findIndex((item) => item.id === active);
    for (let i = index < 0 ? null : index; i !== null; i = parents[i]) within.add(i);
    return items.filter((_, index) => {
      const parent = parents[index];
      return parent === null || within.has(parent);
    });
  });

  let list = $state<HTMLElement>();

  const linkTo = (id: string) => list?.querySelector<HTMLElement>(`[data-id="${CSS.escape(id)}"]`);

  /** Marks the clicked heading, without moving the link that was clicked within the list. */
  async function onclick(id: string) {
    if (variant !== 'side' || !list) return;
    const before = linkTo(id)?.getBoundingClientRect().top;
    clicked = id;
    active = id;
    await tick();
    const after = linkTo(id)?.getBoundingClientRect().top;
    if (before !== undefined && after !== undefined) list.scrollTop += after - before;
  }

  $effect(() => {
    // Keep the marked heading in view within the (independently scrolling) list, as the reader
    // scrolls through the page.
    if (!active || !list || clicked) return;
    const link = linkTo(active);
    if (!link || link.offsetParent === null) return;
    const box = list.getBoundingClientRect();
    const top = link.getBoundingClientRect().top - box.top;
    if (top < 16 || top > list.clientHeight - 48) {
      list.scrollTop += top - list.clientHeight / 2;
    }
  });
</script>

{#snippet links()}
  <ul>
    {#each shown as item (item.id)}
      <li style:--depth={item.depth - 2}>
        <a
          href="#{item.id}"
          data-id={item.id}
          onclick={() => onclick(item.id)}
          aria-current={active === item.id ? 'location' : undefined}
        >
          {item.text}
        </a>
      </li>
    {/each}
  </ul>
{/snippet}

{#if items.length === 0}
  <!-- A page without sections has no table of contents. -->
{:else if variant === 'inline'}
  <details class="narrow">
    <summary>On this page</summary>
    {@render links()}
  </details>
{:else}
  <nav class="wide" aria-label="On this page" bind:this={list}>
    <p>On this page</p>
    {@render links()}
  </nav>
{/if}

<style>
  .narrow {
    margin-bottom: 1.5rem;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    font-size: 0.9375rem;
  }

  .narrow summary {
    padding: 0.5rem 0.9rem;
    font-weight: 600;
    cursor: pointer;
  }

  .narrow ul {
    max-height: 60vh;
    padding: 0 0.5rem 0.6rem;
    overflow-y: auto;
  }

  .wide {
    display: none;
    position: sticky;
    top: var(--header-height);
    max-height: calc(100dvh - var(--header-height));
    padding: 2rem 0 2rem;
    overflow-y: auto;
    font-size: 0.875rem;
  }

  .wide p {
    margin: 0 0 0.5rem 0.6rem;
    color: var(--text);
    font-weight: 600;
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  a {
    display: block;
    padding: 0.2rem 0.6rem;
    padding-left: calc(0.6rem + var(--depth) * 0.8rem);
    border-left: 2px solid transparent;
    color: var(--text-muted);
    line-height: 1.35;
  }

  a:hover {
    color: var(--text);
    text-decoration: none;
  }

  .wide a[aria-current] {
    border-left-color: var(--accent);
    color: var(--accent);
  }

  @media (min-width: 80rem) {
    .narrow {
      display: none;
    }

    .wide {
      display: block;
    }
  }
</style>
