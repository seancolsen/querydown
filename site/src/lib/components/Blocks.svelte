<!--
  Renders prose: HTML from Markdown, with a component for each code block. An element that holds a
  code block, such as the list item of a step in some instructions, is rendered here too, with its
  contents rendered by this component in turn.
-->
<script lang="ts">
  import type { Block } from '#lib/content-types.ts';

  import Blocks from './Blocks.svelte';
  import CodeBlock from './CodeBlock.svelte';
  import Example from './Example.svelte';

  let { blocks }: { blocks: Block[] } = $props();
</script>

{#each blocks as block}
  {#if block.type === 'html'}
    <!-- eslint-disable-next-line svelte/no-at-html-tags -- rendered from our own Markdown -->
    {@html block.html}
  {:else if block.type === 'element'}
    <svelte:element this={block.tag} {...block.attributes}>
      <Blocks blocks={block.children} />
    </svelte:element>
  {:else if block.type === 'code'}
    <CodeBlock source={block.source}>
      <!-- eslint-disable-next-line svelte/no-at-html-tags -- highlighted from our own Markdown -->
      {@html block.html}
    </CodeBlock>
  {:else if block.type === 'example'}
    <Example {...block} />
  {/if}
{/each}
