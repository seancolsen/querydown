<!--
  An example in the docs: its description, its code, and then what it compiles to.

  An example that compiles has a button that shows its SQL, which links to the playground. One that
  doesn't compile, or whose output the page shows itself, links to the playground directly. One
  that's expected to fail underlines the part of its code that the error is about, and explains
  the error beneath it, in a way that shows the failure to be part of the lesson.
-->
<script lang="ts">
  import type { Example } from '#lib/content-types.ts';

  import CodeAction from './CodeAction.svelte';
  import CodeBlock from './CodeBlock.svelte';

  let { caption, source, code, playground, sql, error }: Example = $props();

  let showSql = $state(false);

  /**
   * A compiler message, split into its text (at even positions) and the code it quotes with
   * backticks, as Markdown does (at odd ones).
   */
  const parts = (message: string) => message.split(/`([^`]+)`/);
</script>

<div class="example">
  {#if caption}
    <div class="caption">
      <!-- eslint-disable-next-line svelte/no-at-html-tags -- rendered from our own Markdown -->
      {@html caption}
    </div>
  {/if}

  <CodeBlock {source} attached={showSql || !!error} failed={!!error}>
    <!-- eslint-disable-next-line svelte/no-at-html-tags -- highlighted from our own Markdown -->
    <pre class="qd"><code>{@html code}</code></pre>
    {#snippet actions()}
      {#if sql}
        <CodeAction
          label={showSql ? 'Hide the SQL' : 'Show the SQL'}
          expanded={showSql}
          onclick={() => (showSql = !showSql)}
        >
          {#snippet icon()}<path d="M4 6l4 4 4-4" />{/snippet}
        </CodeAction>
      {:else if playground}
        <CodeAction label="Open in the playground" href={playground}>
          {#snippet icon()}<path d="M5 3.5v9l7.5-4.5z" />{/snippet}
        </CodeAction>
      {/if}
    {/snippet}
  </CodeBlock>

  {#if error}
    <p class="error">
      This query fails to compile.
      {@render message(error.message)}
      {#if error.hint}<span class="hint">{@render message(error.hint)}</span>{/if}
    </p>
  {:else if sql && showSql}
    <div class="sql">
      {#if playground}
        <a class="playground" href={playground} target="_blank" rel="noopener">
          Open in the playground
        </a>
      {/if}
      <pre><code>{sql}</code></pre>
    </div>
  {/if}
</div>

{#snippet message(text: string)}
  {#each parts(text) as part, index}{#if index % 2}<code>{part}</code>{:else}{part}{/if}{/each}
{/snippet}

<style>
  .example {
    margin: 0 0 1.25rem;
  }

  .example :global(.code-block) {
    margin-bottom: 0;
  }

  .caption {
    margin: 0 0 0.4rem;
    color: var(--text-muted);
    font-style: italic;
  }

  .caption > :global(p) {
    margin: 0;
  }

  .sql {
    position: relative;
    margin-top: -1px;
    border: 1px solid var(--border);
    border-radius: 0 0 var(--radius) var(--radius);
    font-size: 0.875rem;
  }

  .playground {
    position: absolute;
    top: 0.4rem;
    right: 0.6rem;
    padding: 0 0.25rem;
    border-radius: 4px;
    background: var(--bg);
    font-size: 0.75rem;
  }

  .sql pre {
    margin: 0;
    padding: 0.5rem 0.75rem;
    overflow-x: auto;
    line-height: 1.5;
    color: var(--text-muted);
  }

  .sql code {
    font-size: inherit;
  }

  .error {
    margin: -1px 0 0;
    padding: 0.45rem 1rem;
    border: 1px solid color-mix(in srgb, var(--danger) 40%, var(--border));
    border-radius: 0 0 var(--radius) var(--radius);
    background: var(--danger-subtle);
    font-size: 0.875rem;
  }

  /* The part of the code that the error is about, as `highlightQuerydown` marks it. */
  .example :global(.qd-mark) {
    text-decoration: underline wavy var(--danger);
    text-decoration-skip-ink: none;
    text-decoration-thickness: 1px;
    text-underline-offset: 0.2em;
  }

  .hint {
    color: var(--text-muted);
  }
</style>
