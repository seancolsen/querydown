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

  onMount(async () => {
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
      }
    });
  });
</script>

<h1>Querydown Playground</h1>

<div style="height: 300px" bind:this={editorElement} />

<pre>{content}</pre>
