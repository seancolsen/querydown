<script lang="ts">
  import { onMount } from 'svelte';

  let editorElement: HTMLDivElement;
  let content = [
    'issues',
    'created_at:>@6M|ago',
    '--#assignments',
    '++#labels{name:["Regression" "Bug"]}',
    '#comments{user.team.name!"Backend"}:10',
    '$author.username',
    '$#comments.created_at%min \\sd',
  ].join('\n');
  let sql = "";
  let compile: ((input: string) => string) | undefined;

  function handle_change(input: string) {
    if (compile) {
      sql = compile(input);
    }
  }

  onMount(async () => {
    const qd = await import('querydown-js');

    // TODO figure out how to store this schema in one place instead of copying it
    const schema_response = await fetch('/issue_schema.json');
    const schema_str = await schema_response.text();
    compile = input => qd.compile(schema_str, "postgres", input);
    
    const monaco = await import('monaco-editor');
    const editor = monaco.editor.create(editorElement, {
      value: content,
      language: 'text',
      automaticLayout: true,
      minimap: {
        enabled: false,
      },
    });
    const model = editor.getModel();
    editor.onDidChangeModelContent(() => {
      if (model) {
        content = model.getValue();
        handle_change(content);
      }
    });
  });
</script>

<h1>Querydown Playground</h1>

<div style="height: 300px" bind:this={editorElement} />

<pre>{sql}</pre>
