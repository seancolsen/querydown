import {
  insertCompletionText,
  pickedCompletion,
  startCompletion,
  type Completion,
  type CompletionContext,
  type CompletionResult,
  type CompletionSource,
} from "@codemirror/autocomplete";
import type { EditorView } from "@codemirror/view";

import type { Grammar } from "./tokens.js";

// ------------------------------------------------------------------------------------------------
// The bindings' completer
// ------------------------------------------------------------------------------------------------
//
// These types mirror those of the Querydown JavaScript bindings (`querydown-js`), whose
// `QuerydownCompleter` is passed in rather than imported, so that this package works however the
// bindings were built and initialized.

/**
 * The kinds of item that the bindings' completer offers, mirroring `QuerydownCompletionKind` in the
 * bindings' TypeScript declarations.
 */
export const COMPLETION_KINDS = [
  "table",
  "column",
  "computed-column",
  "custom-comparison",
  "function",
  "aggregate-function",
  "window-function",
  "constant",
  "variable",
] as const;

export type CompletionKind = (typeof COMPLETION_KINDS)[number];

/** Something that could be typed, as offered by the bindings' completer. */
export interface CompletionItem {
  label: string;
  /** One of {@link COMPLETION_KINDS}, or (from a newer version of the bindings) a kind not known here. */
  kind: string;
  insert: string | null;
  detail: string | null;
  info: string | null;
  boost: number;
}

/** Where the cursor is within a column annotation (`@{…}`). */
export interface AnnotationPosition {
  /**
   * The keys and array indices leading from the top-level annotation object to the object or array
   * the cursor is in. When a value is being typed, the last element is its key or index.
   */
  path: (string | number)[];
  /** Whether a key or a value is being typed. */
  slot: "key" | "value";
  /** When a key is being typed, the keys its object already has. */
  keys: string[];
}

/** What the bindings' completer returns. Offsets are UTF-16, like CodeMirror's positions. */
export interface Completions {
  from: number;
  to: number;
  items: CompletionItem[];
  annotation: AnnotationPosition | null;
}

/** The options accepted by the bindings' completer. */
export interface CompleterOptions {
  grammar?: Grammar;
  baseTable?: string;
  definitions?: string;
}

/**
 * The completer from the Querydown JavaScript bindings: a `QuerydownCompleter`, created with your
 * database's schema (`new QuerydownCompleter(schemaJson)`).
 */
export interface Completer {
  complete(source: string, cursor: number, options?: CompleterOptions): Completions;
}

// ------------------------------------------------------------------------------------------------
// Annotation schemas
// ------------------------------------------------------------------------------------------------

/**
 * A JSON Schema describing the column annotations your application accepts, used to complete their
 * keys and values. The supported keywords are listed here; others are ignored.
 *
 * ```ts
 * const annotations: AnnotationSchema = {
 *   type: "object",
 *   properties: {
 *     width: { type: "number", description: "The column's width in pixels" },
 *     align: { enum: ["left", "center", "right"] },
 *   },
 * };
 * ```
 */
export interface AnnotationSchema {
  type?: string | string[];
  title?: string;
  description?: string;
  /** An object's keys, and the schema of each one's value. */
  properties?: Record<string, AnnotationSchema>;
  /** The schema of the values of keys not in `properties`. */
  additionalProperties?: boolean | AnnotationSchema;
  /** The schema of an array's elements (after any `prefixItems`). */
  items?: boolean | AnnotationSchema;
  /** The schemas of an array's first elements, by position. */
  prefixItems?: AnnotationSchema[];
  /** The values allowed. */
  enum?: readonly unknown[];
  /** The one value allowed. */
  const?: unknown;
  /** Alternatives, whose keys and values are all offered. */
  anyOf?: AnnotationSchema[];
  oneOf?: AnnotationSchema[];
  allOf?: AnnotationSchema[];
  /** A reference to a schema elsewhere in the document, e.g. `#/$defs/color`. */
  $ref?: string;
  $defs?: Record<string, AnnotationSchema>;
  definitions?: Record<string, AnnotationSchema>;
}

/**
 * Completes column annotations from your own knowledge of them: called with where the cursor is
 * within an annotation, it returns the options to offer there (or null for none).
 */
export type AnnotationCompleter = (
  position: AnnotationPosition,
  context: CompletionContext,
) => readonly Completion[] | null;

/** The schemas that `schema` stands for: itself, or the alternatives it lists, following `$ref`s. */
function alternatives(
  schema: AnnotationSchema | boolean | undefined,
  root: AnnotationSchema,
  depth = 0,
): AnnotationSchema[] {
  if (typeof schema !== "object" || depth > 32) return [];
  if (schema.$ref !== undefined) {
    const target = resolveRef(schema.$ref, root);
    return target ? alternatives(target, root, depth + 1) : [];
  }
  const nested = [...(schema.anyOf ?? []), ...(schema.oneOf ?? []), ...(schema.allOf ?? [])];
  return [schema, ...nested.flatMap((alternative) => alternatives(alternative, root, depth + 1))];
}

/** Resolves a local JSON pointer like `#/$defs/color`. */
function resolveRef(ref: string, root: AnnotationSchema): AnnotationSchema | undefined {
  if (!ref.startsWith("#")) return undefined;
  let target: unknown = root;
  for (const part of ref.slice(1).split("/").filter(Boolean)) {
    const key = decodeURIComponent(part).replaceAll("~1", "/").replaceAll("~0", "~");
    target = typeof target === "object" && target !== null ? (target as Record<string, unknown>)[key] : undefined;
  }
  return typeof target === "object" && target !== null ? (target as AnnotationSchema) : undefined;
}

/** The schemas of the value at `segment` within a value described by `schema`. */
function child(schema: AnnotationSchema, segment: string | number): AnnotationSchema | boolean | undefined {
  if (typeof segment === "number") {
    return schema.prefixItems?.[segment] ?? schema.items;
  }
  return schema.properties?.[segment] ?? schema.additionalProperties;
}

/** The schemas describing the value at `path` within a value described by `root`. */
function schemasAt(root: AnnotationSchema, path: readonly (string | number)[]): AnnotationSchema[] {
  let schemas = alternatives(root, root);
  for (const segment of path) {
    schemas = schemas.flatMap((schema) => alternatives(child(schema, segment), root));
  }
  return schemas;
}

const IDENTIFIER = /^[\p{L}_][\p{L}\p{N}_]*$/u;

/** How a string is written in an annotation: bare if it's an identifier, otherwise quoted. */
function annotationString(value: string): string {
  return IDENTIFIER.test(value) ? value : `'${value.replaceAll("\\", "\\\\").replaceAll("'", "\\'")}'`;
}

/** How a JSON value is written in an annotation, or undefined for one that isn't offered. */
function annotationValue(value: unknown): string | undefined {
  if (value === null) return "@null";
  if (typeof value === "boolean") return `@${value}`;
  if (typeof value === "number") return String(value);
  if (typeof value === "string") return annotationString(value);
  return undefined;
}

function typeLabel(schema: AnnotationSchema): string | undefined {
  if (schema.enum) return "enum";
  return Array.isArray(schema.type) ? schema.type.join(" | ") : schema.type;
}

/** Inserts `text` for a completion, then starts completing again (e.g. a key's values). */
function applyThenComplete(text: string) {
  return (view: EditorView, completion: Completion, from: number, to: number): void => {
    view.dispatch({
      ...insertCompletionText(view.state, text, from, to),
      annotations: pickedCompletion.of(completion),
    });
    startCompletion(view);
  };
}

/** The options that `schema` offers at `position`. */
export function annotationOptions(
  schema: AnnotationSchema,
  position: AnnotationPosition,
): Completion[] {
  const schemas = schemasAt(schema, position.path);
  const options = new Map<string, Completion>();
  if (position.slot === "key") {
    for (const object of schemas) {
      for (const [key, value] of Object.entries(object.properties ?? {})) {
        if (position.keys.includes(key) || options.has(key)) continue;
        const described = alternatives(value, schema);
        const description = described.find((s) => s.description)?.description;
        const type = described.map(typeLabel).find(Boolean);
        options.set(key, {
          label: key,
          apply: applyThenComplete(`${annotationString(key)}:`),
          type: "property",
          ...(type ? { detail: type } : {}),
          ...(description ? { info: description } : {}),
        });
      }
    }
  } else {
    const offer = (value: unknown, type: string, info?: string) => {
      const text = annotationValue(value);
      if (text === undefined || options.has(text)) return;
      options.set(text, {
        label: typeof value === "string" ? value : text,
        apply: text,
        type,
        ...(info ? { info } : {}),
      });
    };
    for (const value of schemas) {
      for (const option of value.enum ?? []) offer(option, "enum", value.description);
      if ("const" in value) offer(value.const, "enum", value.description);
      const types = [value.type].flat();
      if (types.includes("boolean")) {
        offer(true, "constant");
        offer(false, "constant");
      }
      if (types.includes("null")) offer(null, "constant");
    }
  }
  // In the schema's order, among options that match equally well.
  return [...options.values()].map((option, index) => ({
    ...option,
    sortText: String(index).padStart(6, "0"),
  }));
}

// ------------------------------------------------------------------------------------------------
// The completion source
// ------------------------------------------------------------------------------------------------

/** A value, or a function that returns the current value (for one that may change). */
export type Dynamic<T> = T | (() => T);

export interface QuerydownCompletionConfig {
  /**
   * A `QuerydownCompleter` from the Querydown JavaScript bindings (`querydown-js`), created with
   * your database's schema.
   */
  completer: Completer;
  /**
   * For an editor holding a section of a query (other than the definitions): the query's base
   * table, whose columns are offered. Pass a function if it may change.
   */
  baseTable?: Dynamic<string | undefined>;
  /**
   * For an editor holding a section of a query (other than the definitions): the text of the
   * definitions section, so that what it defines is offered. Pass a function to read it from
   * another editor, e.g. `() => definitionsEditor.state.doc.toString()`.
   */
  definitions?: Dynamic<string | undefined>;
  /**
   * How to complete column annotations (`@{…}`): a JSON Schema describing the annotations your
   * application accepts, or a function that returns the options for a position within one.
   * Without this, annotations aren't completed.
   */
  annotations?: AnnotationSchema | AnnotationCompleter;
}

/** The CodeMirror type of each kind of item, which picks its icon. */
const COMPLETION_TYPES: Record<CompletionKind, string> = {
  table: "class",
  column: "property",
  "computed-column": "property",
  "custom-comparison": "method",
  function: "function",
  "aggregate-function": "function",
  "window-function": "function",
  constant: "constant",
  variable: "variable",
};

/**
 * Characters after which completion starts without anything typed: `#` (tables), `.` (path
 * steps), `$` (result columns), `\` (sorting expressions), `|` `%` (functions) and `@` (variables).
 */
const TRIGGERS = new Set(["#", ".", "$", "\\", "|", "%", "@"]);

/** Characters after which completion starts within an annotation: keys after `{`, values after `:`. */
const ANNOTATION_TRIGGERS = new Set(["{", "[", ":", "@"]);

/** A word being typed, which the options remain valid for. */
const WORD = /^[\p{L}\p{N}_]*$/u;
const ANNOTATION_WORD = /^@?[\p{L}\p{N}_]*$/u;

function read<T>(value: Dynamic<T>): T {
  return typeof value === "function" ? (value as () => T)() : value;
}

/**
 * Converts an item from the bindings' completer into a CodeMirror completion. `index` is its
 * position among the items, whose order (e.g. a table's columns in the schema's order) is kept
 * among options that match equally well.
 */
function toCompletion(item: CompletionItem, index: number): Completion {
  const type = COMPLETION_TYPES[item.kind as CompletionKind];
  return {
    label: item.label,
    sortText: String(index).padStart(6, "0"),
    // Each item has its CodeMirror type, which picks the icon, plus its Querydown kind, which
    // becomes a class (`cm-completionIcon-computed-column`) for styling particular kinds.
    type: type && type !== item.kind ? `${type} ${item.kind}` : item.kind,
    ...(item.insert !== null ? { apply: item.insert } : {}),
    ...(item.detail !== null ? { detail: item.detail } : {}),
    ...(item.info !== null ? { info: item.info } : {}),
    ...(item.boost ? { boost: item.boost } : {}),
  };
}

/**
 * A CodeMirror completion source for Querydown. You don't usually need this: pass `completion` to
 * `querydown()` instead, which installs it for the language. Use it directly to combine it with
 * other sources.
 */
export function querydownCompletionSource(
  config: QuerydownCompletionConfig & { grammar?: Grammar },
): CompletionSource {
  let reportedFailure = false;
  return (context: CompletionContext): CompletionResult | null => {
    const source = context.state.doc.toString();
    let completions: Completions;
    try {
      const baseTable = read(config.baseTable);
      const definitions = read(config.definitions);
      completions = config.completer.complete(source, context.pos, {
        grammar: config.grammar ?? "query",
        ...(baseTable !== undefined ? { baseTable } : {}),
        ...(definitions !== undefined ? { definitions } : {}),
      });
    } catch (error) {
      if (!reportedFailure) {
        reportedFailure = true;
        console.error("Querydown completion failed:", error);
      }
      return null;
    }
    const { from, annotation } = completions;
    const typed = source.slice(from, context.pos);
    const trigger = source[from - 1] ?? "";
    const triggers = annotation ? ANNOTATION_TRIGGERS : TRIGGERS;
    // Start completing (unless asked to) only once something has been typed, or after a character
    // that introduces something completable.
    if (!context.explicit && typed === "" && !triggers.has(trigger)) return null;
    if (annotation) {
      const { annotations } = config;
      if (!annotations) return null;
      const options =
        typeof annotations === "function"
          ? annotations(annotation, context)
          : annotationOptions(annotations, annotation);
      if (!options?.length) return null;
      return { from, options, validFor: ANNOTATION_WORD };
    }
    if (!completions.items.length) return null;
    return { from, options: completions.items.map(toCompletion), validFor: WORD };
  };
}
