<!--
  An example of Querydown beside the SQL that it compiles to, to show at a glance how much SQL a
  little Querydown stands for. The SQL is small, and isn't meant to be read so much as seen.

  Each box is as tall as its code. On a wide screen the two sit side by side, at the same fixed
  width, with their middles level, so that the arrow between them is in the middle of the page. On
  a narrow screen the SQL sits below.
-->
<script lang="ts">
  let {
    code,
    sql,
  }: {
    /** The highlighted Querydown, as HTML. */
    code: string;
    /** The highlighted SQL, as the HTML of a `<pre>` element. */
    sql: string;
  } = $props();
</script>

<section class="showcase" aria-label="An example of Querydown, and the SQL it compiles to">
  <div class="side querydown">
    <p class="label">Querydown</p>
    <div class="pane">
      <!-- eslint-disable-next-line svelte/no-at-html-tags -- highlighted from our own Markdown -->
      <pre class="qd"><code>{@html code}</code></pre>
    </div>
  </div>

  <svg class="arrow" viewBox="0 0 40 32" aria-hidden="true">
    <path d="M4 11.5h18V5l14 11-14 11v-6.5H4z" />
  </svg>

  <div class="side sql">
    <p class="label">SQL</p>
    <div class="pane">
      <!-- eslint-disable-next-line svelte/no-at-html-tags -- highlighted compiler output -->
      {@html sql}
    </div>
  </div>
</section>

<style>
  .showcase {
    --wash: color-mix(in srgb, var(--bg-inset) 40%, var(--bg-subtle));
    --dots: color-mix(in srgb, var(--text-muted) 18%, transparent);
    --label-height: 1.6rem;
    /* How far above and below the example its backdrop reaches. */
    --reach: 8rem;
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin: 3.5rem 0 3.25rem;
  }

  /* The backdrop: a soft wash of color, faintly blue in the middle, that fades out in every
     direction, flowing out past the edges of the window at the sides. A border image's outset is
     painted, but takes no room, so the backdrop doesn't make the page scroll sideways. The gradient
     is as wide as the outset, so its width is set from the middle of the window. The showcase spans
     the page's centered column, with the example centered within it, so the backdrop is centered in
     the window.

     It sits behind everything else on the page (so isn't isolated with the example), lest it tint
     the text above and below. */
  .showcase::before,
  .showcase::after {
    content: '';
    position: absolute;
    z-index: -1;
    pointer-events: none;
  }

  .showcase::before {
    inset: 0;
    border-image: radial-gradient(
        60vw 50% at 50% 50%,
        color-mix(in srgb, var(--accent) 10%, var(--wash)),
        var(--wash) 38%,
        color-mix(in srgb, var(--wash) 80%, transparent) 56%,
        color-mix(in srgb, var(--wash) 40%, transparent) 76%,
        transparent
      )
      fill 0 / 0 / var(--reach) 100vmax;
  }

  /* A faint grid of dots over the middle of the backdrop, fading out before the edges of the page's
     content. */
  .showcase::after {
    inset: -2.5rem calc(-1 * var(--gutter));
    background: radial-gradient(circle at 1px 1px, var(--dots) 1px, transparent 1.5px) 0 0 / 14px
      14px;
    mask-image: radial-gradient(closest-side, #000 40%, transparent);
  }

  .side {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .label {
    height: calc(var(--label-height) - 0.4rem);
    margin: 0 0 0.4rem 0.15rem;
    color: var(--text-muted);
    font-size: 0.75rem;
    font-weight: 650;
    letter-spacing: 0.07em;
    line-height: calc(var(--label-height) - 0.4rem);
    text-transform: uppercase;
  }

  .querydown .label {
    color: var(--accent);
  }

  .pane {
    position: relative;
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg);
    overflow: hidden;
  }

  /* The Querydown is the point, so it stands forward of the SQL. */
  .querydown .pane {
    border-color: color-mix(in srgb, var(--accent) 35%, var(--border));
    box-shadow:
      0 0 0 4px color-mix(in srgb, var(--accent) 7%, transparent),
      0 2px 8px -2px var(--shadow);
  }

  .pane :global(pre) {
    margin: 0;
    background: none !important;
  }

  .pane :global(pre code) {
    font-size: inherit;
  }

  .querydown pre {
    padding: 0.8rem 1rem;
    overflow-x: auto;
    font-size: 0.9375rem;
    line-height: 1.6;
  }

  /* Small enough that there's plainly a lot of it, and still legible up close. The lines wrap, so
     that it never scrolls. */
  .sql .pane :global(pre) {
    padding: 0.6rem 0.75rem;
    font-size: 0.6rem;
    line-height: 1.2;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  /* A block arrow, lighter than everything around it: part of the picture, not a control. */
  .arrow {
    flex: none;
    align-self: center;
    width: 2.25rem;
    height: 1.8rem;
    margin-block: 0.2rem;
    rotate: 90deg;
    fill: color-mix(in srgb, var(--accent) 24%, var(--bg-subtle));
    stroke: color-mix(in srgb, var(--accent) 24%, var(--bg-subtle));
    stroke-width: 3;
    stroke-linejoin: round;
    mask-image: linear-gradient(90deg, rgb(0 0 0 / 0.3), #000 65%);
  }

  @media (min-width: 48rem) {
    .showcase {
      display: grid;
      /* The two panes are the same width, so that, centered, they put the arrow in the middle of
         the page. Each is as tall as its code, and they're centered on each other: their labels
         are the same height, so the middles of the panes are level too. */
      grid-template-columns: 18rem auto 18rem;
      justify-content: center;
      align-items: center;
      gap: 1.1rem;
    }

    .arrow {
      width: 2.75rem;
      height: 2.2rem;
      /* Level with the middle of the panes, below their labels. */
      margin: var(--label-height) 0 0;
      rotate: none;
    }
  }
</style>
