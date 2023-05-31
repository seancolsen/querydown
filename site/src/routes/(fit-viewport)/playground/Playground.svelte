<script lang="ts">
  import { onMount } from 'svelte';
  import type { editor as MonacoEditor } from 'monaco-editor';

  import { starting_querydown } from './constants';
  import TabArea from './TabArea.svelte';
  import Tab from './Tab.svelte';
  import EditorGroup from './EditorGroup.svelte';

  export let compile: (schema_json: string, dialect: string, input: string) => string;
  /**
   * Note: This component does not react to changes to this prop from the
   * parent after mount. With more work, we could improve this if needed.
   */
  export let default_schema_json: string;
  export let monaco_editor_create: typeof MonacoEditor.create;

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

    function handle_change() {
      let sql = '';
      try {
        sql = compile(schema_code, 'postgres', qd_code);
      } catch (e) {
        sql = `-- ${String(e)}`;
      }
      sql_editor.getModel()?.setValue(sql);
    }

    handle_change();

    const qd_model = qd_editor.getModel();
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
