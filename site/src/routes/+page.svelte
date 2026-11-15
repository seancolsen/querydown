<script lang="ts">
  import Content from '#lib/components/Content.svelte';
  import Footer from '#lib/components/Footer.svelte';
  import Logo from '#lib/components/Logo.svelte';
  import Wordmark from '#lib/components/Wordmark.svelte';
  import { SITE_NAME, SITE_TAGLINE } from '#lib/site.ts';

  import type { PageProps } from './$types';

  let { data }: PageProps = $props();
</script>

<svelte:head>
  <title>{SITE_NAME} · {SITE_TAGLINE}</title>
  <meta name="description" content={data.description} />
</svelte:head>

<main class="home">
  <!-- The logo and the wordmark stand in for the page's title, which is kept for screen readers. -->
  <div class="hero" aria-hidden="true">
    <Logo class="hero-logo" />
    <Wordmark class="hero-wordmark" />
  </div>
  <Content segments={data.segments} styles={data.styles} />
  <Footer />
</main>

<style>
  .home {
    max-width: 54rem;
    margin: 0 auto;
    padding: 3rem var(--gutter) 0;
  }

  .hero {
    --size: clamp(2.75rem, 11vw, 4.25rem);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: calc(var(--size) * 0.3);
    margin-bottom: 1.5rem;
  }

  .hero :global(.hero-logo) {
    height: var(--size);
  }

  /* In the same proportion to the logo as in the header. */
  .hero :global(.hero-wordmark) {
    height: calc(var(--size) * 0.66);
  }

  /* The footer carries on the band that the questions sit on (see content/home.css), right up to
     the bottom of the page. */
  .home > :global(footer) {
    margin-top: 0;
    background: var(--bg-subtle);
    box-shadow: 0 0 0 100vmax var(--bg-subtle);
    clip-path: inset(0 -100vmax);
  }

  .hero + :global(.prose) > :global(h1:first-child) {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  /* The opening paragraph is the site's introduction. */
  .hero + :global(.prose) > :global(h1 + p) {
    max-width: 40rem;
    margin: 0 auto;
    color: var(--text-muted);
    font-size: clamp(1.1rem, 2.6vw, 1.3rem);
    line-height: 1.5;
    text-align: center;
    text-wrap: balance;
  }
</style>
