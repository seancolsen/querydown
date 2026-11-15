<script lang="ts">
  // The site's fonts, bundled into the build (see `--font-sans` in app.css).
  import '@fontsource-variable/red-hat-text/wght.css';
  import '@fontsource-variable/red-hat-text/wght-italic.css';
  import '@fontsource-variable/red-hat-display/wght.css';
  import '@fontsource-variable/red-hat-mono/wght.css';
  import '@fontsource-variable/red-hat-mono/wght-italic.css';
  import '../app.css';

  // The upright Latin fonts, which nearly every page shows straight away. Preloading them means the
  // browser doesn't wait for the stylesheet before it starts to fetch them. Vite gives these the
  // same URLs as the stylesheets above do.
  import textFont from '@fontsource-variable/red-hat-text/files/red-hat-text-latin-wght-normal.woff2?url';
  import displayFont from '@fontsource-variable/red-hat-display/files/red-hat-display-latin-wght-normal.woff2?url';
  import monoFont from '@fontsource-variable/red-hat-mono/files/red-hat-mono-latin-wght-normal.woff2?url';

  import { onMount } from 'svelte';

  import { afterNavigate, refreshAll } from '$app/navigation';
  import { page } from '$app/state';

  import Header from '#lib/components/Header.svelte';
  import NavTree from '#lib/components/NavTree.svelte';
  import Search from '#lib/components/Search.svelte';
  import { watchSections } from '#lib/reading.svelte.ts';

  import type { LayoutProps } from './$types';

  let { data, children }: LayoutProps = $props();

  let drawer: HTMLDialogElement;
  let drawerTree: NavTree;
  let search: Search;

  function openDrawer() {
    drawer.showModal();
    drawerTree.reveal();
  }

  /** Closes the drawer when a link in it is followed, or when the backdrop is clicked. */
  function onDrawerClick(event: MouseEvent) {
    const target = event.target as Element;
    if (target === drawer || target.closest('a')) drawer.close();
  }

  afterNavigate(() => drawer.close());

  // Follow the reader through the sections of the page, for the navigation to mark.
  $effect(() => watchSections(page.data.toc ?? []));

  onMount(() => {
    // While developing, re-render the page when a Markdown file changes (see vite.config.ts).
    import.meta.hot?.on('content:update', () => void refreshAll());
  });
</script>

<svelte:head>
  {#each [textFont, displayFont, monoFont] as href (href)}
    <link rel="preload" {href} as="font" type="font/woff2" crossorigin="anonymous" />
  {/each}
  <link rel="stylesheet" href="/querydown-theme.css" />
</svelte:head>

<Header nav={data.nav} onMenu={openDrawer} onSearch={() => search.open()} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<dialog class="drawer" bind:this={drawer} aria-label="Navigation" onclick={onDrawerClick}>
  <div class="drawer-header">
    <span>Navigation</span>
    <button type="button" class="icon-button" onclick={() => drawer.close()} aria-label="Close">
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 6l12 12M18 6 6 18" /></svg>
    </button>
  </div>
  <NavTree bind:this={drawerTree} nodes={data.nav} />
</dialog>

<Search bind:this={search} />

{@render children()}

<style>
  .drawer {
    position: fixed;
    inset: 0 auto 0 0;
    width: min(20rem, 85vw);
    height: 100dvh;
    max-height: none;
    margin: 0;
    padding: 0.5rem 0.75rem 2rem;
    border: 0;
    border-right: 1px solid var(--border);
    background: var(--bg);
    color: var(--text);
    overflow-y: auto;
  }

  .drawer::backdrop {
    background: rgb(0 0 0 / 0.4);
  }

  .drawer-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.5rem;
    padding-left: 0.6rem;
    color: var(--text-muted);
    font-size: 0.875rem;
  }

  .drawer-header svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
  }
</style>
