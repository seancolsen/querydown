<script lang="ts">
  const ACTIVE_CLASS = 'active';

  export let active: Element | undefined = undefined;

  let area: HTMLElement | undefined;

  function handle_change_active(new_active: Element | undefined) {
    const old_active = area?.querySelector(`:scope > .${ACTIVE_CLASS}`);
    old_active?.classList.remove(ACTIVE_CLASS);
    new_active?.classList.add(ACTIVE_CLASS);
  }

  $: handle_change_active(active);
</script>

<div class="TabArea" bind:this={area}>
  <slot />
</div>

<style>
  .TabArea {
    height: 100%;
    overflow: hidden;
  }
  .TabArea > :global(*) {
    height: 100%;
  }
  .TabArea > :global(:not(.active)) {
    display: none;
  }
</style>
