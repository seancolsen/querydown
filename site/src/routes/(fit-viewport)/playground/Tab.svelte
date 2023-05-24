<script lang="ts">
  export let active: Element | undefined = undefined;
  export let tab: Element;
</script>

<button class="Tab passthrough" class:active={tab === active} on:click={() => (active = tab)}>
  <slot />
</button>

<style>
  .Tab {
    cursor: pointer;
    padding: 0.2em 0.6em 0.4em 0.6em;
    border-radius: 0.2em 0.2em 0 0;
  }
  .active {
    background-color: white;
    padding-top: 0.4em;
    border-style: solid;
    border-color: #aaa;
    border-width: 1px 1px 0 1px;
    position: relative;
    z-index: 1;
    top: 1px;
  }
  :not(.active) {
    background-color: #d9d9d9;
    color: #444;
  }
</style>
