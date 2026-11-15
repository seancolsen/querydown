<!--
  The language's features, each with an example: a list of the features beside the Querydown of the
  chosen one, which is followed by a sentence saying what the query shows, and a link to the docs.
  The section's heading sits within the box, above both.

  The features are tabs. On a wide screen they're a column, and the chosen example sits level with
  its feature's name, or as near to level as it can be without leaving the box. On a narrow screen
  they're a row of pills that scrolls sideways, beside a button that lists them all, and the example
  beneath them is only as tall as it needs to be. Without JavaScript, the first feature's example
  shows, and the docs remain a link away.
-->
<script lang="ts">
  import type { Feature } from '#lib/content-types.ts';

  let { heading = null, features }: { heading?: string | null; features: Feature[] } = $props();

  const id = $props.id();

  let current = $state(0);
  /** Whether the visitor has chosen a feature, after which an example fades in as it appears. */
  let chosen = $state(false);
  /** Whether the list of every feature is open (on a narrow screen). */
  let menuOpen = $state(false);
  /** How far down the stage each example sits, in pixels (on a wide screen). */
  let offsets: number[] = $state([]);

  let picker: HTMLElement;
  let list: HTMLElement;
  let menuButton: HTMLButtonElement;
  let menu: HTMLElement | undefined = $state();
  let stage: HTMLElement;
  let tabs: HTMLButtonElement[] = $state([]);
  let panels: HTMLElement[] = $state([]);

  function select(index: number, focus = false) {
    current = (index + features.length) % features.length;
    chosen = true;
    menuOpen = false;
    const tab = tabs[current];
    if (focus) tab.focus();
    // Center the feature's pill in its row. (A column of features doesn't scroll, so stays put.)
    const row = list.getBoundingClientRect();
    const pill = tab.getBoundingClientRect();
    list.scrollTo({ left: list.scrollLeft + pill.left - row.left - (row.width - pill.width) / 2 });
  }

  function onKeydown(event: KeyboardEvent) {
    const moves: Record<string, number> = {
      ArrowDown: current + 1,
      ArrowRight: current + 1,
      ArrowUp: current - 1,
      ArrowLeft: current - 1,
      Home: 0,
      End: features.length - 1,
    };
    if (!(event.key in moves)) return;
    event.preventDefault();
    select(moves[event.key], true);
  }

  /**
   * Puts the middle of each example level with the middle of its feature's name, then moves it as
   * little as it takes to keep the whole example within the stage.
   *
   * This is only right on a wide screen, where the features are a column beside the stage and every
   * example is laid out. On a narrow one, the styles ignore the result.
   */
  function place() {
    const top = stage.getBoundingClientRect().top;
    const room = stage.clientHeight;
    offsets = panels.map((panel, index) => {
      const name = tabs[index].getBoundingClientRect();
      const height = panel.offsetHeight;
      const level = name.top + name.height / 2 - top - height / 2;
      return Math.round(Math.max(0, Math.min(level, room - height)));
    });
  }

  // The examples are placed again whenever something that decides their place changes size: when
  // the window is resized, say, or a font loads.
  $effect(() => {
    const observer = new ResizeObserver(place);
    for (const element of [list, stage, ...panels]) observer.observe(element);
    return () => observer.disconnect();
  });

  // The list of every feature is scrolled into view as it opens, since it can reach below the
  // window. It closes when the visitor presses Escape, or turns to something else.
  $effect(() => {
    if (!menuOpen) return;
    menu?.scrollIntoView({ block: 'nearest' });
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      menuOpen = false;
      menuButton.focus();
    };
    const onElsewhere = (event: Event) => {
      if (!picker.contains(event.target as Node)) menuOpen = false;
    };
    document.addEventListener('keydown', onKey);
    document.addEventListener('pointerdown', onElsewhere);
    document.addEventListener('focusin', onElsewhere);
    return () => {
      document.removeEventListener('keydown', onKey);
      document.removeEventListener('pointerdown', onElsewhere);
      document.removeEventListener('focusin', onElsewhere);
    };
  });
</script>

<section class="features">
  {#if heading}
    <div class="heading">
      <!-- eslint-disable-next-line svelte/no-at-html-tags -- rendered from our own Markdown -->
      {@html heading}
    </div>
  {/if}

  <div class="body">
    <div class="picker" bind:this={picker}>
      <div
        class="list"
        bind:this={list}
        role="tablist"
        aria-orientation="vertical"
        aria-label="Language features"
      >
        {#each features as feature, index}
          <button
            bind:this={tabs[index]}
            type="button"
            class="tab"
            role="tab"
            id="{id}-tab-{index}"
            aria-selected={index === current}
            aria-controls="{id}-panel-{index}"
            tabindex={index === current ? 0 : -1}
            onclick={() => select(index)}
            onkeydown={onKeydown}
          >
            {feature.name}
          </button>
        {/each}
      </div>

      <button
        bind:this={menuButton}
        type="button"
        class="all"
        aria-label="All features"
        aria-expanded={menuOpen}
        aria-controls="{id}-menu"
        onclick={() => (menuOpen = !menuOpen)}
      >
        All
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="m4 6 4 4 4-4" /></svg>
      </button>

      {#if menuOpen}
        <ul
          bind:this={menu}
          class="menu"
          id="{id}-menu"
          aria-label="All features"
          style:--rows={Math.ceil(features.length / 2)}
        >
          {#each features as feature, index}
            <li>
              <button
                type="button"
                aria-current={index === current ? 'true' : undefined}
                onclick={() => select(index, true)}
              >
                {feature.name}
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <div class="stage" class:chosen bind:this={stage}>
      {#each features as feature, index}
        <div
          bind:this={panels[index]}
          class="panel"
          class:active={index === current}
          role="tabpanel"
          id="{id}-panel-{index}"
          aria-labelledby="{id}-tab-{index}"
          inert={index !== current}
          style:--offset="{offsets[index] ?? 0}px"
        >
          <!-- eslint-disable-next-line svelte/no-at-html-tags -- highlighted from our own Markdown -->
          <pre class="qd"><code>{@html feature.code}</code></pre>
          <div class="caption">
            <!-- eslint-disable-next-line svelte/no-at-html-tags -- rendered from our own Markdown -->
            {@html feature.caption}
          </div>
          <p class="links">
            {#if feature.href}
              <a class="more" href={feature.href}>
                Read the docs<svg viewBox="0 0 16 16" aria-hidden="true"
                  ><path d="M3 8h9.5M8.5 4l4 4-4 4" /></svg
                >
              </a>
            {/if}
            <a class="try" href={feature.playground} target="_blank" rel="noopener">
              Open in the playground
            </a>
          </p>
        </div>
      {/each}
    </div>
  </div>
</section>

<style>
  .features {
    --glow: color-mix(in srgb, var(--accent) 10%, transparent);
    position: relative;
    margin: 3rem 0;
    padding: 1.4rem 1rem 1.25rem;
    border: 1px solid var(--border);
    border-radius: 14px;
    background:
      radial-gradient(110% 70% at 0% 0%, var(--glow), transparent 65%),
      radial-gradient(80% 60% at 100% 100%, var(--glow), transparent 70%), var(--bg-subtle);
    box-shadow: 0 18px 40px -28px var(--shadow);
    isolation: isolate;
  }

  /* A line of the accent color along the top edge, fading out at both ends. */
  .features::before {
    content: '';
    position: absolute;
    top: -1px;
    left: 12%;
    right: 12%;
    height: 2px;
    border-radius: 2px;
    background: linear-gradient(90deg, transparent, var(--accent), transparent);
    opacity: 0.7;
  }

  .heading :global(h2) {
    margin: 0 0 1.25rem;
    font-family: var(--font-display);
    font-size: 1.5rem;
    font-weight: 650;
    line-height: 1.25;
    letter-spacing: -0.01em;
    text-wrap: balance;
  }

  .heading :global(.heading-anchor) {
    margin-left: 0.4em;
    color: var(--text-muted);
    font-weight: 400;
    opacity: 0;
  }

  .heading :global(h2:hover .heading-anchor),
  .heading :global(.heading-anchor:focus-visible) {
    opacity: 1;
  }

  /* On a narrow screen, the features are a row of pills that scrolls sideways, above the example,
     with a button at the end of the row that lists them all. */
  .body {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .picker {
    position: relative;
    display: flex;
    align-items: center;
    gap: 0.35rem;
    min-width: 0;
  }

  .list {
    flex: 1;
    display: flex;
    gap: 0.35rem;
    min-width: 0;
    margin-left: -1rem;
    padding: 0.25rem 1rem;
    overflow-x: auto;
    scrollbar-width: none;
    overscroll-behavior-x: contain;
    mask-image: linear-gradient(90deg, transparent, #000 1rem, #000 calc(100% - 1rem), transparent);
  }

  .list::-webkit-scrollbar {
    display: none;
  }

  .tab,
  .all {
    position: relative;
    flex: none;
    padding: 0.32rem 0.8rem;
    border: 0;
    border-radius: 999px;
    font-size: 0.875rem;
    line-height: 1.4;
    white-space: nowrap;
    cursor: pointer;
    transition:
      color 0.15s,
      background-color 0.15s,
      box-shadow 0.15s;
  }

  .tab {
    background: color-mix(in srgb, var(--bg) 55%, transparent);
    box-shadow: inset 0 0 0 1px var(--border);
    color: var(--text-muted);
    text-align: left;
  }

  .tab:hover {
    color: var(--text);
  }

  .tab[aria-selected='true'] {
    background: var(--bg);
    box-shadow:
      inset 0 0 0 1px color-mix(in srgb, var(--accent) 45%, var(--border)),
      0 1px 3px var(--shadow);
    color: var(--accent);
  }

  .all {
    display: inline-flex;
    align-items: center;
    gap: 0.15rem;
    padding-right: 0.5rem;
    background: var(--bg);
    box-shadow:
      inset 0 0 0 1px var(--border-strong),
      0 1px 3px var(--shadow);
    color: var(--text);
    font-weight: 550;
  }

  .all[aria-expanded='true'] {
    background: var(--accent-subtle);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent) 45%, var(--border));
    color: var(--accent);
  }

  .all svg,
  .more svg {
    width: 1rem;
    height: 1rem;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.3;
    stroke-linecap: round;
    stroke-linejoin: round;
    transition: transform 0.15s;
  }

  .all[aria-expanded='true'] svg {
    transform: rotate(180deg);
  }

  /* Every feature, in two columns that read downwards, so that all of them fit on a phone. */
  .menu {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    z-index: 1;
    display: grid;
    grid-auto-flow: column;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    grid-template-rows: repeat(var(--rows), auto);
    gap: 0.1rem 0.25rem;
    margin: 0.15rem 0 0;
    padding: 0.35rem;
    scroll-margin-bottom: 0.75rem;
    list-style: none;
    border: 1px solid var(--border-strong);
    border-radius: 10px;
    background: var(--bg);
    box-shadow:
      0 2px 6px var(--shadow),
      0 16px 36px -12px var(--shadow);
  }

  .menu li {
    display: flex;
  }

  .menu button {
    flex: 1;
    padding: 0.45rem 0.6rem;
    border: 0;
    border-radius: 6px;
    background: none;
    color: var(--text);
    font-size: 0.875rem;
    line-height: 1.3;
    text-align: left;
    cursor: pointer;
  }

  .menu button:hover {
    background: var(--bg-inset);
  }

  .menu button[aria-current] {
    background: var(--accent-subtle);
    color: var(--accent);
    font-weight: 600;
  }

  .stage {
    min-width: 0;
  }

  /* Only the chosen feature's example shows. */
  .panel {
    display: none;
    min-width: 0;
  }

  .panel.active {
    display: block;
  }

  .chosen .panel.active {
    animation: appear 0.2s;
  }

  @keyframes appear {
    from {
      opacity: 0;
    }
  }

  /* The code looks as it does in the docs, but stands out from the box rather than from the page. */
  .panel pre {
    margin: 0;
    padding: 0.6rem 0.85rem;
    overflow-x: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg);
    font-size: 0.875rem;
    line-height: 1.5;
    tab-size: 2;
  }

  .panel pre code {
    font-size: inherit;
  }

  .caption {
    margin-top: 0.65rem;
    font-size: 0.9375rem;
    line-height: 1.5;
    text-wrap: pretty;
  }

  .caption :global(p) {
    margin: 0;
  }

  .links {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.2rem 1.25rem;
    margin: 0.5rem 0 0;
    font-size: 0.875rem;
  }

  .more {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    font-weight: 550;
  }

  .more:hover {
    text-decoration: none;
  }

  .more:hover svg {
    transform: translateX(2px);
  }

  .try {
    color: var(--text-muted);
  }

  .try:hover {
    color: var(--text);
  }

  /* On a wide screen, the features are a column to the left of the example. */
  @media (min-width: 48rem) {
    .features {
      padding: 1.9rem 2rem 2rem;
    }

    .heading :global(h2) {
      margin-bottom: 1.4rem;
    }

    .body {
      display: grid;
      grid-template-columns: 13.5rem minmax(0, 1fr);
      gap: 1.75rem;
    }

    .picker {
      display: block;
    }

    .all,
    .menu {
      display: none;
    }

    .list {
      flex-direction: column;
      gap: 0.1rem;
      margin: 0;
      padding: 0;
      overflow: visible;
      mask-image: none;
    }

    .tab {
      padding: 0.3rem 0.75rem;
      border-radius: 8px;
      background: none;
      box-shadow: none;
    }

    .tab:hover {
      background: color-mix(in srgb, var(--bg) 60%, transparent);
    }

    .tab[aria-selected='true'] {
      background: var(--bg);
      box-shadow:
        inset 0 0 0 1px var(--border),
        0 1px 3px var(--shadow);
      font-weight: 600;
    }

    /* A bar of the accent color at the left of the chosen feature. */
    .tab[aria-selected='true']::before {
      content: '';
      position: absolute;
      top: 0.45rem;
      bottom: 0.45rem;
      left: 0;
      width: 3px;
      border-radius: 0 3px 3px 0;
      background: var(--accent);
    }

    /* The stage is as tall as the column of features. Every example is laid out at the top of it,
       each as tall as its own contents, so that `place` can measure them all. Each is then moved
       down to be level with its feature's name. */
    .stage {
      display: grid;
      align-items: start;
    }

    .panel,
    .panel.active {
      grid-area: 1 / 1;
      display: block;
      translate: 0 var(--offset);
    }

    .panel:not(.active) {
      visibility: hidden;
    }
  }

  @media (prefers-reduced-motion: no-preference) {
    .list {
      scroll-behavior: smooth;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .tab,
    .all,
    .all svg,
    .more svg {
      transition: none;
    }

    .chosen .panel.active {
      animation: none;
    }
  }
</style>
