<!--
  A page's entry in its `<head>` for search engines and link previews: its title and description,
  its canonical URL, and the Open Graph tags that chat apps and social networks build a preview
  from. Every page that should be found by a search engine has one.
-->
<script lang="ts">
  import { canonicalUrl, summarize } from '#lib/seo.ts';
  import { SITE_NAME, SOCIAL_CARD } from '#lib/site.ts';

  interface Props {
    /** The whole `<title>`, including the site's name. */
    title: string;
    /** What the page is for. It's shortened to what a search engine shows. */
    description: string;
    /**
     * The page's path, such as `/docs/language`. It's given, rather than read from the address
     * that the page was reached at, because a static host may serve the page at others too
     * (`/docs/language.html`, `/docs/language/`), and they should all name the same one.
     */
    path: string;
    /** The Open Graph type: `website` for the site's own pages, `article` for pages of content. */
    type?: 'website' | 'article';
    /** A description of the page in schema.org's vocabulary, for search engines, as JSON-LD. */
    structuredData?: Record<string, unknown>;
  }

  let { title, description, path, type = 'website', structuredData }: Props = $props();

  const url = $derived(canonicalUrl(path));
  const summary = $derived(summarize(description));
  const card = $derived(canonicalUrl(SOCIAL_CARD.path));

  // `<` is escaped so that nothing in the data can end the script element.
  const jsonLd = $derived(
    structuredData
      ? `<script type="application/ld+json">${JSON.stringify(structuredData).replaceAll('<', '\\u003c')}<\/script>`
      : '',
  );
</script>

<svelte:head>
  <title>{title}</title>
  <meta name="description" content={summary} />
  <link rel="canonical" href={url} />
  <meta property="og:site_name" content={SITE_NAME} />
  <meta property="og:type" content={type} />
  <meta property="og:title" content={title} />
  <meta property="og:description" content={summary} />
  <meta property="og:url" content={url} />
  <meta property="og:image" content={card} />
  <meta property="og:image:width" content={String(SOCIAL_CARD.width)} />
  <meta property="og:image:height" content={String(SOCIAL_CARD.height)} />
  <meta property="og:image:alt" content={SOCIAL_CARD.alt} />
  <meta name="twitter:card" content="summary_large_image" />
  <!-- eslint-disable-next-line svelte/no-at-html-tags -- our own data, as JSON -->
  {@html jsonLd}
</svelte:head>
