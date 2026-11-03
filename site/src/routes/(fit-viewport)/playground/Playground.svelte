<script lang="ts">
  import { onMount } from 'svelte';
  import type { editor as MonacoEditor } from 'monaco-editor';

  import { starting_querydown } from './constants';
  import TabArea from './TabArea.svelte';
  import Tab from './Tab.svelte';
  import EditorGroup from './EditorGroup.svelte';

  export let compile: typeof import('querydown-js').compile;
  export let is_querydown_error: typeof import('querydown-js').is_querydown_error;
  /**
   * Note: This component does not react to changes to this prop from the
   * parent after mount. With more work, we could improve this if needed.
   */
  export let default_schema_json: string;
  export let monaco_editor_create: typeof MonacoEditor.create;
  export let monaco_set_model_markers: typeof MonacoEditor.setModelMarkers;

  /** Formats (possibly multi-line) text as SQL line comments. */
  function sql_comment(text: string): string {
    return text
      .split('\n')
      .map((line) => `-- ${line}`)
      .join('\n');
  }

  let schema_editor_element: HTMLDivElement;
  let qd_editor_element: HTMLDivElement;
  let sql_editor_element: HTMLDivElement;
  let active_input: Element;
  let active_output: Element;

  onMount(() => {
    active_input = qd_editor_element;
    active_output = sql_editor_element;
    const common_options: MonacoEditor.IStandaloneEditorConstructionOptions = {
      automaticLayout: true,
      minimap: {
        enabled: false,
      },
    };
    const schema_editor = monaco_editor_create(schema_editor_element, {
      ...common_options,
      value: default_schema_json,
      // TODO: figure out how to enable 'json' without getting errors.
      language: 'text',
    });
    const qd_editor = monaco_editor_create(qd_editor_element, {
      ...common_options,
      value: starting_querydown,
      language: 'qd',
    });
    const sql_editor = monaco_editor_create(sql_editor_element, {
      ...common_options,
      value: '',
      readOnly: true,
      language: 'sql',
    });

    let qd_code = starting_querydown;
    let schema_code = default_schema_json;

    const qd_model = qd_editor.getModel();

    function handle_change() {
      let sql = '';
      let markers: MonacoEditor.IMarkerData[] = [];
      try {
        // `compile` also returns `columnAnnotations`, but the playground only displays the SQL.
        sql = compile(schema_code, 'postgres', qd_code).sql;
      } catch (e) {
        if (is_querydown_error(e)) {
          sql = sql_comment(e.hint ? `${e.message}\nHint: ${e.hint}` : e.message);
          // Only errors in the Querydown source (currently, syntax errors) have a span. Schema
          // errors don't, so there's nothing to mark in the schema editor.
          if (e.span) {
            const { start, end } = e.span;
            const is_empty = start.offset === end.offset;
            markers = [
              {
                severity: 8, // MarkerSeverity.Error (a value import would load Monaco eagerly)
                message: e.hint ? `${e.reason}\n\nHint: ${e.hint}` : e.reason,
                startLineNumber: start.line,
                startColumn: start.column,
                endLineNumber: end.line,
                // Make an empty span (e.g. at the end of input) at least one character wide.
                endColumn: is_empty ? end.column + 1 : end.column,
              },
            ];
          }
        } else {
          sql = sql_comment(String(e));
        }
      }
      sql_editor.getModel()?.setValue(sql);
      if (qd_model) {
        monaco_set_model_markers(qd_model, 'querydown', markers);
      }
    }

    handle_change();

    qd_editor.onDidChangeModelContent(() => {
      if (qd_model) {
        qd_code = qd_model.getValue();
        handle_change();
      }
    });

    const schema_model = schema_editor.getModel();
    schema_editor.onDidChangeModelContent(() => {
      if (schema_model) {
        schema_code = schema_model.getValue();
        handle_change();
      }
    });
  });
</script>

<div class="editors">
  <EditorGroup>
    <svelte:fragment slot="tabs">
      <Tab tab={schema_editor_element} bind:active={active_input}>schema.json</Tab>
      <Tab tab={qd_editor_element} bind:active={active_input}>input.qd</Tab>
    </svelte:fragment>
    <TabArea bind:active={active_input}>
      <div bind:this={schema_editor_element} />
      <div bind:this={qd_editor_element} />
    </TabArea>
  </EditorGroup>

  <EditorGroup>
    <svelte:fragment slot="tabs">
      <Tab tab={sql_editor_element} bind:active={active_output}>output.sql</Tab>
    </svelte:fragment>
    <TabArea bind:active={active_output}>
      <div bind:this={sql_editor_element} />
    </TabArea>
  </EditorGroup>
</div>

<style>
  .editors {
    display: grid;
    grid-template: auto / 1fr 1fr;
    height: 100%;
    gap: 0.5rem;
    overflow: hidden;
  }
</style>
