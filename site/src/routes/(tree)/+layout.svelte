<!-- The layout for pages that sit in the navigation tree, which is shown beside them. -->
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
    max-width: 92rem;
    margin: 0 auto;
    padding: 0 var(--gutter);
  }

  aside {
    display: none;
  }

  @media (min-width: 64rem) {
    .layout {
      display: grid;
      grid-template-columns: 15rem minmax(0, 1fr);
      gap: 2.5rem;
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
