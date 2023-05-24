<div class="EditorGroup">
  <div class="tabs">
    <slot name="tabs" />
  </div>
  <slot />
</div>

<style>
  .EditorGroup {
    display: grid;
    grid-template: auto 1fr / 1fr;
    overflow: hidden;
  }
  .tabs {
    display: flex;
    flex-direction: row;
    align-items: flex-end;
    padding: 0.3rem 0 0 0.5rem;
    isolation: isolate;
    border-bottom: solid 1px #aaa;
  }
</style>
