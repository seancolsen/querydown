<!--
  Renders a page's segments: HTML from Markdown, interleaved with the components it asked for. Also
  includes the page's own stylesheet, if it has one.
-->
<script lang="ts">
  import type { Segment } from '#lib/content-types.ts';

  import ErDiagram from './ErDiagram.svelte';
  import Slideshow from './Slideshow.svelte';

  interface Props {
    segments: Segment[];
    /** The page's own CSS. */
    styles?: string | null;
  }

  let { segments, styles = null }: Props = $props();

  // The stylesheet is our own, so it's included as it is.
  const styleTag = $derived(styles ? `<style>${styles}</style>` : '');
</script>

<svelte:head>
  <!-- eslint-disable-next-line svelte/no-at-html-tags -- our own CSS -->
  {@html styleTag}
</svelte:head>

{#each segments as segment}
  {#if segment.type === 'html'}
    <div class="prose">
      <!-- eslint-disable-next-line svelte/no-at-html-tags -- rendered from our own Markdown -->
      {@html segment.html}
    </div>
  {:else if segment.type === 'slideshow'}
    <Slideshow slides={segment.slides} note={segment.note} />
  {:else if segment.type === 'er-diagram'}
    <ErDiagram />
  {/if}
{/each}
