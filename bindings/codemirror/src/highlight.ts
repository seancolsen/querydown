import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import type { Extension } from "@codemirror/state";

import { querydownTags } from "./tags.js";
import { TOKEN_KINDS, type TokenKind } from "./tokens.js";

/** How to style one kind of token. Each property is a CSS value, e.g. `fontWeight: "bold"`. */
export interface TokenStyle {
  color?: string;
  backgroundColor?: string;
  fontWeight?: string;
  fontStyle?: string;
  textDecoration?: string;
}

/**
 * Styles for some or all kinds of Querydown token. An entry may be a full {@link TokenStyle} or just
 * a color (any CSS color, including `var(--my-color)`). A missing or `null` entry leaves that kind
 * of token unstyled, so it takes the editor's text color.
 */
export type QuerydownPalette = { readonly [K in TokenKind]?: string | TokenStyle | null };

/** The default palette for editors with a light theme. */
export const lightPalette = {
  comment: { color: "#6e7781", fontStyle: "italic" },
  string: "#a3412b",
  "string-escape": { color: "#c4510f", fontWeight: "bold" },
  "search-term": { color: "#a3412b", backgroundColor: "#a3412b14" },
  number: "#1f7a3a",
  duration: "#1f7a3a",
  date: "#1f7a3a",
  constant: { color: "#1f7a3a", fontWeight: "bold" },
  variable: "#b35900",
  "base-table": { color: "#0b7285", fontWeight: "bold" },
  table: "#0b7285",
  column: "#3b3fb0",
  function: "#686300",
  "aggregate-function": { color: "#686300", fontWeight: "bold" },
  pipe: "#9a8f0b",
  "comparison-operator": { color: "#c2185b", fontWeight: "bold" },
  operator: "#57606a",
  "has-quantity": { color: "#c2185b", fontWeight: "bold" },
  glob: { color: "#8250df", fontWeight: "bold" },
  case: { color: "#c2185b", fontWeight: "bold" },
  "path-separator": "#8c959f",
  "result-column": { color: "#8250df", fontWeight: "bold" },
  sort: { color: "#8250df", fontWeight: "bold" },
  flag: "#8250df",
  alias: { color: "#8a3fb3", fontStyle: "italic" },
  annotation: "#6e7781",
  "annotation-key": "#0b7285",
  pipeline: { color: "#c2185b", fontWeight: "bold" },
  definition: "#c2185b",
  bracket: "#57606a",
  punctuation: "#57606a",
  invalid: { color: "#cf222e", textDecoration: "underline wavy" },
} as const satisfies Required<QuerydownPalette>;

/** The default palette for editors with a dark theme. */
export const darkPalette = {
  comment: { color: "#8b949e", fontStyle: "italic" },
  string: "#ff9e9e",
  "string-escape": { color: "#ffcb6b", fontWeight: "bold" },
  "search-term": { color: "#ff9e9e", backgroundColor: "#ff9e9e1f" },
  number: "#7ee787",
  duration: "#7ee787",
  date: "#7ee787",
  constant: { color: "#7ee787", fontWeight: "bold" },
  variable: "#ffa657",
  "base-table": { color: "#56d4dd", fontWeight: "bold" },
  table: "#56d4dd",
  column: "#9db4ff",
  function: "#e6db74",
  "aggregate-function": { color: "#e6db74", fontWeight: "bold" },
  pipe: "#c4b94f",
  "comparison-operator": { color: "#ff7eb6", fontWeight: "bold" },
  operator: "#9198a1",
  "has-quantity": { color: "#ff7eb6", fontWeight: "bold" },
  glob: { color: "#d2a8ff", fontWeight: "bold" },
  case: { color: "#ff7eb6", fontWeight: "bold" },
  "path-separator": "#6e7681",
  "result-column": { color: "#d2a8ff", fontWeight: "bold" },
  sort: { color: "#d2a8ff", fontWeight: "bold" },
  flag: "#d2a8ff",
  alias: { color: "#d2a8ff", fontStyle: "italic" },
  annotation: "#8b949e",
  "annotation-key": "#56d4dd",
  pipeline: { color: "#ff7eb6", fontWeight: "bold" },
  definition: "#ff7eb6",
  bracket: "#9198a1",
  punctuation: "#9198a1",
  invalid: { color: "#ff7b72", textDecoration: "underline wavy" },
} as const satisfies Required<QuerydownPalette>;

export interface QuerydownHighlightStyleOptions {
  /**
   * When set, the style only applies in editors with a matching theme, as determined by
   * CodeMirror's `EditorView.darkTheme` facet (which themes set via `EditorView.theme(…, { dark })`).
   */
  themeType?: "light" | "dark";
}

/** Builds a `HighlightStyle` that styles Querydown's tokens according to `palette`. */
export function querydownHighlightStyle(
  palette: QuerydownPalette,
  options: QuerydownHighlightStyleOptions = {},
): HighlightStyle {
  const specs = TOKEN_KINDS.flatMap((kind) => {
    const style = palette[kind];
    if (style === undefined || style === null) return [];
    const properties = typeof style === "string" ? { color: style } : style;
    return [{ tag: querydownTags[kind], ...properties }];
  });
  return HighlightStyle.define(specs, options);
}

export interface QuerydownHighlightingConfig {
  /** Overrides for some kinds of token in light themes, applied on top of {@link lightPalette}. */
  light?: QuerydownPalette;
  /** Overrides for some kinds of token in dark themes, applied on top of {@link darkPalette}. */
  dark?: QuerydownPalette;
}

/**
 * Syntax highlighting for Querydown that follows the editor's theme: {@link lightPalette} in editors
 * with a light theme and {@link darkPalette} in editors with a dark one. Override the colors of
 * particular kinds of token with `config`, e.g.
 *
 * ```ts
 * querydownHighlighting({ light: { column: "#005cc5" }, dark: { column: "var(--my-column)" } })
 * ```
 */
export function querydownHighlighting(config: QuerydownHighlightingConfig = {}): Extension {
  return [
    syntaxHighlighting(
      querydownHighlightStyle({ ...lightPalette, ...config.light }, { themeType: "light" }),
    ),
    syntaxHighlighting(
      querydownHighlightStyle({ ...darkPalette, ...config.dark }, { themeType: "dark" }),
    ),
  ];
}
