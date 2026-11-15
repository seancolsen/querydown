<!--
  The site's navigation, as a tree. A group is a collapsible label, and only pages are links. Only
  the groups that hold the current page start open, and moving to another page opens the groups
  that hold it and closes the ones that don't.
-->
<script lang="ts">
  import { page } from '$app/state';

  import { contains, isGroup, type NavNode } from '#lib/nav.ts';

  import Self from './NavTree.svelte';

  let { nodes, depth = 0 }: { nodes: NavNode[]; depth?: number } = $props();
</script>

<ul class:nested={depth > 0}>
  {#each nodes as node (isGroup(node) ? node.path : node.href)}
    <li>
      {#if isGroup(node)}
        <details open={contains(node, page.url.pathname)}>
          <summary>{node.title}</summary>
          <Self nodes={node.children} depth={depth + 1} />
        </details>
      {:else}
        <a href={node.href} aria-current={page.url.pathname === node.href ? 'page' : undefined}>
          {node.title}
        </a>
      {/if}
    </li>
  {/each}
</ul>

<style>
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  ul.nested {
    margin-left: 0.6rem;
    padding-left: 0.5rem;
    border-left: 1px solid var(--border);
  }

  a,
  summary {
    display: block;
    padding: 0.3rem 0.6rem;
    border-radius: var(--radius);
    color: var(--text-muted);
    line-height: 1.35;
  }

  a:hover,
  summary:hover {
    background: var(--bg-inset);
    color: var(--text);
    text-decoration: none;
  }

  a[aria-current='page'] {
    background: var(--accent-subtle);
    color: var(--accent);
    font-weight: 600;
  }

  summary {
    display: flex;
    align-items: center;
    justify-content: space-between;
    color: var(--text);
    font-weight: 600;
    cursor: pointer;
    list-style: none;
    user-select: none;
  }

  summary::-webkit-details-marker {
    display: none;
  }

  summary::after {
    content: '';
    width: 0.4rem;
    height: 0.4rem;
    margin-right: 0.2rem;
    border: solid var(--text-muted);
    border-width: 0 1.5px 1.5px 0;
    transform: rotate(-45deg);
    transition: transform 0.1s;
  }

  details[open] > summary::after {
    transform: rotate(45deg);
  }

  li {
    margin: 1px 0;
  }
</style>
