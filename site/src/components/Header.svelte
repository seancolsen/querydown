<script lang="ts">
  import { page } from '$app/stores';

  import BrandName from '../icons/BrandName.svelte';
  import GitHub from '../icons/GitHub.svelte';
  import Logo from '../icons/Logo.svelte';

  const links = [
    { href: '/', name: 'Home' },
    // { href: '/docs', name: 'Docs' },
    { href: '/playground', name: 'Playground'},
  ];
  
</script>

<header>
  <div class="brand">
    <a href="/">
      <span class="logo">
        <Logo />
      </span>
      <span class="brand-name">
        <BrandName />
      </span>
    </a>
  </div>
  <div class="right">
    <nav>
      {#each links as {href, name} (href)}
        <a {href} class:active={$page.url.pathname === href}>{name}</a>
      {/each}
    </nav>
    <div class="external-links">
      <a
        href="https://github.com/seancolsen/querydown"
        target="_blank"
        title="Source code on GitHub"
      >
        <GitHub />
      </a>
    </div>
  </div>
</header>

<style>
  header {
    display: flex;
    justify-content: space-between;
    padding: 0.5rem 0.5rem 0 0.5rem;
  }
  .logo {
    font-size: 2rem;
    margin-right: 1rem;
  }
  .brand-name {
    font-size: 1.7rem;
  }
  .right {
    display: flex;
    align-items: center;
  }
  .external-links {
    margin-left: 2rem;
  }
  a {
    color: inherit;
    text-decoration: none;
  }
  .right a {
    padding: 0.5rem;
    font-size: 1.2rem;
    color: #444;
  }
  nav a:hover,
  nav a.active {
    color: black;
    border-bottom: solid 0.15rem #888;
  }
</style>
