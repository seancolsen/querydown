<!--
  Site search, in a dialog. The index (`/search.json`) and the search library are only fetched once
  the dialog is first opened.
-->
<script lang="ts">
  import { goto } from '$app/navigation';
  import type MiniSearch from 'minisearch';

  import { tokenize, type SearchEntry } from '#lib/search.ts';

  const MAX_RESULTS = 12;
  const SNIPPET_LENGTH = 140;

  let dialog: HTMLDialogElement;
  let input: HTMLInputElement;
  let list = $state<HTMLElement>();
  let query = $state('');
  let selected = $state(0);
  let index = $state.raw<MiniSearch<SearchEntry> | null>(null);
  let failed = $state(false);
  let loading: Promise<void> | undefined;

  async function load() {
    try {
      const [{ default: MiniSearch }, response] = await Promise.all([
        import('minisearch'),
        fetch('/search.json'),
      ]);
      if (!response.ok) throw new Error(`Could not load the search index (${response.status}).`);
      const entries: SearchEntry[] = await response.json();
      const built = new MiniSearch<SearchEntry>({
        fields: ['heading', 'page', 'text'],
        storeFields: ['url', 'page', 'heading', 'trail', 'text'],
        tokenize,
      });
      built.addAll(entries);
      index = built;
    } catch (error) {
      console.error(error);
      failed = true;
      loading = undefined;
    }
  }

  export function open() {
    failed = false;
    loading ??= load();
    if (!dialog.open) dialog.showModal();
    input.select();
  }

  const terms = $derived(tokenize(query.toLowerCase()));

  const results = $derived.by(() => {
    if (!index || terms.length === 0) return [];
    return index
      .search(query, {
        // Match the start of a term while it's being typed, and forgive a typo in a longer word.
        prefix: true,
        fuzzy: (term) => (term.length > 4 ? 0.2 : false),
        boost: { heading: 4, page: 2 },
        combineWith: 'AND',
      })
      .slice(0, MAX_RESULTS) as unknown as SearchEntry[];
  });

  $effect(() => {
    // Start again from the first result whenever the results change.
    void results;
    selected = 0;
  });

  /** A stretch of the section's text around the first term it matches, split up for marking. */
  function snippet(text: string): { text: string; match: boolean }[] {
    const flat = text.replace(/\s+/g, ' ').trim();
    const lower = flat.toLowerCase();
    const first = Math.min(
      ...terms.map((term) => lower.indexOf(term)).filter((position) => position >= 0),
    );
    const start = Number.isFinite(first) ? Math.max(0, first - 40) : 0;
    const shown = flat.slice(start, start + SNIPPET_LENGTH);
    const pattern = new RegExp(
      `(${terms.map((term) => term.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')).join('|')})`,
      'gi',
    );
    const parts = shown
      .split(pattern)
      .filter(Boolean)
      .map((part) => ({ text: part, match: terms.includes(part.toLowerCase()) }));
    if (start > 0) parts.unshift({ text: '… ', match: false });
    if (start + SNIPPET_LENGTH < flat.length) parts.push({ text: ' …', match: false });
    return parts;
  }

  function choose(entry: SearchEntry) {
    dialog.close();
    void goto(entry.url);
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      const step = event.key === 'ArrowDown' ? 1 : -1;
      selected = Math.max(0, Math.min(results.length - 1, selected + step));
      list?.children[selected]?.scrollIntoView({ block: 'nearest' });
    } else if (event.key === 'Enter' && results[selected]) {
      event.preventDefault();
      choose(results[selected]);
    }
  }

  function onWindowKeydown(event: KeyboardEvent) {
    const target = event.target as HTMLElement | null;
    const typing =
      target?.closest('input, textarea, select, [contenteditable="true"], .cm-editor') != null;
    const slash = event.key === '/' && !typing && !event.ctrlKey && !event.metaKey;
    const commandK = event.key.toLowerCase() === 'k' && (event.ctrlKey || event.metaKey);
    if (slash || commandK) {
      event.preventDefault();
      open();
    }
  }
</script>

<svelte:window onkeydown={onWindowKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<dialog
  bind:this={dialog}
  aria-label="Search"
  onclick={(event) => event.target === dialog && dialog.close()}
>
  <div class="panel">
    <div class="field">
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <circle cx="11" cy="11" r="6.5" /><path d="m16 16 4.5 4.5" />
      </svg>
      <input
        bind:this={input}
        bind:value={query}
        onkeydown={onKeydown}
        type="search"
        placeholder="Search the docs"
        aria-label="Search the docs"
        autocomplete="off"
        autocapitalize="off"
        spellcheck="false"
      />
      <button type="button" class="close" onclick={() => dialog.close()}>Esc</button>
    </div>

    {#if failed}
      <p class="status">The search index couldn't be loaded.</p>
    {:else if terms.length === 0}
      <p class="status">
        Search for a word or for syntax, such as <code>sorting</code> or <code>++</code>.
      </p>
    {:else if !index}
      <p class="status">Loading…</p>
    {:else if results.length === 0}
      <p class="status">Nothing matches “{query}”.</p>
    {:else}
      <ul bind:this={list}>
        {#each results as result, i (result.id)}
          <li>
            <a
              href={result.url}
              class:selected={i === selected}
              onclick={(event) => {
                event.preventDefault();
                choose(result);
              }}
              onmousemove={() => (selected = i)}
            >
              <span class="trail">{result.trail.join(' › ')}</span>
              <span class="heading">{result.heading || result.page}</span>
              <span class="snippet">
                {#each snippet(result.text) as part}
                  {#if part.match}<mark>{part.text}</mark>{:else}{part.text}{/if}
                {/each}
              </span>
            </a>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</dialog>

<style>
  dialog {
    width: min(40rem, calc(100vw - 2rem));
    max-height: min(36rem, calc(100dvh - 4rem));
    margin-top: 4rem;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--bg);
    color: var(--text);
    box-shadow: 0 1rem 3rem var(--shadow);
  }

  dialog::backdrop {
    background: rgb(0 0 0 / 0.4);
  }

  dialog[open] {
    display: flex;
  }

  .panel {
    display: flex;
    flex-direction: column;
    width: 100%;
    min-height: 0;
  }

  .field {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.6rem 0.9rem;
    border-bottom: 1px solid var(--border);
  }

  .field svg {
    flex: none;
    width: 1.2rem;
    height: 1.2rem;
    fill: none;
    stroke: var(--text-muted);
    stroke-width: 1.8;
    stroke-linecap: round;
  }

  input {
    flex: 1;
    min-width: 0;
    padding: 0.3rem 0;
    border: 0;
    outline: 0;
    background: none;
    font-size: 1.05rem;
  }

  input::-webkit-search-cancel-button {
    display: none;
  }

  .close {
    padding: 0.1rem 0.45rem;
    border: 1px solid var(--border-strong);
    border-radius: 4px;
    background: none;
    color: var(--text-muted);
    font-size: 0.75rem;
    cursor: pointer;
  }

  .status {
    margin: 0;
    padding: 1.25rem 1rem;
    color: var(--text-muted);
  }

  .status code {
    padding: 0.1em 0.3em;
    border-radius: 4px;
    background: var(--bg-inset);
  }

  ul {
    margin: 0;
    padding: 0.4rem;
    overflow-y: auto;
    list-style: none;
  }

  a {
    display: grid;
    gap: 0.1rem;
    padding: 0.5rem 0.65rem;
    border-radius: var(--radius);
    color: var(--text);
  }

  a:hover {
    text-decoration: none;
  }

  a.selected {
    background: var(--accent-subtle);
  }

  .trail {
    color: var(--text-muted);
    font-size: 0.75rem;
  }

  .heading {
    font-weight: 600;
  }

  .snippet {
    color: var(--text-muted);
    font-size: 0.875rem;
    overflow-wrap: anywhere;
  }

  mark {
    background: none;
    color: var(--accent);
    font-weight: 600;
  }
</style>
