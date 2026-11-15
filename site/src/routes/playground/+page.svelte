<!--
  The playground's page. The playground itself (with the compiler and CodeMirror) is loaded in the
  browser once the page opens, so that none of it weighs on the rest of the site.
-->
<script lang="ts">
  import { onMount, type Component } from 'svelte';

  import PageMeta from '#lib/components/PageMeta.svelte';
  import { PLAYGROUND } from '#lib/nav.ts';
  import { SITE_NAME } from '#lib/site.ts';

  let Playground = $state<Component | null>(null);
  let failure = $state<string | null>(null);

  onMount(async () => {
    try {
      const [{ default: init }, { default: wasmUrl }, playground] = await Promise.all([
        import('querydown-js'),
        import('querydown-js/querydown_js_bg.wasm?url'),
        import('./Playground.svelte'),
      ]);
      await init({ module_or_path: wasmUrl });
      Playground = playground.default;
    } catch (error) {
      console.error(error);
      failure = error instanceof Error ? error.message : String(error);
    }
  });
</script>

<PageMeta
  title="{SITE_NAME} playground · Write a query, see its SQL"
  description="Write Querydown in your browser, and see the SQL that it compiles to for PostgreSQL or DuckDB. Nothing to install."
  path={PLAYGROUND.href}
/>

<main>
  <h1 class="visually-hidden">Playground</h1>
  {#if Playground}
    <Playground />
  {:else if failure}
    <p class="status" role="alert">The playground couldn't be loaded: {failure}</p>
  {:else}
    <p class="status">Loading the compiler…</p>
    <noscript><p class="status">The playground needs JavaScript.</p></noscript>
  {/if}
</main>

<style>
  main {
    display: flex;
    flex-direction: column;
    height: calc(100dvh - var(--header-height));
    min-height: 28rem;
  }

  .status {
    margin: auto;
    padding: 2rem;
    color: var(--text-muted);
  }
</style>
