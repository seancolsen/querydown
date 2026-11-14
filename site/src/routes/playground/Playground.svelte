<!--
  The playground: an editor for a query (and for the schema it's compiled against), beside the SQL
  and the annotations that the query compiles to.

  Its state lives in the URL's fragment (see `lib/playground-url.ts`), so that a link can open the
  playground with a particular query, which is how the docs' examples open here.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import type { Diagnostic } from '@codemirror/lint';
  import {
    compile,
    is_querydown_error,
    QuerydownCompleter,
    type CompileResult,
    type QuerydownError,
  } from 'querydown-js';

  import { goto } from '$app/navigation';

  import {
    DIALECTS,
    parsePlaygroundFragment,
    playgroundFragment,
    type Dialect,
  } from '#lib/playground-url.ts';
  import { theme } from '#lib/theme.svelte.ts';

  import sampleSchema from '../../../../compiler/resources/test/issue_schema.json?raw';
  import {
    createJsonEditor,
    createQueryEditor,
    createSqlEditor,
    type Editor,
    type QueryEditor,
  } from './editors';
  import { decodeSchema, encodeSchema } from './schema-codec';

  const STARTING_QUERY = String.raw`// Welcome to the Querydown playground!
// Edit this query, and the SQL beside it updates as you type.

#issues
status:open
created_at:<6m
\\due_date
$title
$author.username -> author
$#comments -> comments @{align:right}
`;

  const DIALECT_NAMES: Record<Dialect, string> = { postgres: 'PostgreSQL', duckdb: 'DuckDB' };

  // -----------------------------------------------------------------------------------------------
  // State
  // -----------------------------------------------------------------------------------------------

  let query = $state(STARTING_QUERY);
  let schema = $state(sampleSchema);
  let dialect = $state<Dialect>('postgres');
  let limit = $state<number | null>(null);
  let offset = $state<number | null>(null);

  let inputTab = $state<'query' | 'schema'>('query');
  let outputTab = $state<'sql' | 'metadata'>('sql');
  let copied = $state(false);

  const customSchema = $derived(schema !== sampleSchema);
  const count = (value: number | null) =>
    value !== null && Number.isInteger(value) && value >= 0 ? value : null;

  type Outcome = { result: CompileResult; error: null } | { result: null; error: QuerydownError };

  const outcome = $derived.by((): Outcome => {
    try {
      const options = { limit: count(limit), offset: count(offset) };
      return { result: compile(schema, dialect, query, options), error: null };
    } catch (error) {
      if (!is_querydown_error(error)) throw error;
      return { result: null, error };
    }
  });

  const sql = $derived(outcome.result?.sql ?? '');
  const metadata = $derived(
    outcome.result
      ? JSON.stringify(
          {
            columnAnnotations: outcome.result.columnAnnotations,
            queryAnnotation: outcome.result.queryAnnotation,
          },
          null,
          2,
        )
      : '',
  );

  /** Underlines for the query editor. A schema error isn't about the query, so it has none. */
  const diagnostics = $derived.by((): Diagnostic[] => {
    const span = outcome.error?.span;
    if (!outcome.error || !span) return [];
    const from = Math.min(span.start.offset, query.length);
    // An empty span (e.g. at the end of the input) is widened so that there's something to see.
    const to = Math.min(Math.max(span.end.offset, from + 1), query.length);
    return [{ from: Math.min(from, to), to, severity: 'error', message: outcome.error.reason }];
  });

  // -----------------------------------------------------------------------------------------------
  // The URL
  // -----------------------------------------------------------------------------------------------

  /** Takes the playground's state from the URL's fragment, if it has one. */
  async function readUrl() {
    const link = parsePlaygroundFragment(location.hash);
    if (link.query === undefined) return;
    query = link.query;
    dialect = link.dialect ?? 'postgres';
    limit = link.limit ?? null;
    offset = link.offset ?? null;
    try {
      schema = link.schema ? await decodeSchema(link.schema) : sampleSchema;
    } catch (error) {
      console.error('The schema in the URL could not be read.', error);
      schema = sampleSchema;
    }
  }

  let ready = $state(false);

  $effect(() => {
    // Keep the URL in step with the state, once things settle after an edit.
    if (!ready) return;
    const link = { query, dialect, limit: count(limit), offset: count(offset) };
    const custom = customSchema ? schema : null;
    const timer = setTimeout(async () => {
      const encoded = custom ? await encodeSchema(custom) : null;
      // A shallow navigation changes the URL without loading anything.
      void goto(`#${playgroundFragment({ ...link, schema: encoded })}`, {
        shallow: true,
        replace: true,
      });
    }, 300);
    return () => clearTimeout(timer);
  });

  async function copyLink() {
    const encoded = customSchema ? await encodeSchema(schema) : null;
    const link = { query, dialect, limit: count(limit), offset: count(offset), schema: encoded };
    const url = new URL(location.href);
    url.hash = playgroundFragment(link);
    await navigator.clipboard.writeText(url.href);
    copied = true;
    setTimeout(() => (copied = false), 1500);
  }

  function reset() {
    query = STARTING_QUERY;
    schema = sampleSchema;
    dialect = 'postgres';
    limit = null;
    offset = null;
    inputTab = 'query';
  }

  // -----------------------------------------------------------------------------------------------
  // The editors
  // -----------------------------------------------------------------------------------------------

  let queryElement: HTMLElement;
  let schemaElement: HTMLElement;
  let sqlElement: HTMLElement;
  let metadataElement: HTMLElement;

  let queryEditor = $state.raw<QueryEditor>();
  let schemaEditor = $state.raw<Editor>();
  let sqlEditor = $state.raw<Editor>();
  let metadataEditor = $state.raw<Editor>();

  onMount(() => {
    let editors: Editor[] = [];
    void readUrl().then(() => {
      const initial = { theme: theme.current };
      queryEditor = createQueryEditor({
        ...initial,
        parent: queryElement,
        text: query,
        onChange: (text) => (query = text),
      });
      schemaEditor = createJsonEditor({
        ...initial,
        parent: schemaElement,
        text: schema,
        label: 'Schema',
        onChange: (text) => (schema = text),
      });
      sqlEditor = createSqlEditor({ ...initial, parent: sqlElement, text: sql });
      metadataEditor = createJsonEditor({
        ...initial,
        parent: metadataElement,
        text: metadata,
        label: 'Metadata',
      });
      editors = [queryEditor, schemaEditor, sqlEditor, metadataEditor];
      ready = true;
    });
    return () => editors.forEach((editor) => editor.view.destroy());
  });

  // The state is the source of truth, and the editors follow it. (An edit in an editor changes the
  // state, which then has nothing left to change in the editor.)
  $effect(() => queryEditor?.setText(query));
  $effect(() => schemaEditor?.setText(schema));
  $effect(() => sqlEditor?.setText(sql));
  $effect(() => metadataEditor?.setText(metadata));
  $effect(() => queryEditor?.setDiagnostics(diagnostics));

  $effect(() => {
    const current = theme.current;
    for (const editor of [queryEditor, schemaEditor, sqlEditor, metadataEditor]) {
      editor?.setTheme(current);
    }
  });

  $effect(() => {
    // Complete names from the schema, for as long as the schema is valid.
    if (!queryEditor) return;
    let completer: QuerydownCompleter | null = null;
    try {
      completer = new QuerydownCompleter(schema);
    } catch {
      // The schema is being edited. Its error shows in the output.
    }
    queryEditor.setCompleter(completer);
    return () => completer?.free();
  });
</script>

<svelte:window onhashchange={() => void readUrl()} />

<div class="playground">
  <div class="toolbar">
    <label>
      <span>Dialect</span>
      <select bind:value={dialect}>
        {#each DIALECTS as option (option)}
          <option value={option}>{DIALECT_NAMES[option]}</option>
        {/each}
      </select>
    </label>
    <label>
      <span>Limit</span>
      <input type="number" min="0" step="1" placeholder="none" bind:value={limit} />
    </label>
    <label>
      <span>Offset</span>
      <input type="number" min="0" step="1" placeholder="none" bind:value={offset} />
    </label>
    <div class="actions">
      <button type="button" onclick={reset}>Reset</button>
      <button type="button" onclick={copyLink}>{copied ? 'Copied' : 'Copy link'}</button>
    </div>
  </div>

  <div class="panes">
    <section class="pane" aria-label="Input">
      <div class="tabs" role="tablist" aria-label="Input">
        <button
          type="button"
          role="tab"
          aria-selected={inputTab === 'query'}
          onclick={() => (inputTab = 'query')}
        >
          Querydown
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={inputTab === 'schema'}
          onclick={() => (inputTab = 'schema')}
        >
          Schema{customSchema ? ' (edited)' : ''}
        </button>
        {#if inputTab === 'schema'}
          <span class="note">
            {#if customSchema}
              <button type="button" class="link" onclick={() => (schema = sampleSchema)}>
                Restore the sample schema
              </button>
            {:else}
              The <a href="/docs/sample-schema">sample schema</a>. Edit it, or paste your own.
            {/if}
          </span>
        {/if}
      </div>
      <div class="editor" hidden={inputTab !== 'query'} bind:this={queryElement}></div>
      <div class="editor" hidden={inputTab !== 'schema'} bind:this={schemaElement}></div>
    </section>

    <section class="pane" aria-label="Output">
      <div class="tabs" role="tablist" aria-label="Output">
        <button
          type="button"
          role="tab"
          aria-selected={outputTab === 'sql'}
          onclick={() => (outputTab = 'sql')}
        >
          SQL
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={outputTab === 'metadata'}
          onclick={() => (outputTab = 'metadata')}
        >
          Metadata
        </button>
        {#if outputTab === 'metadata' && !outcome.error}
          <span class="note">
            The <a href="/docs/language#annotations">annotations</a> written in the query.
          </span>
        {/if}
      </div>
      {#if outcome.error}
        {@const error = outcome.error}
        <div class="error" role="alert">
          <p>
            <strong>{error.kind} error</strong>
            {#if error.span}
              <span class="where">
                line {error.span.start.line}, column {error.span.start.column}
              </span>
            {/if}
          </p>
          <p>{error.reason}</p>
          {#if error.hint}<p class="hint">{error.hint}</p>{/if}
        </div>
      {/if}
      <div
        class="editor"
        hidden={outputTab !== 'sql' || outcome.error !== null}
        bind:this={sqlElement}
      ></div>
      <div
        class="editor"
        hidden={outputTab !== 'metadata' || outcome.error !== null}
        bind:this={metadataElement}
      ></div>
    </section>
  </div>
</div>

<style>
  .playground {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .toolbar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem 1rem;
    padding: 0.5rem var(--gutter);
    border-bottom: 1px solid var(--border);
    background: var(--bg-subtle);
    font-size: 0.875rem;
  }

  label {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    color: var(--text-muted);
  }

  select,
  input,
  .actions button {
    height: 1.9rem;
    padding: 0 0.5rem;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    background: var(--bg);
    color: var(--text);
  }

  input {
    width: 5rem;
  }

  .actions {
    display: flex;
    gap: 0.5rem;
    margin-left: auto;
  }

  .actions button {
    cursor: pointer;
  }

  .actions button:hover {
    background: var(--bg-inset);
  }

  .panes {
    flex: 1;
    display: grid;
    grid-template-rows: minmax(0, 1fr) minmax(0, 1fr);
    min-height: 0;
  }

  .pane {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }

  .pane + .pane {
    border-top: 1px solid var(--border-strong);
  }

  .tabs {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0 calc(var(--gutter) - 0.6rem);
    border-bottom: 1px solid var(--border);
    font-size: 0.875rem;
  }

  .tabs [role='tab'] {
    padding: 0.5rem 0.6rem;
    border: 0;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
    background: none;
    color: var(--text-muted);
    cursor: pointer;
  }

  .tabs [role='tab']:hover {
    color: var(--text);
  }

  .tabs [role='tab'][aria-selected='true'] {
    border-bottom-color: var(--accent);
    color: var(--text);
    font-weight: 600;
  }

  .note {
    margin-left: auto;
    padding-right: 0.6rem;
    color: var(--text-muted);
    font-size: 0.8125rem;
    text-align: right;
  }

  .link {
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent);
    cursor: pointer;
  }

  .link:hover {
    text-decoration: underline;
  }

  .editor {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .editor[hidden] {
    display: none;
  }

  .error {
    margin: 1rem var(--gutter);
    padding: 0.75rem 1rem;
    border: 1px solid color-mix(in srgb, var(--danger) 40%, var(--border));
    border-radius: var(--radius);
    background: var(--danger-subtle);
    overflow-wrap: anywhere;
  }

  .error p {
    margin: 0;
  }

  .error p + p {
    margin-top: 0.4rem;
  }

  .error strong {
    color: var(--danger);
    text-transform: capitalize;
  }

  .where,
  .hint {
    color: var(--text-muted);
  }

  .where {
    margin-left: 0.5rem;
    font-size: 0.875rem;
  }

  @media (min-width: 56rem) {
    .panes {
      grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
      grid-template-rows: none;
    }

    .pane + .pane {
      border-top: 0;
      border-left: 1px solid var(--border-strong);
    }
  }

  /* Colors for the SQL and JSON, whose tokens CodeMirror marks with these classes. */
  .playground :global(:is(.tok-keyword, .tok-operatorKeyword)) {
    color: light-dark(#cf222e, #ff7b72);
  }

  .playground :global(:is(.tok-string, .tok-string2)) {
    color: light-dark(#0a3069, #a5d6ff);
  }

  .playground :global(:is(.tok-number, .tok-bool, .tok-atom, .tok-literal)) {
    color: light-dark(#0550ae, #79c0ff);
  }

  .playground :global(.tok-propertyName) {
    color: light-dark(#116329, #7ee787);
  }

  .playground :global(.tok-comment) {
    color: var(--text-muted);
    font-style: italic;
  }

  .playground :global(:is(.tok-punctuation, .tok-operator)) {
    color: var(--text-muted);
  }
</style>
