<!--
  The headings of the current page. On a wide screen the `side` variant sits beside the content and
  marks the section being read; otherwise the `inline` variant is a collapsed list above the
  content. A page renders both, and CSS shows the one that suits the screen.
-->
<script lang="ts">
  import type { TocItem } from '#lib/content-types.ts';

  interface Props {
    items: TocItem[];
    /** `inline` is a collapsed list for narrow screens, and `side` a sidebar for wide ones. */
    variant: 'inline' | 'side';
  }

  let { items, variant }: Props = $props();

  let active = $state<string | null>(null);

  $effect(() => {
    if (variant !== 'side') return;
    const headings = items
      .map((item) => document.getElementById(item.id))
      .filter((element) => element !== null);
    if (headings.length === 0) return;

    // The section being read is the last one whose heading has passed the top of the viewport.
    const update = () => {
      const threshold = 96;
      let current: string | null = null;
      for (const heading of headings) {
        if (heading.getBoundingClientRect().top > threshold) break;
        current = heading.id;
      }
      active = current;
    };
    update();
    window.addEventListener('scroll', update, { passive: true });
    return () => window.removeEventListener('scroll', update);
  });

  let list = $state<HTMLElement>();

  $effect(() => {
    // Keep the marked heading in view within the (independently scrolling) list.
    if (!active || !list) return;
    const link = list.querySelector<HTMLElement>(`[data-id="${CSS.escape(active)}"]`);
    if (!link || link.offsetParent === null) return;
    const scroller = list.closest<HTMLElement>('.wide');
    if (!scroller) return;
    const top = link.offsetTop - scroller.offsetTop;
    if (top < scroller.scrollTop + 16 || top > scroller.scrollTop + scroller.clientHeight - 48) {
      scroller.scrollTop = top - scroller.clientHeight / 2;
    }
  });
</script>

{#snippet links()}
  <ul>
    {#each items as item (item.id)}
      <li style:--depth={item.depth - 2}>
        <a
          href="#{item.id}"
          data-id={item.id}
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
