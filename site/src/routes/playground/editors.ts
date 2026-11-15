/**
 * The playground's CodeMirror editors.
 *
 * This module (and CodeMirror with it) is only loaded in the browser, once the playground opens.
 */

import { autocompletion } from '@codemirror/autocomplete';
import { json } from '@codemirror/lang-json';
import { sql } from '@codemirror/lang-sql';
import { syntaxHighlighting } from '@codemirror/language';
import { setDiagnostics, type Diagnostic } from '@codemirror/lint';
import { Compartment, EditorState, type Extension } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { classHighlighter } from '@lezer/highlight';
import { basicSetup } from 'codemirror';
import { querydown } from 'querydown-codemirror';
import { tokenize, type QuerydownCompleter } from 'querydown-js';

import type { Theme } from '#lib/theme.svelte.ts';

/**
 * The editors' appearance. Its colors are the site's CSS variables, so they follow the site's
 * theme by themselves. But CodeMirror also needs to be told whether the theme is dark, because
 * that's how Querydown's highlighting chooses between its light and dark palettes.
 */
function appearance(theme: Theme): Extension {
  return EditorView.theme(
    {
      '&': { height: '100%', color: 'var(--text)', backgroundColor: 'var(--bg)' },
      '&.cm-focused': { outline: 'none' },
      '.cm-scroller': {
        fontFamily: 'var(--font-mono)',
        fontSize: '0.875rem',
        lineHeight: '1.5',
        fontVariantLigatures: 'none',
      },
      '.cm-content': { caretColor: 'var(--text)', padding: '0.6rem 0' },
      '.cm-cursor': { borderLeftColor: 'var(--text)' },
      '.cm-gutters': {
        color: 'var(--text-muted)',
        backgroundColor: 'var(--bg)',
        borderRight: '1px solid var(--border)',
      },
      '.cm-activeLine': { backgroundColor: 'transparent' },
      '&.cm-focused .cm-activeLine': {
        backgroundColor: 'color-mix(in srgb, var(--bg-inset) 55%, transparent)',
      },
      '.cm-activeLineGutter': { color: 'var(--text)', backgroundColor: 'transparent' },
      '&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-selectionBackground':
        { backgroundColor: 'color-mix(in srgb, var(--accent) 25%, transparent)' },
      '.cm-tooltip': {
        color: 'var(--text)',
        backgroundColor: 'var(--bg)',
        border: '1px solid var(--border-strong)',
        borderRadius: '6px',
        boxShadow: '0 0.5rem 1.5rem var(--shadow)',
      },
      '.cm-tooltip-autocomplete ul li[aria-selected]': {
        color: 'var(--text)',
        backgroundColor: 'var(--accent-subtle)',
      },
      '.cm-completionDetail': { color: 'var(--text-muted)', fontStyle: 'normal' },
      '.cm-foldPlaceholder': {
        color: 'var(--text-muted)',
        backgroundColor: 'var(--bg-inset)',
        border: 'none',
      },
    },
    { dark: theme === 'dark' },
  );
}

/** An editor whose appearance can be switched between the light and dark themes. */
export interface Editor {
  view: EditorView;
  setTheme(theme: Theme): void;
  /** Replaces the document, unless it already holds `text`. */
  setText(text: string): void;
}

interface EditorConfig {
  parent: HTMLElement;
  text: string;
  theme: Theme;
  extensions: Extension[];
  /** Called with the new text whenever it changes. Without this, the editor is read-only. */
  onChange?: (text: string) => void;
  label: string;
}

function createEditor(config: EditorConfig): Editor {
  const themeCompartment = new Compartment();
  const { onChange } = config;
  const view = new EditorView({
    parent: config.parent,
    state: EditorState.create({
      doc: config.text,
      extensions: [
        basicSetup,
        EditorView.lineWrapping,
        EditorView.contentAttributes.of({ 'aria-label': config.label }),
        themeCompartment.of(appearance(config.theme)),
        onChange
          ? EditorView.updateListener.of((update) => {
              if (update.docChanged) onChange(update.state.doc.toString());
            })
          : [EditorState.readOnly.of(true), EditorView.editable.of(false)],
        config.extensions,
      ],
    }),
  });
  return {
    view,
    setTheme: (theme) =>
      view.dispatch({ effects: themeCompartment.reconfigure(appearance(theme)) }),
    setText: (text) => {
      if (view.state.doc.toString() === text) return;
      view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: text } });
    },
  };
}

type EditorOptions = Pick<EditorConfig, 'parent' | 'text' | 'theme'>;

/**
 * Highlighting for the languages other than Querydown. Tokens get classes like `tok-keyword`, which
 * the playground styles with the site's colors.
 */
const classes = syntaxHighlighting(classHighlighter);

/** The editor for the query, with Querydown's highlighting and autocompletion. */
export interface QueryEditor extends Editor {
  /** Completes names from another schema. Without a completer, there's no completion. */
  setCompleter(completer: QuerydownCompleter | null): void;
  /** Underlines the part of the query that an error is about. */
  setDiagnostics(diagnostics: Diagnostic[]): void;
}

export function createQueryEditor(
  options: EditorOptions & { onChange: (text: string) => void },
): QueryEditor {
  const language = new Compartment();
  const support = (completer: QuerydownCompleter | null) =>
    querydown({ tokenize, completion: completer ? { completer } : undefined });
  const editor = createEditor({
    ...options,
    label: 'Querydown',
    extensions: [language.of(support(null)), autocompletion()],
  });
  return {
    ...editor,
    setCompleter: (completer) =>
      editor.view.dispatch({ effects: language.reconfigure(support(completer)) }),
    setDiagnostics: (diagnostics) =>
      editor.view.dispatch(setDiagnostics(editor.view.state, diagnostics)),
  };
}

export function createJsonEditor(
  options: EditorOptions & { label: string; onChange?: (text: string) => void },
): Editor {
  return createEditor({ ...options, extensions: [json(), classes] });
}

export function createSqlEditor(options: EditorOptions): Editor {
  return createEditor({ ...options, label: 'SQL', extensions: [sql(), classes] });
}
