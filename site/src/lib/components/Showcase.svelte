<!--
  An example of Querydown beside the SQL that it compiles to, to show at a glance how much SQL a
  little Querydown stands for. The SQL is small, and isn't meant to be read so much as seen.

  On a wide screen the two sit side by side, and the Querydown decides how tall the box is: the SQL
  fills the height beside it, and scrolls. On a narrow screen the SQL sits below, at a fixed height.
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
    --glow: color-mix(in srgb, var(--accent) 11%, transparent);
    --dots: color-mix(in srgb, var(--text-muted) 18%, transparent);
    --label-height: 1.6rem;
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin: 2.5rem 0 2rem;
    padding: 1rem 1rem 1.1rem;
    border: 1px solid var(--border);
    border-radius: 14px;
    background: radial-gradient(55% 80% at 42% 50%, var(--glow), transparent 75%), var(--bg-subtle);
    box-shadow: 0 18px 40px -28px var(--shadow);
    isolation: isolate;
  }

  /* A faint grid of dots behind it all, fading out toward the edges of the box. */
  .showcase::before {
    content: '';
    position: absolute;
    inset: 0;
    z-index: -1;
    border-radius: inherit;
    background: radial-gradient(circle at 1px 1px, var(--dots) 1px, transparent 1.5px) 0 0 / 14px
      14px;
    mask-image: radial-gradient(70% 90% at 40% 50%, #000, transparent);
    pointer-events: none;
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
     that it only scrolls one way. */
  .sql .pane {
    height: 13rem;
  }

  .sql .pane :global(pre) {
    height: 100%;
    padding: 0.6rem 0.75rem 1.75rem;
    overflow-y: auto;
    font-size: 0.625rem;
    line-height: 1.5;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    scrollbar-width: thin;
    scrollbar-color: var(--border-strong) transparent;
  }

  /* The SQL fades out at the foot of its pane, as if it goes on. */
  .sql .pane::after {
    content: '';
    position: absolute;
    inset: auto 0 0;
    height: 1.75rem;
    background: linear-gradient(transparent, var(--bg));
    pointer-events: none;
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
      /* The Querydown is never narrower than its longest line. */
      grid-template-columns: minmax(max-content, 2fr) auto minmax(0, 3fr);
      align-items: stretch;
      gap: 1.1rem;
      padding: 1.4rem 1.6rem 1.6rem;
    }

    /* The SQL takes no height of its own, so it's stretched to the height of the Querydown. */
    .sql {
      contain: size;
    }

    .sql .pane {
      flex: 1;
      height: auto;
      min-height: 0;
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
