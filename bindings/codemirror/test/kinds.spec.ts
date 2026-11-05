import { readFileSync } from "node:fs";

import { expect, test } from "@playwright/test";

import { COMPLETION_KINDS } from "../src/completion.js";
import { TOKEN_KINDS } from "../src/tokens.js";

/** The members of a string union type declared in the bindings' `.d.ts`. */
function declaredUnion(name: string): string[] {
  const declarations = readFileSync(new URL("wasm/querydown_js.d.ts", import.meta.url), "utf8");
  const union = new RegExp(`export type ${name} =([^;]*);`).exec(declarations)?.[1];
  expect(union, `the bindings should declare \`${name}\``).toBeDefined();
  return [...union!.matchAll(/"([^"]+)"/g)].map((match) => match[1]!);
}

test("TOKEN_KINDS matches the kinds declared by the bindings", () => {
  expect([...TOKEN_KINDS].sort()).toEqual(declaredUnion("QuerydownTokenKind").sort());
});

test("COMPLETION_KINDS matches the kinds declared by the bindings", () => {
  expect([...COMPLETION_KINDS].sort()).toEqual(declaredUnion("QuerydownCompletionKind").sort());
});
