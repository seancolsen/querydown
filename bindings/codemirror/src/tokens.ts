/**
 * The kinds of token that the Querydown bindings' `tokenize` function produces, mirroring
 * `QuerydownTokenKind` in the bindings' TypeScript declarations. See the bindings' documentation for
 * what each one means.
 */
export const TOKEN_KINDS = [
  "comment",
  "string",
  "string-escape",
  "search-term",
  "number",
  "duration",
  "date",
  "constant",
  "variable",
  "base-table",
  "table",
  "column",
  "function",
  "aggregate-function",
  "pipe",
  "comparison-operator",
  "operator",
  "has-quantity",
  "glob",
  "case",
  "path-separator",
  "result-column",
  "sort",
  "flag",
  "alias",
  "annotation",
  "annotation-key",
  "annotation-value",
  "annotation-bracket",
  "pipeline",
  "definition",
  "bracket",
  "punctuation",
  "invalid",
] as const;

/** The role a span of Querydown source plays, for syntax highlighting. */
export type TokenKind = (typeof TOKEN_KINDS)[number];

/** A classified span of source, with offsets in UTF-16 code units (like JavaScript strings). */
export interface Token {
  from: number;
  to: number;
  /** One of {@link TOKEN_KINDS}, or (from a newer version of the bindings) a kind not known here. */
  kind: string;
}

/**
 * Which grammar to read source with: a whole query, or one section of a sectioned query (as passed
 * to the bindings' `compile_sections`).
 */
export type Grammar = "query" | "definitions" | "conditions" | "sorting" | "display";

/**
 * The signature of the `tokenize` function exported by the Querydown JavaScript bindings
 * (`querydown-js`). It is passed in, rather than imported, so that this package works however the
 * bindings were built and initialized.
 */
export type Tokenize = (source: string, grammar: Grammar) => Token[];
