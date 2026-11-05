// An editor with Querydown completion against the issue-tracker schema used throughout the docs.
// `completion.spec.ts` sets it up for each test through `window.harness`; open
// `/completion.html` with `npm run test:visual:serve` to try it out (`?theme=dark` for dark).

import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/700.css";
import "./style.css";

import { autocompletion } from "@codemirror/autocomplete";
import { EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";

import {
  querydown,
  type AnnotationSchema,
  type Completer,
  type Grammar,
  type QuerydownCompletionConfig,
} from "../../src/index.js";
import init, { QuerydownCompleter, tokenize } from "../wasm/querydown_js.js";
import schemaJson from "../../../../compiler/resources/test/issue_schema.json?raw";

await init();

const dark = new URLSearchParams(location.search).get("theme") === "dark";
document.documentElement.dataset.theme = dark ? "dark" : "light";

/** An application's column annotations, as a JSON Schema. */
const annotationSchema: AnnotationSchema = {
  type: "object",
  properties: {
    width: { type: "number", description: "The column's width, in pixels." },
    align: { enum: ["left", "center", "right"], description: "How to align the column's values." },
    hidden: { type: "boolean" },
    format: { $ref: "#/$defs/format" },
    rules: {
      type: "array",
      description: "Conditional formatting, applied in order.",
      items: {
        type: "object",
        properties: { gte: { type: "number" }, bg: { $ref: "#/$defs/color" } },
      },
    },
  },
  $defs: {
    format: { anyOf: [{ enum: ["date", "relative time"] }, { const: "YYYY-MM-DD" }] },
    color: { enum: ["#fbc9ff", "#d5d2ff"], description: "A highlight color." },
  },
};

const theme = EditorView.theme(
  {
    "&": {
      color: dark ? "#e6edf3" : "#1f2328",
      backgroundColor: dark ? "#0d1117" : "#ffffff",
      border: `1px solid ${dark ? "#30363d" : "#d0d7de"}`,
      borderRadius: "6px",
    },
    ".cm-content, .cm-tooltip": {
      fontFamily: "'JetBrains Mono', monospace",
      fontSize: "14px",
      fontVariantLigatures: "none",
    },
    ".cm-content": { padding: "8px 0" },
    ".cm-line": { padding: "0 12px" },
  },
  { dark },
);

export interface Setup {
  /** The document, with the cursor marked by `‸` (or at the end). */
  doc: string;
  grammar?: Grammar;
  baseTable?: string;
  definitions?: string;
  /** Complete annotations with the schema above (the default), or by listing every key path. */
  annotations?: "schema" | "function" | "none";
  /** Use a completer that throws, rather than the real one. */
  failing?: boolean;
}

let view: EditorView | undefined;

const failingCompleter: Completer = {
  complete() {
    throw new Error("not initialized");
  },
};

function annotations(setup: Setup): QuerydownCompletionConfig["annotations"] {
  switch (setup.annotations ?? "schema") {
    case "schema":
      return annotationSchema;
    case "function":
      return (position) => [{ label: `${position.slot}:${position.path.join(".") || "root"}` }];
    default:
      return undefined;
  }
}

const harness = {
  /** Replaces the editor with a new one, focused, as described by `setup`. */
  setup(setup: Setup): void {
    view?.destroy();
    const cursor = setup.doc.includes("‸") ? setup.doc.indexOf("‸") : setup.doc.length;
    const completion: QuerydownCompletionConfig = {
      completer: setup.failing ? failingCompleter : new QuerydownCompleter(schemaJson),
      ...(setup.baseTable !== undefined ? { baseTable: setup.baseTable } : {}),
      ...(setup.definitions !== undefined ? { definitions: () => setup.definitions } : {}),
      annotations: annotations(setup),
    };
    view = new EditorView({
      parent: document.getElementById("editor")!,
      state: EditorState.create({
        doc: setup.doc.replace("‸", ""),
        selection: { anchor: cursor },
        extensions: [
          theme,
          // No delay before an option can be accepted, so that tests can accept one right away.
          autocompletion({ interactionDelay: 0 }),
          querydown({ tokenize, grammar: setup.grammar ?? "query", completion }),
        ],
      }),
    });
    view.focus();
  },

  /** The editor's text. */
  doc(): string {
    return view?.state.doc.toString() ?? "";
  },
};

declare global {
  interface Window {
    harness: typeof harness;
  }
}
window.harness = harness;

harness.setup({ doc: "#issues status:open $title $author.‸" });
await document.fonts.ready;
document.documentElement.dataset.ready = "true";
