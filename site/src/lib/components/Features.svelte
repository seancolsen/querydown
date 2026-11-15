<!--
  The language's features, each with an example: a list of the features beside a window that shows
  the Querydown of the chosen one. The section's heading sits within the box, above both.

  The features are tabs. Every example is rendered, stacked in one place, so that the window is as
  tall as the longest of them and doesn't change size from one feature to the next. Without
  JavaScript, the window shows the first feature, and the docs remain a link away.
-->
<script lang="ts">
  import type { Feature } from '#lib/content-types.ts';

  let { heading = null, features }: { heading?: string | null; features: Feature[] } = $props();

  let current = $state(0);
  let copied = $state(false);
  let tabs: HTMLButtonElement[] = $state([]);

  /** A file name for an example, from its feature's name: "User-defined functions" is `user-defined-functions.qd`. */
  const fileName = (name: string) =>
    `${name
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-|-$/g, '')}.qd`;

  const lineCount = (source: string) => source.split('\n').length;

  function select(index: number, focus = false) {
    current = (index + features.length) % features.length;
    copied = false;
    const tab = tabs[current];
    if (focus) tab.focus();
    tab.scrollIntoView({ block: 'nearest', inline: 'nearest' });
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

  function copy() {
    const index = current;
    void navigator.clipboard.writeText(features[index].source).then(() => {
      if (current !== index) return;
      copied = true;
      setTimeout(() => (copied = false), 1500);
    });
  }
</script>

<section class="features">
  {#if heading}
    <div class="heading">
      <!-- eslint-disable-next-line svelte/no-at-html-tags -- rendered from our own Markdown -->
      {@html heading}
    </div>
  {/if}

  <div class="body">
    <div class="list" role="tablist" aria-orientation="vertical" aria-label="Language features">
      {#each features as feature, index}
        <button
          bind:this={tabs[index]}
          type="button"
          class="tab"
          role="tab"
          id="feature-tab-{index}"
          aria-selected={index === current}
          aria-controls="feature-panel-{index}"
          tabindex={index === current ? 0 : -1}
          onclick={() => select(index)}
          onkeydown={onKeydown}
        >
          <span class="number" aria-hidden="true">{String(index + 1).padStart(2, '0')}</span>
          <span class="name">{feature.name}</span>
        </button>
      {/each}
    </div>

    <div class="stage">
      {#each features as feature, index}
        <div
          class="window"
          class:active={index === current}
          role="tabpanel"
          id="feature-panel-{index}"
          aria-labelledby="feature-tab-{index}"
          inert={index !== current}
        >
          <div class="titlebar">
            <span class="lights" aria-hidden="true"><i></i><i></i><i></i></span>
            <span class="file">{fileName(feature.name)}</span>
            <span class="actions">
              <button
                type="button"
                class="action"
                class:done={copied && index === current}
                aria-label="Copy"
                title="Copy"
                onclick={copy}
              >
                {#if copied && index === current}
                  <svg viewBox="0 0 16 16" aria-hidden="true"><path d="m3.5 8.5 3 3 6-7" /></svg>
                {:else}
                  <svg viewBox="0 0 16 16" aria-hidden="true">
                    <path
                      d="M5.5 1.5h7a1 1 0 0 1 1 1v8.5M3.5 4.5h7a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1h-7a1 1 0 0 1-1-1v-8a1 1 0 0 1 1-1z"
                    />
                  </svg>
                {/if}
              </button>
              <a
                class="action"
                href={feature.playground}
                target="_blank"
                rel="noopener"
                aria-label="Open in the playground"
                title="Open in the playground"
              >
                <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M5 3.5v9l7.5-4.5z" /></svg>
              </a>
            </span>
          </div>

          <div class="about">
            <div class="caption">
              <!-- eslint-disable-next-line svelte/no-at-html-tags -- rendered from our own Markdown -->
              {@html feature.caption}
            </div>
            {#if feature.href}
              <a class="more" href={feature.href}>
                Read the docs<svg viewBox="0 0 16 16" aria-hidden="true"
                  ><path d="M3 8h9.5M8.5 4l4 4-4 4" /></svg
                >
              </a>
            {/if}
          </div>
          <div class="code">
            <ol class="gutter" aria-hidden="true">
              {#each { length: lineCount(feature.source) }, line}
                <li>{line + 1}</li>
              {/each}
            </ol>
            <!-- eslint-disable-next-line svelte/no-at-html-tags -- highlighted from our own Markdown -->
            <pre class="qd"><code>{@html feature.code}</code></pre>
          </div>
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
    padding: 1.4rem 1rem 1rem;
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

  /* On a narrow screen, the features are a row of pills that scrolls sideways, above the window. */
  .body {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .list {
    display: flex;
    gap: 0.35rem;
    margin: 0 -1rem;
    padding: 0.15rem 1rem 0.35rem;
    overflow-x: auto;
    scrollbar-width: none;
    overscroll-behavior-x: contain;
    mask-image: linear-gradient(90deg, transparent, #000 1rem, #000 calc(100% - 1rem), transparent);
  }

  .list::-webkit-scrollbar {
    display: none;
  }

  .tab {
    position: relative;
    flex: none;
    display: flex;
    align-items: baseline;
    gap: 0.6rem;
    padding: 0.32rem 0.8rem;
    border: 0;
    border-radius: 999px;
    background: color-mix(in srgb, var(--bg) 55%, transparent);
    box-shadow: inset 0 0 0 1px var(--border);
    color: var(--text-muted);
    font-size: 0.875rem;
    line-height: 1.4;
    text-align: left;
    white-space: nowrap;
    cursor: pointer;
    transition:
      color 0.15s,
      background-color 0.15s,
      box-shadow 0.15s;
  }

  .number {
    font-family: var(--font-mono);
    font-size: 0.6875rem;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.02em;
    opacity: 0.65;
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

  .tab[aria-selected='true'] .number {
    opacity: 1;
  }

  /* The windows of every feature share one place, so the tallest sets the height of them all. */
  .stage {
    display: grid;
    min-width: 0;
  }

  .window {
    grid-area: 1 / 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--bg);
    box-shadow:
      0 1px 2px var(--shadow),
      0 16px 36px -20px var(--shadow);
    overflow: hidden;
    opacity: 0;
    visibility: hidden;
    transform: translateY(6px);
    transition:
      opacity 0.2s,
      transform 0.2s,
      visibility 0s linear 0.2s;
  }

  .window.active {
    opacity: 1;
    visibility: visible;
    transform: none;
    transition:
      opacity 0.2s,
      transform 0.2s;
  }

  .titlebar {
    display: flex;
    align-items: center;
    gap: 0.8rem;
    padding: 0.3rem 0.4rem 0.3rem 0.9rem;
    border-bottom: 1px solid var(--border);
    background: var(--bg-subtle);
  }

  .lights {
    display: flex;
    gap: 0.35rem;
  }

  .lights i {
    width: 0.6rem;
    height: 0.6rem;
    border-radius: 50%;
    background: var(--border-strong);
    opacity: 0.8;
  }

  .file {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    color: var(--text-muted);
    font-family: var(--font-mono);
    font-size: 0.75rem;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .actions {
    display: flex;
    gap: 0.1rem;
  }

  .action {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.8rem;
    height: 1.8rem;
    padding: 0;
    border: 0;
    border-radius: var(--radius);
    background: none;
    color: var(--text-muted);
    cursor: pointer;
  }

  .action:hover {
    background: var(--bg-inset);
    color: var(--text);
  }

  .action.done {
    color: var(--accent);
  }

  .action svg,
  .more svg {
    width: 1rem;
    height: 1rem;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.3;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .code {
    flex: 1;
    display: flex;
    padding: 1rem 0 1.25rem;
    font-family: var(--font-mono);
    font-size: 0.875rem;
    line-height: 1.7;
  }

  .gutter {
    flex: none;
    margin: 0;
    padding: 0 0.9rem 0 1rem;
    color: var(--text-muted);
    list-style: none;
    opacity: 0.5;
    text-align: right;
    user-select: none;
  }

  .code pre {
    flex: 1;
    min-width: 0;
    margin: 0;
    padding: 0 1.25rem 0 0.25rem;
    overflow-x: auto;
    font-size: inherit;
    line-height: inherit;
    tab-size: 2;
  }

  .code pre code {
    font-size: inherit;
  }

  /* What the example finds, and a link to the docs, between the title bar and the code. */
  .about {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.4rem 1.25rem;
    padding: 0.8rem 1rem 0.85rem;
    border-bottom: 1px dashed var(--border);
    font-size: 0.9375rem;
    line-height: 1.45;
  }

  .caption {
    flex: 1 1 16rem;
    color: var(--text);
    text-wrap: pretty;
  }

  .caption :global(p) {
    margin: 0;
  }

  .more {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    margin-left: auto;
    font-size: 0.875rem;
    font-weight: 550;
    white-space: nowrap;
  }

  .more svg {
    transition: transform 0.15s;
  }

  .more:hover {
    text-decoration: none;
  }

  .more:hover svg {
    transform: translateX(2px);
  }

  /* On a wide screen, the features are a vertical list to the left of the window. */
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
      gap: 1.5rem;
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

    .code {
      font-size: 0.9375rem;
      padding-top: 1.25rem;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .tab,
    .window,
    .window.active,
    .more svg {
      transition: none;
    }

    .window {
      transform: none;
    }
  }
</style>
