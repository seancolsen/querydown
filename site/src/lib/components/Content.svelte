<!-- Renders a page's segments: HTML from Markdown, interleaved with the components it asked for. -->
<script lang="ts">
  import type { Segment } from '#lib/content-types.ts';

  import ErDiagram from './ErDiagram.svelte';
  import Slideshow from './Slideshow.svelte';

  let { segments }: { segments: Segment[] } = $props();
</script>

{#each segments as segment}
  {#if segment.type === 'html'}
    <div class="prose">
      <!-- eslint-disable-next-line svelte/no-at-html-tags -- rendered from our own Markdown -->
      {@html segment.html}
    </div>
  {:else if segment.type === 'slideshow'}
    <Slideshow slides={segment.slides} />
  {:else if segment.type === 'er-diagram'}
    <ErDiagram />
  {/if}
{/each}
