<!--
  Renders a page's segments: prose from Markdown, interleaved with the components it asked for. Also
  includes the page's own stylesheet, if it has one.
-->
<script lang="ts">
  import type { Segment } from '#lib/content-types.ts';

  import Blocks from './Blocks.svelte';
  import ErDiagram from './ErDiagram.svelte';
  import Features from './Features.svelte';
  import Showcase from './Showcase.svelte';

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
  {#if segment.type === 'prose'}
    <div class="prose">
      <Blocks blocks={segment.blocks} />
    </div>
  {:else if segment.type === 'showcase'}
    <Showcase code={segment.code} sql={segment.sql} />
  {:else if segment.type === 'features'}
    <Features heading={segment.heading} features={segment.features} note={segment.note} />
  {:else if segment.type === 'er-diagram'}
    <ErDiagram />
  {/if}
{/each}
