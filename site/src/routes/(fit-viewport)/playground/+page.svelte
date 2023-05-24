<script lang="ts">
  import { onMount } from 'svelte';
  import Playground from './Playground.svelte';
  import type { editor as MonacoEditor } from 'monaco-editor';
  import Spinner from '../../../icons/Spinner.svelte';

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
