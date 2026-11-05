import { readFileSync } from "node:fs";

import { expect, test } from "@playwright/test";

import { TOKEN_KINDS } from "../src/tokens.js";

test("TOKEN_KINDS matches the kinds declared by the bindings", () => {
  const declarations = readFileSync(new URL("wasm/querydown_js.d.ts", import.meta.url), "utf8");
  const union = /export type QuerydownTokenKind =([^;]*);/.exec(declarations)?.[1];
  expect(union, "the bindings should declare `QuerydownTokenKind`").toBeDefined();
  const declared = [...union!.matchAll(/"([^"]+)"/g)].map((match) => match[1]);
  expect([...TOKEN_KINDS].sort()).toEqual(declared.sort());
});
