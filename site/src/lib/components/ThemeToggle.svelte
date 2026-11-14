<!--
  Switches between the light and dark themes. Both icons are rendered and CSS shows the one for the
  theme in effect, so the button is right before JavaScript loads, whatever the browser prefers.
-->
<script lang="ts">
  import { theme } from '#lib/theme.svelte.ts';
</script>

<button
  type="button"
  class="icon-button"
  onclick={() => theme.toggle()}
  aria-label="Switch between light and dark themes"
  title="Switch between light and dark themes"
>
  <svg class="sun" viewBox="0 0 24 24" aria-hidden="true">
    <circle cx="12" cy="12" r="4" />
    <path
      d="M12 2.5v2.5M12 19v2.5M2.5 12H5M19 12h2.5M5.3 5.3l1.8 1.8M16.9 16.9l1.8 1.8M5.3 18.7l1.8-1.8M16.9 7.1l1.8-1.8"
    />
  </svg>
  <svg class="moon" viewBox="0 0 24 24" aria-hidden="true">
    <path d="M20 14.5A8 8 0 0 1 9.5 4a8 8 0 1 0 10.5 10.5z" />
  </svg>
</button>

<style>
  svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  /* The icon shows the theme you'd switch to. */
  .sun {
    display: none;
  }

  @media (prefers-color-scheme: dark) {
    :global(:root:not([data-theme='light'])) .sun {
      display: block;
    }
    :global(:root:not([data-theme='light'])) .moon {
      display: none;
    }
  }

  :global(:root[data-theme='dark']) .sun {
    display: block;
  }

  :global(:root[data-theme='dark']) .moon {
    display: none;
  }
</style>
