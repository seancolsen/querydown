<script lang="ts">
  import Nav from '../../components/Header.svelte';
</script>

<Nav />

<main>
  <slot />
</main>

<style>
  main {
    padding: 0.5rem;
  }
</style>
