<!--
  A slideshow of examples, each showing the Querydown, a description of it, and the SQL it compiles
  to. A note about all of them (such as which schema they use) may follow the slides.

  The slides sit in a row that scrolls and snaps, so the slideshow works by swiping or scrolling
  even without JavaScript. With it, the buttons and dots move between slides, and the slides
  advance by themselves until the visitor takes over.
-->
<script lang="ts">
  import type { Slide } from '#lib/content-types.ts';

  let { slides, note = null }: { slides: Slide[]; note?: string | null } = $props();

  const INTERVAL = 8000;

  let track: HTMLElement;
  let current = $state(0);
  /** Whether the slides advance by themselves. They stop for good once the visitor interacts. */
  let autoplay = $state(true);
  let hovered = $state(false);

  function show(index: number, behavior: ScrollBehavior = 'smooth') {
    const target = (index + slides.length) % slides.length;
    track.scrollTo({ left: target * track.clientWidth, behavior });
  }

  function onScroll() {
    current = Math.round(track.scrollLeft / track.clientWidth);
  }

  function takeOver() {
    autoplay = false;
  }

  $effect(() => {
    if (!autoplay || hovered) return;
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
    const timer = setInterval(() => {
      if (!document.hidden) show(current + 1);
    }, INTERVAL);
    return () => clearInterval(timer);
  });
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -- these only stop the autoplay -->
<section
  class="slideshow"
  aria-roledescription="carousel"
  aria-label="Examples of Querydown"
  onpointerenter={() => (hovered = true)}
  onpointerleave={() => (hovered = false)}
  onpointerdown={takeOver}
  onkeydown={takeOver}
  onwheel={takeOver}
  onfocusin={takeOver}
>
  <p class="badge" aria-hidden="true">Examples</p>

  <div class="track" bind:this={track} onscroll={onScroll}>
    {#each slides as slide, index}
      <div
        class="slide"
        role="group"
        aria-roledescription="slide"
        aria-label="{index + 1} of {slides.length}"
      >
        <div class="query">
          <div class="pane">
            <div class="label">
              <span>Querydown</span>
              <a href={slide.playground}>Open in the playground</a>
            </div>
            <!-- eslint-disable-next-line svelte/no-at-html-tags -- rendered from our own Markdown -->
            <pre class="qd"><code>{@html slide.code}</code></pre>
          </div>
          <div class="caption">
            <!-- eslint-disable-next-line svelte/no-at-html-tags -- rendered from our own Markdown -->
            {@html slide.caption}
          </div>
        </div>
        <svg class="arrow" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M12 4v15m-6-6 6 6 6-6" />
        </svg>
        <div class="pane">
          <div class="label"><span>SQL</span></div>
          <!-- eslint-disable-next-line svelte/no-at-html-tags -- highlighted compiler output -->
          {@html slide.sql}
        </div>
      </div>
    {/each}
  </div>

  <div class="controls">
    <button
      type="button"
      class="icon-button"
      aria-label="Previous example"
      onclick={() => show(current - 1)}
    >
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m14.5 6-6 6 6 6" /></svg>
    </button>
    <div class="dots">
      {#each slides as _, index}
        <button
          type="button"
          class="dot"
          aria-label="Example {index + 1}"
          aria-current={index === current ? 'true' : undefined}
          onclick={() => show(index)}
        ></button>
      {/each}
    </div>
    <button
      type="button"
      class="icon-button"
      aria-label="Next example"
      onclick={() => show(current + 1)}
    >
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m9.5 6 6 6-6 6" /></svg>
    </button>
  </div>

  {#if note}
    <div class="note">
      <!-- eslint-disable-next-line svelte/no-at-html-tags -- rendered from our own Markdown -->
      {@html note}
    </div>
  {/if}
</section>

<style>
  .slideshow {
    position: relative;
    margin: 2.5rem 0 1.5rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-subtle);
  }

  /* The label that sits on the top border. */
  .badge {
    position: absolute;
    top: 0;
    left: 50%;
    z-index: 1;
    margin: 0;
    padding: 0.1rem 0.8rem;
    border: 1px solid color-mix(in srgb, var(--accent) 30%, var(--border));
    border-radius: 999px;
    background: var(--accent-subtle);
    color: var(--accent);
    font-size: 0.75rem;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    transform: translate(-50%, -50%);
    white-space: nowrap;
  }

  .track {
    display: flex;
    overflow-x: auto;
    scroll-snap-type: x mandatory;
    scrollbar-width: none;
    overscroll-behavior-x: contain;
  }

  .track::-webkit-scrollbar {
    display: none;
  }

  /* On a narrow screen, a slide is a column: the Querydown, its description, an arrow pointing
     down, and the SQL. On a wide one, the Querydown and its description sit to the left of the
     SQL, with an arrow pointing right between them. */
  .slide {
    flex: none;
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    padding: 1.4rem 0.75rem 0.25rem;
    scroll-snap-align: start;
    scroll-snap-stop: always;
  }

  .query {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    min-width: 0;
  }

  .caption {
    padding: 0 0.5rem;
    color: var(--text-muted);
    font-size: 0.875rem;
    font-style: italic;
    line-height: 1.45;
    text-align: center;
    text-wrap: balance;
  }

  .caption :global(p) {
    margin: 0;
  }

  .caption :global(p)::before {
    content: '(';
  }

  .caption :global(p)::after {
    content: ')';
  }

  .arrow {
    flex: none;
    align-self: center;
    width: 1.5rem;
    height: 1.5rem;
    fill: none;
    stroke: var(--text-muted);
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .pane {
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg);
    overflow: hidden;
  }

  .label {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.3rem 0.8rem;
    border-bottom: 1px solid var(--border);
    color: var(--text-muted);
    font-size: 0.75rem;
    letter-spacing: 0.04em;
  }

  .label span {
    text-transform: uppercase;
  }

  /* Code wraps rather than scrolling sideways, so that a whole query is always in view. */
  .pane :global(pre) {
    margin: 0;
    padding: 0.75rem 0.8rem;
    background: none !important;
    font-size: 0.75rem;
    line-height: 1.5;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  /* The tallest slide sets the height of them all, so long SQL scrolls instead. */
  .pane :global(pre.shiki) {
    max-height: 18rem;
    overflow-y: auto;
  }

  .pane :global(pre.qd) {
    font-size: 0.875rem;
  }

  .pane :global(pre code) {
    font-size: inherit;
  }

  .controls {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.15rem 0 0.35rem;
  }

  .controls svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .dots {
    display: flex;
    gap: 0.1rem;
  }

  .dot {
    width: 1.4rem;
    height: 1.4rem;
    padding: 0;
    border: 0;
    background: none;
    cursor: pointer;
  }

  .dot::before {
    content: '';
    display: block;
    width: 0.5rem;
    height: 0.5rem;
    margin: auto;
    border-radius: 50%;
    background: var(--border-strong);
  }

  .dot[aria-current]::before {
    background: var(--accent);
  }

  .note {
    padding: 0 0.9rem 0.6rem;
    color: var(--text-muted);
    font-size: 0.8125rem;
    text-align: right;
  }

  .note :global(p) {
    margin: 0;
  }

  .note :global(a) {
    color: inherit;
    text-decoration: underline;
    text-decoration-color: color-mix(in srgb, currentColor 40%, transparent);
    text-underline-offset: 0.15em;
  }

  .note :global(a:hover) {
    color: var(--text);
    text-decoration-color: currentColor;
  }

  @media (min-width: 48rem) {
    .slide {
      display: grid;
      grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
      align-items: center;
      gap: 0.75rem;
      padding: 1.6rem 1.25rem 0.5rem;
    }

    .arrow {
      transform: rotate(-90deg);
    }
  }
</style>
