<script lang="ts">
  import Nav from '../../components/Header.svelte';
</script>

<div class="fit-viewport">
  <div class="top">
    <Nav />
  </div>
  <div class="bottom">
    <slot />
  </div>
</div>

<style>
  .fit-viewport {
    height: 100vh;
    display: grid;
    grid-template: auto 1fr / 1fr;
  }
  .top {
    flex: 0 0 auto;
  }
  .bottom {
    flex: 1 1 auto;
    overflow: hidden;
  }
</style>
