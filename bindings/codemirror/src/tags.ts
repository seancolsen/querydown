import { Tag, tags } from "@lezer/highlight";

import type { TokenKind } from "./tokens.js";

/**
 * A highlighting tag for each kind of Querydown token, e.g. `querydownTags.column`.
 *
 * Use these to style Querydown in your own `HighlightStyle`. Every token also carries the closest
 * standard tag from `@lezer/highlight` (see {@link standardTags}), so highlight styles written for
 * other languages color Querydown sensibly too, just less precisely.
 */
export const querydownTags: Readonly<Record<TokenKind, Tag>> = {
  comment: Tag.define("querydown.comment"),
  string: Tag.define("querydown.string"),
  "string-escape": Tag.define("querydown.string-escape"),
  "search-term": Tag.define("querydown.search-term"),
  number: Tag.define("querydown.number"),
  duration: Tag.define("querydown.duration"),
  date: Tag.define("querydown.date"),
  constant: Tag.define("querydown.constant"),
  variable: Tag.define("querydown.variable"),
  "base-table": Tag.define("querydown.base-table"),
  table: Tag.define("querydown.table"),
  column: Tag.define("querydown.column"),
  function: Tag.define("querydown.function"),
  "aggregate-function": Tag.define("querydown.aggregate-function"),
  pipe: Tag.define("querydown.pipe"),
  "comparison-operator": Tag.define("querydown.comparison-operator"),
  operator: Tag.define("querydown.operator"),
  "has-quantity": Tag.define("querydown.has-quantity"),
  glob: Tag.define("querydown.glob"),
  case: Tag.define("querydown.case"),
  "path-separator": Tag.define("querydown.path-separator"),
  "result-column": Tag.define("querydown.result-column"),
  sort: Tag.define("querydown.sort"),
  flag: Tag.define("querydown.flag"),
  alias: Tag.define("querydown.alias"),
  annotation: Tag.define("querydown.annotation"),
  "annotation-key": Tag.define("querydown.annotation-key"),
  "annotation-value": Tag.define("querydown.annotation-value"),
  "annotation-bracket": Tag.define("querydown.annotation-bracket"),
  pipeline: Tag.define("querydown.pipeline"),
  definition: Tag.define("querydown.definition"),
  bracket: Tag.define("querydown.bracket"),
  punctuation: Tag.define("querydown.punctuation"),
  invalid: Tag.define("querydown.invalid"),
};

/**
 * The standard `@lezer/highlight` tag that each kind of token also carries, so that generic
 * highlight styles (e.g. CodeMirror's `defaultHighlightStyle`) apply to Querydown.
 */
export const standardTags: Readonly<Record<TokenKind, Tag>> = {
  comment: tags.comment,
  string: tags.string,
  "string-escape": tags.escape,
  "search-term": tags.special(tags.string),
  number: tags.number,
  duration: tags.number,
  date: tags.literal,
  constant: tags.atom,
  variable: tags.variableName,
  "base-table": tags.special(tags.typeName),
  table: tags.typeName,
  column: tags.propertyName,
  function: tags.function(tags.variableName),
  "aggregate-function": tags.function(tags.variableName),
  pipe: tags.operator,
  "comparison-operator": tags.compareOperator,
  operator: tags.operator,
  "has-quantity": tags.logicOperator,
  glob: tags.special(tags.operator),
  case: tags.controlKeyword,
  "path-separator": tags.derefOperator,
  "result-column": tags.keyword,
  sort: tags.keyword,
  flag: tags.modifier,
  alias: tags.definition(tags.propertyName),
  annotation: tags.meta,
  "annotation-key": tags.attributeName,
  "annotation-value": tags.special(tags.literal),
  "annotation-bracket": tags.special(tags.bracket),
  pipeline: tags.separator,
  definition: tags.definitionOperator,
  bracket: tags.bracket,
  punctuation: tags.punctuation,
  invalid: tags.invalid,
};
