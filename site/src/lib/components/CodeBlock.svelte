<!--
  A block of code in the docs, with a button that copies it. Its other actions, if it has any, go
  beside that button.
-->
<script lang="ts">
  import type { Snippet } from 'svelte';

  import CodeAction from './CodeAction.svelte';

  interface Props {
    /** The code, to copy. */
    source: string;
    /** Something sits beneath the block, so its bottom corners are square. */
    attached?: boolean;
    /** The code is an example that doesn't compile, so the block is tinted to show it. */
    failed?: boolean;
    /** The highlighted code, as a `<pre>` element. */
    children: Snippet;
    /** More `CodeAction`s, after the button that copies the code. */
    actions?: Snippet;
  }

  let { source, attached = false, failed = false, children, actions }: Props = $props();

  let copied = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;

  /**
   * Copies text with a hidden textarea and `execCommand`, for when the async clipboard API isn't
   * available (it exists only in secure contexts, so not on a page served over plain HTTP from
   * anywhere but localhost).
   */
  function copyWithSelection(text: string): boolean {
    const area = document.createElement('textarea');
    area.value = text;
    area.setAttribute('readonly', '');
    area.style.cssText = 'position:fixed;top:0;left:0;opacity:0';
    document.body.append(area);
    area.select();
    try {
      return document.execCommand('copy');
    } finally {
      area.remove();
    }
  }

  async function copy() {
    try {
      await navigator.clipboard.writeText(source);
    } catch {
      if (!copyWithSelection(source)) return;
    }
    copied = true;
    clearTimeout(timer);
    timer = setTimeout(() => (copied = false), 1500);
  }
</script>

<div class="code-block" class:attached class:failed>
  {@render children()}
  <div class="actions">
    {#if copied}<span class="copied">Copied</span>{/if}
    <CodeAction label="Copy" onclick={copy} done={copied}>
      {#snippet icon()}
        <path
          d="M5.5 1.5h7a1 1 0 0 1 1 1v8.5M3.5 4.5h7a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1h-7a1 1 0 0 1-1-1v-8a1 1 0 0 1 1-1z"
        />
      {/snippet}
    </CodeAction>
    {@render actions?.()}
  </div>
</div>

<style>
  .code-block {
    position: relative;
    margin: 0 0 1rem;
  }

  .code-block > :global(pre) {
    margin: 0;
    padding: 0.5rem 0.75rem;
    overflow-x: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-subtle);
    line-height: 1.5;
    tab-size: 2;
  }

  .code-block > :global(pre code) {
    font-size: inherit;
  }

  .attached > :global(pre) {
    border-radius: var(--radius) var(--radius) 0 0;
  }

  .failed > :global(pre) {
    border-color: color-mix(in srgb, var(--danger) 40%, var(--border));
  }

  .actions {
    position: absolute;
    top: 0.25rem;
    right: 0.25rem;
    display: flex;
    gap: 0.15rem;
    border-radius: var(--radius);
    background: var(--bg-subtle);
    opacity: 0.55;
    transition: opacity 0.1s;
  }

  .code-block:hover .actions,
  .actions:focus-within,
  .actions:has(:global([aria-expanded='true'])) {
    opacity: 1;
  }

  .copied {
    position: absolute;
    top: 50%;
    right: 100%;
    translate: 0 -50%;
    margin-right: 0.4rem;
    padding: 0.1rem 0.4rem;
    border-radius: 4px;
    background: var(--bg-inset);
    color: var(--accent);
    font-size: 0.75rem;
    white-space: nowrap;
  }
</style>
