<!--
  A small icon button in the corner of a code block (see `CodeBlock`), or a link that opens in a new
  tab if it has an `href`.
-->
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    /** What the action does, for its tooltip and for screen readers. */
    label: string;
    /** Where the action goes, if it's a link. */
    href?: string;
    onclick?: () => void;
    /** Whether what it shows is showing, if it's a toggle. It looks pressed while it is. */
    expanded?: boolean;
    /** Whether it has just done its job, which tints it. */
    done?: boolean;
    /** Its icon: the contents of an SVG whose view box is 16 by 16. */
    icon: Snippet;
  }

  let { label, href, onclick, expanded, done = false, icon }: Props = $props();
</script>

{#if href}
  <a class="code-action" {href} target="_blank" rel="noopener" aria-label={label} title={label}>
    <svg viewBox="0 0 16 16" aria-hidden="true">{@render icon()}</svg>
  </a>
{:else}
  <button
    type="button"
    class="code-action"
    class:done
    aria-expanded={expanded}
    aria-label={label}
    title={label}
    {onclick}
  >
    <svg viewBox="0 0 16 16" aria-hidden="true">{@render icon()}</svg>
  </button>
{/if}

<style>
  .code-action {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.9rem;
    height: 1.9rem;
    padding: 0;
    border: 0;
    border-radius: var(--radius);
    background: none;
    color: var(--text-muted);
    cursor: pointer;
  }

  .code-action:hover {
    background: var(--bg-inset);
    color: var(--text);
  }

  svg {
    width: 1rem;
    height: 1rem;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.3;
    stroke-linejoin: round;
    stroke-linecap: round;
  }

  .done {
    color: var(--accent);
  }

  [aria-expanded='true'] {
    background: var(--accent-subtle);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent) 40%, transparent);
    color: var(--accent);
  }

  [aria-expanded='true'] svg {
    transform: rotate(180deg);
  }
</style>
