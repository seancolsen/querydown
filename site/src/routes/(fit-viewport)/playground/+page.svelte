<script lang="ts">
  import { onMount } from 'svelte';
  import Playground from './Playground.svelte';
  import type { editor as MonacoEditor } from 'monaco-editor';
  import Spinner from '../../../icons/Spinner.svelte';
  import { monarch } from './monarch';

  let compile: ((schema_json: string, dialect: string, input: string) => string) | undefined;
  let default_schema_json: string | undefined;
  let monaco_editor_create: typeof MonacoEditor.create | undefined;

  async function init_compile() {
    const qd = await import('querydown-js');
    compile = qd.compile;
  }

  async function init_default_schema_json() {
    const schema_response = await fetch('/issue_schema.json');
    default_schema_json = await schema_response.text();
  }

  async function init_monaco_editor_create() {
    const monaco = await import('monaco-editor');
    monaco.languages.register({ id: 'qd' });
    monaco.languages.setMonarchTokensProvider('qd', monarch);
    monaco.editor.defineTheme('qd', {
      base: 'vs',
      colors: {},
      inherit: true,
      rules: [
        { token: 'base-table', foreground: '287f90', fontStyle: 'bold underline' },
        { token: 'table-with-many', foreground: '287f90' },
        { token: 'column', foreground: '1e1f63' },
        { token: 'column-control', foreground: '737373', fontStyle: 'bold' },
        { token: 'column-prefix', foreground: 'c963e3', fontStyle: 'bold' },
        { token: 'duration', foreground: 'a63f87' },
        { token: 'date', foreground: 'a63f87' },
        { token: 'qd-number', foreground: 'a66565' },
        { token: 'comparison-operator', foreground: '000be3', fontStyle: 'bold' },
        { token: 'has', foreground: '787113', fontStyle: 'bold' },
        { token: 'scalar-pipe', foreground: '787113', fontStyle: 'bold' },
        { token: 'scalar-function', foreground: '787113' },
        { token: 'aggregate-pipe', foreground: 'b1ad70', fontStyle: 'bold' },
        { token: 'aggregate-function', foreground: '787113', fontStyle: 'bold' },
        { token: 'invalid', foreground: 'ffa1a6', fontStyle: 'strikethrough' },
        { token: 'alias-prefix', foreground: 'a5a5a5' },
        { token: 'alias', foreground: '7e52a5', fontStyle: 'italic' },
      ],
    });
    monaco.editor.setTheme('qd');
    monaco_editor_create = monaco.editor.create;
  }

  onMount(() => {
    void init_compile();
    void init_default_schema_json();
    void init_monaco_editor_create();
    // TODO handle loading errors
  });
</script>

{#if compile && default_schema_json && monaco_editor_create}
  <Playground {compile} {default_schema_json} {monaco_editor_create} />
{:else}
  <div class="spinner">
    <Spinner />
  </div>
{/if}

<style>
  .spinner {
    display: flex;
    justify-content: center;
    align-items: center;
    height: 100%;
    color: #ccc;
    font-size: 3rem;
  }
</style>
