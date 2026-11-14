/**
 * Syntax highlighting, done on the server so that pages need no JavaScript to show code.
 *
 * Querydown is highlighted by its own parser (the bindings' `tokenize`, the same function the
 * CodeMirror package uses), and colored by the palettes of the CodeMirror package, so an example
 * on a docs page looks the same as it does in the playground's editor. Other languages are
 * highlighted by Shiki.
 */

import { createHighlighter, type Highlighter } from 'shiki';

import { darkPalette, lightPalette } from '../../../../bindings/codemirror/src/highlight';
import type { TokenStyle } from '../../../../bindings/codemirror/src/highlight';
import { tokenize } from './querydown';

export function escapeHtml(text: string): string {
  return text
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;');
}

/** Highlights Querydown source as HTML, with one `<span class="qd-KIND">` for each token. */
export function highlightQuerydown(source: string): string {
  let html = '';
  let position = 0;
  for (const token of tokenize(source, 'query')) {
    html += escapeHtml(source.slice(position, token.from));
    html += `<span class="qd-${token.kind}">${escapeHtml(source.slice(token.from, token.to))}</span>`;
    position = token.to;
  }
  return html + escapeHtml(source.slice(position));
}

const CSS_PROPERTIES: Record<keyof TokenStyle, string> = {
  color: 'color',
  backgroundColor: 'background-color',
  fontWeight: 'font-weight',
  fontStyle: 'font-style',
  textDecoration: 'text-decoration',
};

function paletteRules(palette: Record<string, string | TokenStyle>, scope: string): string {
  return Object.entries(palette)
    .map(([kind, style]) => {
      const properties = typeof style === 'string' ? { color: style } : style;
      const declarations = Object.entries(properties)
        .map(([name, value]) => `${CSS_PROPERTIES[name as keyof TokenStyle]}:${value}`)
        .join(';');
      return `${scope} .qd-${kind}{${declarations}}`;
    })
    .join('\n');
}

/**
 * The stylesheet that colors highlighted Querydown, generated from the CodeMirror package's
 * palettes. The dark palette applies under the same conditions as the site's dark theme (see
 * `app.css`).
 */
export function querydownThemeCss(): string {
  return [
    paletteRules(lightPalette, ':root'),
    '@media (prefers-color-scheme: dark) {',
    paletteRules(darkPalette, ':root:not([data-theme="light"])'),
    '}',
    paletteRules(darkPalette, ':root[data-theme="dark"]'),
    '',
  ].join('\n');
}

const LANGUAGES = ['sql', 'rust', 'ts', 'js', 'json', 'sh', 'toml', 'html', 'css', 'svelte'];
const ALIASES: Record<string, string> = {
  typescript: 'ts',
  javascript: 'js',
  bash: 'sh',
  shell: 'sh',
  console: 'sh',
};

let highlighter: Promise<Highlighter> | undefined;

export function loadHighlighter(): Promise<Highlighter> {
  highlighter ??= createHighlighter({ themes: ['github-light', 'github-dark'], langs: LANGUAGES });
  return highlighter;
}

/**
 * Highlights source in a language other than Querydown, as the HTML of a `<pre>` element. Each
 * token carries both its light and its dark color, as custom properties that `app.css` chooses
 * between. A language we don't highlight comes out as plain text.
 */
export function highlightCode(shiki: Highlighter, source: string, language: string | null): string {
  const name = ALIASES[language ?? ''] ?? language ?? 'text';
  return shiki.codeToHtml(source, {
    lang: LANGUAGES.includes(name) ? name : 'text',
    themes: { light: 'github-light', dark: 'github-dark' },
    defaultColor: false,
  });
}
