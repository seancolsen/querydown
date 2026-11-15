<script lang="ts">
  import { page } from '$app/state';

  import { contains, firstPage, HOME, type NavNode } from '#lib/nav.ts';
  import { GITHUB_URL } from '#lib/site.ts';

  import GitHubIcon from './GitHubIcon.svelte';
  import Logo from './Logo.svelte';
  import ThemeToggle from './ThemeToggle.svelte';
  import Wordmark from './Wordmark.svelte';

  interface Props {
    nav: NavNode[];
    onMenu: () => void;
    onSearch: () => void;
  }

  let { nav, onMenu, onSearch }: Props = $props();

  /** The top-level items, each linking to its page, or (for a group) to its first page. */
  const links = $derived(
    nav.flatMap((node) => {
      const target = firstPage(node);
      return !target || target.href === HOME.href
        ? []
        : [{ node, title: node.title, href: target.href }];
    }),
  );

  /** Whether the page is scrolled to the very top. */
  let atTop = $state(true);

  $effect(() => {
    const update = () => (atTop = window.scrollY <= 0);
    update();
    window.addEventListener('scroll', update, { passive: true });
    return () => window.removeEventListener('scroll', update);
  });

  /** The home page's hero needs no line beneath the header until the page scrolls. */
  const borderless = $derived(atTop && page.url.pathname === HOME.href);
</script>

<header class:borderless>
  <button type="button" class="icon-button menu" onclick={onMenu} aria-label="Open the navigation">
    <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 7h16M4 12h16M4 17h16" /></svg>
  </button>

  <a class="brand" href={HOME.href} aria-label="Querydown home">
    <Logo class="logo" aria-hidden="true" />
    <Wordmark class="wordmark" aria-hidden="true" />
  </a>

  <nav aria-label="Sections">
    {#each links as link (link.href)}
      <a
        href={link.href}
        aria-current={contains(link.node, page.url.pathname) ? 'true' : undefined}
      >
        {link.title}
      </a>
    {/each}
  </nav>

  <div class="tools">
    <button type="button" class="search" onclick={onSearch} aria-label="Search the site">
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <circle cx="11" cy="11" r="6.5" /><path d="m16 16 4.5 4.5" />
      </svg>
      <span class="label">Search</span>
      <kbd>/</kbd>
    </button>
    <ThemeToggle />
    <a
      class="icon-button github"
      href={GITHUB_URL}
      target="_blank"
      rel="noopener"
      aria-label="Querydown on GitHub"
    >
      <GitHubIcon />
      <span class="label">GitHub</span>
    </a>
  </div>
</header>

<style>
  header {
    position: sticky;
    top: 0;
    z-index: 10;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    height: var(--header-height);
    padding: 0 var(--gutter);
    border-bottom: 1px solid var(--border);
    background: var(--bg);
    transition: border-color 0.15s;
  }

  header.borderless {
    border-bottom-color: transparent;
  }

  .menu {
    margin-left: -0.5rem;
  }

  .menu svg,
  .search svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    margin-inline: -0.4rem;
    padding: 0.3rem 0.4rem;
    border-radius: var(--radius);
    color: var(--text);
    transition: color 0.15s;
  }

  .brand:hover {
    color: var(--accent);
    text-decoration: none;
  }

  .brand :global(.logo) {
    height: 1.6rem;
  }

  .brand :global(.wordmark) {
    height: 1.05rem;
  }

  nav {
    display: none;
    gap: 0.25rem;
    margin-left: 1.25rem;
  }

  nav a {
    padding: 0.3rem 0.65rem;
    border-radius: var(--radius);
    color: var(--text-muted);
  }

  nav a:hover {
    background: var(--bg-inset);
    color: var(--text);
    text-decoration: none;
  }

  nav a[aria-current] {
    color: var(--text);
    font-weight: 600;
  }

  .tools {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    margin-left: auto;
  }

  .search {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    height: 2.25rem;
    padding: 0 0.6rem;
    border: 1px solid transparent;
    border-radius: var(--radius);
    background: none;
    color: var(--text-muted);
    cursor: pointer;
  }

  .search:hover {
    background: var(--bg-inset);
    color: var(--text);
  }

  .search svg {
    width: 1.15rem;
    height: 1.15rem;
  }

  .search .label,
  .search kbd,
  .github .label {
    display: none;
  }

  .search kbd {
    padding: 0 0.35rem;
    border: 1px solid var(--border-strong);
    border-radius: 4px;
    font-size: 0.75rem;
    line-height: 1.4;
  }

  .github {
    color: var(--text);
  }

  @media (min-width: 40rem) {
    .search {
      min-width: 11rem;
      border-color: var(--border);
      background: var(--bg-subtle);
    }

    .search .label {
      display: inline;
      flex: 1;
      text-align: left;
    }

    .search kbd,
    .github .label {
      display: inline;
    }

    .github {
      padding: 0 0.65rem;
      border: 1px solid var(--border);
      font-weight: 600;
    }
  }

  @media (min-width: 64rem) {
    .menu {
      display: none;
    }

    nav {
      display: flex;
    }
  }
</style>
