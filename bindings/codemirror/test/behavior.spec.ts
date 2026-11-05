import { expect, test, type Page } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.goto("/behavior.html");
  await page.locator("html[data-ready=true]").waitFor();
});

/** Calls one of the probes in `harness/behavior.ts`. */
function probe<K extends keyof Window["probes"]>(
  page: Page,
  name: K,
  ...args: Parameters<Window["probes"][K]>
): Promise<ReturnType<Window["probes"][K]>> {
  return page.evaluate(
    ([name, args]) => (window.probes[name] as (...a: unknown[]) => unknown)(...args),
    [name, args] as const,
  ) as Promise<ReturnType<Window["probes"][K]>>;
}

test("each token becomes a node named for its kind", async ({ page }) => {
  expect(await probe(page, "nodes", "#issues title:a // c\n$x|f(1) /* d */")).toEqual([
    "#issues:BaseTable",
    "title:Column",
    "::ComparisonOperator",
    "a:String",
    "// c:LineComment",
    "$:ResultColumn",
    "x:Column",
    "|:Pipe",
    "f:Function",
    "(:(",
    "1:Number",
    "):)",
    "/* d */:BlockComment",
  ]);
});

test("brackets match, skipping those within strings", async ({ page }) => {
  const doc = '#issues $x|concat("(" (y))';
  const open = doc.indexOf("concat(") + "concat".length;
  expect(await probe(page, "matchingBracket", doc, open)).toBe(doc.length - 1);
});

test("comment tokens are declared for toggling comments", async ({ page }) => {
  expect(await probe(page, "commentTokens")).toEqual({
    line: "//",
    block: { open: "/*", close: "*/" },
  });
});

test("a failing tokenizer leaves the text unhighlighted rather than breaking the editor", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("console", (message) => message.type() === "error" && errors.push(message.text()));
  expect(await probe(page, "failingTokenizer")).toEqual({ text: "#issues $id", tokens: 0 });
  expect(errors).toEqual([expect.stringContaining("Querydown syntax highlighting failed")]);
});
