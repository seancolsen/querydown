<!--
  The layout for pages that sit in the navigation tree, which is shown beside them (with the page's
  table of contents within it).
-->
<script lang="ts">
  import NavTree from '#lib/components/NavTree.svelte';

  import type { LayoutProps } from './$types';

  let { data, children }: LayoutProps = $props();
</script>

<div class="layout">
  <aside>
    <nav aria-label="Site">
      <NavTree nodes={data.nav} />
    </nav>
  </aside>
  {@render children()}
</div>

<style>
  .layout {
    --nav-width: 17rem;
    --content-width: 48rem;
    --column-gap: 2.5rem;
    /* Just wide enough for the two columns, so that there's no space between them to spare. */
    max-width: calc(
      var(--nav-width) + var(--content-width) + var(--column-gap) + 2 * var(--gutter)
    );
    margin: 0 auto;
    padding: 0 var(--gutter);
  }

  aside {
    display: none;
  }

  @media (min-width: 64rem) {
    .layout {
      display: grid;
      grid-template-columns: var(--nav-width) minmax(0, 1fr);
      gap: var(--column-gap);
    }

    aside {
      display: block;
      position: sticky;
      top: var(--header-height);
      align-self: start;
      max-height: calc(100dvh - var(--header-height));
      margin-left: -0.6rem;
      padding: 1.75rem 0.5rem 2rem 0;
      overflow-y: auto;
      font-size: 0.9375rem;
    }
  }
</style>
