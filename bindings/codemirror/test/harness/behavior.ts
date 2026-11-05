// Exposes probes of the language's non-visual behavior for `behavior.spec.ts` to call.

import { matchBrackets, syntaxTree } from "@codemirror/language";
import { EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";

import { querydown, type Tokenize } from "../../src/index.js";
import init, { tokenize } from "../wasm/querydown_js.js";

await init();

function view(doc: string, tokenizer: Tokenize = tokenize): EditorView {
  return new EditorView({
    parent: document.body,
    state: EditorState.create({ doc, extensions: [querydown({ tokenize: tokenizer })] }),
  });
}

const probes = {
  /** The node names of the syntax tree's tokens, as `text:Name` pairs. */
  nodes(doc: string): string[] {
    const { state } = view(doc);
    const nodes: string[] = [];
    syntaxTree(state).iterate({
      enter: (node) => {
        if (node.type.isTop) return;
        nodes.push(`${state.doc.sliceString(node.from, node.to)}:${node.name}`);
      },
    });
    return nodes;
  },

  /** Where the bracket at `pos` is matched, or null if it isn't matched. */
  matchingBracket(doc: string, pos: number): number | null {
    const match = matchBrackets(view(doc).state, pos, 1);
    return match?.matched ? (match.end?.from ?? null) : null;
  },

  /** The comment tokens that the language declares. */
  commentTokens(): unknown {
    return view("").state.languageDataAt("commentTokens", 0)[0];
  },

  /** The editor's text and number of tokens when `tokenize` throws. */
  failingTokenizer(): { text: string; tokens: number } {
    const editor = view("#issues $id", () => {
      throw new Error("not initialized");
    });
    let tokens = 0;
    syntaxTree(editor.state).iterate({ enter: (node) => void (node.type.isTop || tokens++) });
    return { text: editor.contentDOM.textContent ?? "", tokens };
  },
};

declare global {
  interface Window {
    probes: typeof probes;
  }
}
window.probes = probes;
document.documentElement.dataset.ready = "true";
