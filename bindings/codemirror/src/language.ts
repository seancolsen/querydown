import {
  defineLanguageFacet,
  Language,
  languageDataProp,
  LanguageSupport,
} from "@codemirror/language";
import type { Extension } from "@codemirror/state";
import {
  NodeSet,
  NodeType,
  Parser,
  Tree,
  type Input,
  type PartialParse,
  type TreeFragment,
} from "@lezer/common";
import { styleTags, tags, type Tag } from "@lezer/highlight";

import { querydownHighlighting } from "./highlight.js";
import { querydownTags, standardTags } from "./tags.js";
import { TOKEN_KINDS, type Grammar, type Token, type TokenKind, type Tokenize } from "./tokens.js";

/** Language data, e.g. the comment syntax used by CodeMirror's comment toggling commands. */
const languageData = defineLanguageFacet({
  commentTokens: { line: "//", block: { open: "/*", close: "*/" } },
  closeBrackets: { brackets: ["(", "[", "{", "'", '"', "`"] },
});

// ------------------------------------------------------------------------------------------------
// The syntax tree's node types
// ------------------------------------------------------------------------------------------------
//
// The tree is flat: a top node whose children are the tokens, one node per token. Each token kind
// gets its own node type, named in PascalCase (e.g. `BaseTable`), except that comments are split by
// form and brackets get one node type per character. Naming the bracket nodes after their
// characters is what lets CodeMirror's bracket matching pair them up.

const BRACKETS = ["(", ")", "[", "]", "{", "}"] as const;

const BRACKET_TAGS: Record<(typeof BRACKETS)[number], Tag> = {
  "(": tags.paren,
  ")": tags.paren,
  "[": tags.squareBracket,
  "]": tags.squareBracket,
  "{": tags.brace,
  "}": tags.brace,
};

function nodeName(kind: TokenKind): string {
  return kind.replace(/(^|-)(\w)/g, (_, __, c: string) => c.toUpperCase());
}

const nodeNames: string[] = [
  "Querydown",
  ...TOKEN_KINDS.filter((kind) => kind !== "comment" && kind !== "bracket").map(nodeName),
  "LineComment",
  "BlockComment",
  ...BRACKETS,
];

const styles: Record<string, Tag[]> = {
  LineComment: [querydownTags.comment, tags.lineComment],
  BlockComment: [querydownTags.comment, tags.blockComment],
};
for (const kind of TOKEN_KINDS) {
  if (kind !== "comment" && kind !== "bracket") {
    styles[nodeName(kind)] = [querydownTags[kind], standardTags[kind]];
  }
}
for (const bracket of BRACKETS) {
  styles[bracket] = [querydownTags.bracket, BRACKET_TAGS[bracket]];
}

const nodeSet = new NodeSet(
  nodeNames.map((name, id) =>
    id === 0
      ? // CodeMirror finds a language's data through its top node type.
        NodeType.define({ id, name, top: true, props: [[languageDataProp, languageData]] })
      : NodeType.define({ id, name }),
  ),
).extend(styleTags(styles));

const nodeIds = new Map(nodeNames.map((name, id) => [name, id]));

/**
 * The id of the node type for a token of `text`, or `undefined` for a kind this package doesn't
 * know.
 */
function nodeId(token: Token, text: string): number | undefined {
  switch (token.kind) {
    case "comment":
      return nodeIds.get(text.startsWith("//", token.from) ? "LineComment" : "BlockComment");
    case "bracket":
      return nodeIds.get(text[token.from] ?? "");
    default:
      return (TOKEN_KINDS as readonly string[]).includes(token.kind)
        ? nodeIds.get(nodeName(token.kind as TokenKind))
        : undefined;
  }
}

// ------------------------------------------------------------------------------------------------
// The parser
// ------------------------------------------------------------------------------------------------

/**
 * A Lezer parser that delegates to the Querydown bindings' `tokenize`. It isn't incremental: each
 * parse tokenizes the whole document, which is fast at the size of a query.
 */
class QuerydownParser extends Parser {
  #reportedFailure = false;

  constructor(
    readonly tokenize: Tokenize,
    readonly grammar: Grammar,
  ) {
    super();
  }

  createParse(
    input: Input,
    _fragments: readonly TreeFragment[],
    ranges: readonly { from: number; to: number }[],
  ): PartialParse {
    const from = ranges[0]?.from ?? 0;
    const to = ranges[ranges.length - 1]?.to ?? input.length;
    return new WholeParse(from, to, () => this.#buildTree(input.read(from, to), from));
  }

  /** Builds the tree for `text`, which begins at document position `offset`. */
  #buildTree(text: string, offset: number): Tree {
    let tokens: Token[] = [];
    try {
      tokens = this.tokenize(text, this.grammar);
    } catch (error) {
      // E.g. the bindings haven't been initialized yet. Leave the text unhighlighted rather than
      // breaking the editor, but say why, once.
      if (!this.#reportedFailure) {
        this.#reportedFailure = true;
        console.error("Querydown syntax highlighting failed:", error);
      }
    }
    const buffer: number[] = [];
    for (const token of tokens) {
      const id = nodeId(token, text);
      if (id !== undefined && token.from < token.to && token.to <= text.length) {
        // A leaf node: type, start, end, and its size in the buffer (4 numbers, i.e. itself).
        buffer.push(id, token.from + offset, token.to + offset, 4);
      }
    }
    return Tree.build({ buffer, nodeSet, topID: 0, start: offset, length: text.length });
  }
}

/** A parse that completes in a single step. */
class WholeParse implements PartialParse {
  parsedPos: number;
  stoppedAt: number | null = null;

  constructor(
    from: number,
    readonly to: number,
    readonly build: () => Tree,
  ) {
    this.parsedPos = from;
  }

  advance(): Tree {
    const tree = this.build();
    this.parsedPos = this.to;
    return tree;
  }

  stopAt(pos: number): void {
    this.stoppedAt = pos;
  }
}

// ------------------------------------------------------------------------------------------------
// The language
// ------------------------------------------------------------------------------------------------

export interface QuerydownLanguageConfig {
  /** The `tokenize` function exported by the Querydown JavaScript bindings (`querydown-js`). */
  tokenize: Tokenize;
  /**
   * Which grammar to read the document with: `"query"` (the default) for a whole query, or the
   * name of a section for an editor that holds just one section of a sectioned query.
   */
  grammar?: Grammar;
}

/**
 * The Querydown language, without any highlighting. Use {@link querydown} instead unless you want to
 * supply all of the highlighting yourself.
 */
export function querydownLanguage({ tokenize, grammar = "query" }: QuerydownLanguageConfig): Language {
  return new Language(languageData, new QuerydownParser(tokenize, grammar), [], "querydown");
}

let defaultHighlighting: Extension | undefined;

export interface QuerydownConfig extends QuerydownLanguageConfig {
  /**
   * The highlighting to install alongside the language. Defaults to {@link querydownHighlighting}
   * with its default palettes. Pass `false` to install none, e.g. to use a `HighlightStyle` of your
   * own.
   */
  highlighting?: Extension | false;
}

/**
 * Querydown language support for CodeMirror: the language plus, by default, syntax highlighting
 * that follows the editor's light or dark theme.
 *
 * ```ts
 * import init, { tokenize } from "querydown-js";
 * import { querydown } from "querydown-codemirror";
 *
 * await init(wasmUrl);
 * new EditorView({ extensions: [querydown({ tokenize })], parent });
 * ```
 */
export function querydown(config: QuerydownConfig): LanguageSupport {
  // The default highlighting is shared, so that editors don't each define the same styles.
  defaultHighlighting ??= querydownHighlighting();
  const { highlighting = defaultHighlighting } = config;
  return new LanguageSupport(querydownLanguage(config), highlighting === false ? [] : [highlighting]);
}
