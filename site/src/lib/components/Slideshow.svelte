<!--
  A slideshow of examples, each showing a description, the Querydown, and the SQL it compiles to.

  The slides sit in a row that scrolls and snaps, so the slideshow works by swiping or scrolling
  even without JavaScript. With it, the buttons and dots move between slides, and the slides
  advance by themselves until the visitor takes over.
-->
<script lang="ts">
  import type { Slide } from '#lib/content-types.ts';

  let { slides }: { slides: Slide[] } = $props();

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
  <div class="track" bind:this={track} onscroll={onScroll}>
    {#each slides as slide, index}
      <div
        class="slide"
        role="group"
        aria-roledescription="slide"
        aria-label="{index + 1} of {slides.length}"
      >
        <div class="caption">
          <!-- eslint-disable-next-line svelte/no-at-html-tags -- rendered from our own Markdown -->
          {@html slide.caption}
        </div>
        <div class="panes">
          <div class="pane">
            <div class="label">
              <span>Querydown</span>
              <a href={slide.playground}>Open in the playground</a>
            </div>
            <!-- eslint-disable-next-line svelte/no-at-html-tags -- rendered from our own Markdown -->
            <pre class="qd"><code>{@html slide.code}</code></pre>
          </div>
          <div class="pane">
            <div class="label"><span>SQL</span></div>
            <!-- eslint-disable-next-line svelte/no-at-html-tags -- highlighted compiler output -->
            {@html slide.sql}
          </div>
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
</section>

<style>
  .slideshow {
    margin: 1.5rem 0 1rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-subtle);
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

  .slide {
    flex: none;
    width: 100%;
    display: flex;
    flex-direction: column;
    padding: 1.1rem 1.25rem 0.5rem;
    scroll-snap-align: start;
    scroll-snap-stop: always;
  }

  .caption {
    min-height: 3.2em;
    margin-bottom: 0.75rem;
    font-size: 1.05rem;
    text-wrap: balance;
  }

  .caption :global(p) {
    margin: 0;
  }

  .panes {
    display: grid;
    gap: 0.75rem;
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

  .pane :global(pre) {
    /* Every slide is the same height, so that the page doesn't jump from one to the next. Longer
       code scrolls. */
    height: 13.5rem;
    margin: 0;
    padding: 0.75rem 0.8rem;
    overflow: auto;
    background: none !important;
    font-size: 0.8125rem;
    line-height: 1.5;
  }

  .pane :global(pre.qd) {
    height: 6.5rem;
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

  @media (min-width: 48rem) {
    .panes {
      grid-template-columns: 1fr 1fr;
    }

    .caption {
      min-height: 1.6em;
    }

    .pane :global(pre.qd) {
      height: 13.5rem;
    }
  }
</style>
