<script lang="ts">
  import { page } from '$app/state';

  import Footer from '#lib/components/Footer.svelte';
  import { SITE_NAME } from '#lib/site.ts';
</script>

<svelte:head>
  <title>{page.status === 404 ? 'Page not found' : 'Error'} · {SITE_NAME}</title>
  <!-- Whatever status a host serves this page with, it isn't one to list in search results. -->
  <meta name="robots" content="noindex" />
</svelte:head>

<main>
  {#if page.status === 404}
    <h1>Page not found</h1>
    <p>
      There's no page at this address. Try the <a href="/">home page</a>, the
      <a href="/docs/language">language docs</a>, or the search in the header.
    </p>
  {:else}
    <h1>Something went wrong</h1>
    <p>{page.error?.message}</p>
  {/if}
  <Footer />
</main>

<style>
  main {
    max-width: 44rem;
    margin: 0 auto;
    padding: 4rem var(--gutter) 0;
  }

  h1 {
    margin: 0 0 1rem;
    font-family: var(--font-display);
    font-size: 2rem;
  }
</style>
