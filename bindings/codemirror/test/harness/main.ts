// Renders every sample in a CodeMirror editor, in the light or dark theme given by the `theme` query
// parameter (`?theme=dark`). The visual tests screenshot each editor; open the page with
// `npm run test:visual:serve` to look at them directly.

import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/400-italic.css";
import "@fontsource/jetbrains-mono/700.css";
import "./style.css";

import { defaultHighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { EditorState, type Extension } from "@codemirror/state";
import { oneDarkHighlightStyle } from "@codemirror/theme-one-dark";
import { EditorView } from "@codemirror/view";

import { querydown, querydownHighlighting } from "../../src/index.js";
import init, { compile, compile_sections, is_querydown_error, tokenize } from "../wasm/querydown_js.js";
import { samples, type Sample } from "./samples.js";

await init();

const dark = new URLSearchParams(location.search).get("theme") === "dark";
document.documentElement.dataset.theme = dark ? "dark" : "light";

/** A minimal editor theme; the `dark` flag is what selects Querydown's dark palette. */
const editorTheme = EditorView.theme(
  {
    "&": {
      color: dark ? "#e6edf3" : "#1f2328",
      backgroundColor: dark ? "#0d1117" : "#ffffff",
      border: `1px solid ${dark ? "#30363d" : "#d0d7de"}`,
      borderRadius: "6px",
    },
    ".cm-content": {
      fontFamily: "'JetBrains Mono', monospace",
      fontSize: "14px",
      // Show the characters as typed, e.g. `->` rather than an arrow.
      fontVariantLigatures: "none",
      padding: "8px 0",
    },
    ".cm-line": { padding: "0 12px" },
  },
  { dark },
);

function highlighting(sample: Sample): Extension | undefined {
  switch (sample.highlighting) {
    case "custom":
      // Overrides for a few kinds of token, including one taken from a CSS custom property.
      return querydownHighlighting({
        light: {
          "base-table": "var(--custom-table)",
          column: { color: "#c2410c", fontWeight: "bold" },
          string: { color: "#15803d", fontStyle: "italic" },
        },
        dark: {
          "base-table": "var(--custom-table)",
          column: { color: "#fdba74", fontWeight: "bold" },
          string: { color: "#86efac", fontStyle: "italic" },
        },
      });
    case "standard":
      return syntaxHighlighting(dark ? oneDarkHighlightStyle : defaultHighlightStyle);
    default:
      return undefined;
  }
}

/** The syntax error in a sample, if any, found by compiling it against an empty schema. */
function syntaxError(sample: Sample): string | null {
  const schema = JSON.stringify({ tables: [], links: [] });
  const section = (name: Sample["grammar"]) => (sample.grammar === name ? sample.source : "");
  try {
    if (sample.grammar === "query") {
      compile(schema, "postgres", sample.source);
    } else {
      compile_sections(
        schema,
        "postgres",
        "issues",
        section("definitions"),
        section("conditions"),
        section("sorting"),
        section("display"),
      );
    }
  } catch (error) {
    if (is_querydown_error(error) && error.kind === "syntax") return error.message;
  }
  return null;
}

const container = document.getElementById("samples")!;
for (const sample of samples) {
  const section = document.createElement("section");
  section.dataset.sample = sample.name;
  section.dataset.syntaxError = syntaxError(sample) ?? "";
  const heading = document.createElement("h2");
  heading.textContent = `${sample.title} (${sample.grammar})`;
  section.append(heading);
  container.append(section);

  const custom = highlighting(sample);
  new EditorView({
    parent: section,
    state: EditorState.create({
      doc: sample.source,
      extensions: [
        editorTheme,
        querydown({
          tokenize,
          grammar: sample.grammar,
          ...(custom ? { highlighting: custom } : {}),
        }),
      ],
    }),
  });
}

await document.fonts.ready;
document.documentElement.dataset.ready = "true";
