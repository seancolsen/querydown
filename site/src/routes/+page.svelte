<script lang="ts">
  import Content from '#lib/components/Content.svelte';
  import Footer from '#lib/components/Footer.svelte';
  import Logo from '#lib/components/Logo.svelte';
  import { SITE_NAME, SITE_TAGLINE } from '#lib/site.ts';

  import type { PageProps } from './$types';

  let { data }: PageProps = $props();
</script>

<svelte:head>
  <title>{SITE_NAME} · {SITE_TAGLINE}</title>
  <meta name="description" content={data.description} />
</svelte:head>

<main class="home">
  <Logo class="hero-logo" aria-hidden="true" />
  <Content segments={data.segments} />
  <Footer />
</main>

<style>
  .home {
    max-width: 54rem;
    margin: 0 auto;
    padding: 3rem var(--gutter) 0;
  }

  .home :global(.hero-logo) {
    display: block;
    height: 4.5rem;
    margin-bottom: 1.25rem;
  }

  /* The page's title and its opening paragraph are the site's introduction. */
  .home :global(.prose:first-of-type > h1) {
    margin-bottom: 0.75rem;
    font-size: clamp(2.2rem, 6vw, 3rem);
    letter-spacing: -0.02em;
  }

  .home :global(.prose:first-of-type > h1 + p) {
    max-width: 40rem;
    color: var(--text-muted);
    font-size: clamp(1.1rem, 2.6vw, 1.3rem);
    line-height: 1.5;
    text-wrap: balance;
  }
</style>
