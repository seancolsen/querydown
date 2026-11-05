import { expect, test, type Page } from "@playwright/test";

import type { Setup } from "./harness/completion.js";

const ISSUES = [
  "id",
  "title",
  "description",
  "created_at",
  "author",
  "status",
  "project",
  "duplicate_of",
  "due_date",
];
const USERS = ["id", "username", "email", "team", "birth_date", "last_seen"];

test.beforeEach(async ({ page }) => {
  await page.goto("/completion.html");
  await page.locator("html[data-ready=true]").waitFor();
});

/** Replaces the harness's editor with one set up as described. */
async function setup(page: Page, setup: Setup): Promise<void> {
  await page.evaluate((setup) => window.harness.setup(setup), setup);
}

function doc(page: Page): Promise<string> {
  return page.evaluate(() => window.harness.doc());
}

const tooltip = (page: Page) => page.locator(".cm-tooltip-autocomplete");
const labels = (page: Page) => tooltip(page).locator(".cm-completionLabel");
const option = (page: Page, label: string) =>
  tooltip(page).locator("li").filter({ has: page.getByText(label, { exact: true }) });

test("typing `$` offers the base table's columns, in the schema's order", async ({ page }) => {
  await setup(page, { doc: "#issues " });
  await page.keyboard.type("$");
  await expect(labels(page)).toHaveText(ISSUES);
  // Each shows its type, and links say where they lead.
  await expect(option(page, "title").locator(".cm-completionDetail")).toHaveText("text");
  await expect(option(page, "author").locator(".cm-completionDetail")).toHaveText(
    "integer → users",
  );
  await expect(option(page, "title").locator(".cm-completionIcon-property")).toBeVisible();
});

test("typing filters the options, and accepting one inserts it", async ({ page }) => {
  await setup(page, { doc: "#issues " });
  await page.keyboard.type("$author.usern");
  await expect(labels(page)).toHaveText(["username"]);
  await page.keyboard.press("Enter");
  expect(await doc(page)).toBe("#issues $author.username");
});

test("a path leads to the columns of the table it reaches", async ({ page }) => {
  await setup(page, { doc: "#issues ‸ $id" });
  await page.keyboard.type("author.");
  await expect(labels(page)).toHaveText(USERS);
  await page.keyboard.type("team.");
  await expect(labels(page)).toHaveText(["id", "name"]);
});

test("tables after `#`, those related to the current table first", async ({ page }) => {
  await setup(page, { doc: "" });
  await page.keyboard.type("#");
  // At the start of the query, every table is a base table.
  await expect(labels(page)).toHaveCount(11);
  await expect(labels(page).first()).toHaveText("assignments");
  await page.keyboard.type("iss");
  await expect(labels(page)).toHaveText(["issue_labels", "issues"]);
  await page.keyboard.type("ues $#");
  // Of the tables, `clients` and `teams` aren't linked to `issues` directly.
  await expect(labels(page)).toHaveText([
    "assignments",
    "blocks",
    "comments",
    "issue_labels",
    "issues",
    "projects",
    "users",
    "clients",
    "labels",
    "products",
    "teams",
  ]);
});

test("functions after `|` and `%`, with their usage and descriptions", async ({ page }) => {
  await setup(page, { doc: "#issues " });
  await page.keyboard.type("$title|upp");
  await expect(labels(page)).toHaveText(["uppercase"]);
  await expect(option(page, "uppercase").locator(".cm-completionDetail")).toHaveText(
    "x|uppercase",
  );
  await expect(page.locator(".cm-completionInfo")).toHaveText("Converts a string to upper case.");
  await page.keyboard.press("Enter");
  await page.keyboard.type(" $#comments.created_at%m");
  await expect(labels(page)).toHaveText(["max", "min"]);
});

test("variables after `@`, including function parameters", async ({ page }) => {
  await setup(page, { doc: "@@overdue = @due @grace => ‸\n#issues" });
  await page.keyboard.type("@");
  await expect(labels(page)).toHaveText([
    "now",
    "infinity",
    "true",
    "false",
    "null",
    "due",
    "grace",
  ]);
});

test("only opens by itself after something completable", async ({ page }) => {
  await setup(page, { doc: "#issues" });
  // Not after whitespace…
  await page.keyboard.type(" ");
  await page.waitForTimeout(200);
  await expect(tooltip(page)).toHaveCount(0);
  // …nor where nothing applies, such as a string on the right of a comparison…
  await page.keyboard.type("status:op");
  await page.waitForTimeout(200);
  await expect(tooltip(page)).toHaveCount(0);
  // …but it can be asked for.
  await page.keyboard.type(" ");
  await page.keyboard.press("Control+Space");
  await expect(labels(page)).toHaveText(ISSUES);
});

test("names that need quoting are inserted with backticks", async ({ page }) => {
  // The issue-tracker schema has none, so complete a custom comparison with a quoted name.
  await setup(page, { doc: "#issues.`has comment`:@x = ++#comments{body:@x}\n#issues ‸" });
  await page.keyboard.type("has");
  await expect(labels(page)).toHaveText(["has comment"]);
  await expect(option(page, "has comment").locator(".cm-completionIcon-method")).toBeVisible();
  await page.keyboard.press("Enter");
  expect(await doc(page)).toMatch(/#issues `has comment`$/);
});

test("an editor for a section completes from the base table and definitions", async ({ page }) => {
  await setup(page, {
    doc: "",
    grammar: "display",
    baseTable: "issues",
    definitions: "#issues.overdue = due_date:<@now",
  });
  await page.keyboard.type("$ov");
  await expect(labels(page)).toHaveText(["overdue"]);
  await expect(option(page, "overdue").locator(".cm-completionDetail")).toHaveText("computed");
});

test("annotation keys and values come from the annotation schema", async ({ page }) => {
  await setup(page, { doc: "#issues $title " });
  await page.keyboard.type("@{");
  await expect(labels(page)).toHaveText(["width", "align", "hidden", "format", "rules"]);
  await expect(option(page, "width").locator(".cm-completionDetail")).toHaveText("number");
  await page.keyboard.type("al");
  await page.keyboard.press("Enter");
  // Accepting a key adds its `:`, which opens its values.
  expect(await doc(page)).toBe("#issues $title @{align:");
  await expect(labels(page)).toHaveText(["left", "center", "right"]);
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Enter");
  await page.keyboard.type(" hidden:@");
  await expect(labels(page)).toHaveText(["@true", "@false"]);
  await page.keyboard.press("Enter");
  // Keys already present aren't offered again.
  await page.keyboard.press("Space");
  await page.keyboard.press("Control+Space");
  await expect(labels(page)).toHaveText(["width", "format", "rules"]);
  expect(await doc(page)).toBe("#issues $title @{align:center hidden:@true ");
});

test("annotation values follow references, alternatives and arrays", async ({ page }) => {
  await setup(page, { doc: "#issues $title @{format‸}" });
  await page.keyboard.type(":");
  await expect(labels(page)).toHaveText(["date", "relative time", "YYYY-MM-DD"]);
  await page.keyboard.type("r");
  await page.keyboard.press("Enter");
  // A value that isn't an identifier is quoted.
  expect(await doc(page)).toBe("#issues $title @{format:'relative time'}");

  await setup(page, { doc: "#issues $title @{rules:[{gte:10 bg:'#fbc9ff'} ‸]}" });
  await page.keyboard.type("{");
  await expect(labels(page)).toHaveText(["gte", "bg"]);
  await page.keyboard.type("bg:");
  await expect(labels(page)).toHaveText(["#fbc9ff", "#d5d2ff"]);
});

test("annotations can be completed by a function", async ({ page }) => {
  await setup(page, { doc: "#issues $title @{rules:[{", annotations: "function" });
  await page.keyboard.press("Control+Space");
  await expect(labels(page)).toHaveText(["key:rules.0"]);
});

test("without an annotation schema, annotations aren't completed", async ({ page }) => {
  await setup(page, { doc: "#issues $title ", annotations: "none" });
  await page.keyboard.type("@{");
  await page.keyboard.press("Control+Space");
  await page.waitForTimeout(200);
  await expect(tooltip(page)).toHaveCount(0);
});

test("a failing completer leaves the editor working, without completion", async ({ page }) => {
  const errors: string[] = [];
  page.on("console", (message) => message.type() === "error" && errors.push(message.text()));
  await setup(page, { doc: "#issues ", failing: true });
  await page.keyboard.type("$ti");
  await page.waitForTimeout(200);
  await expect(tooltip(page)).toHaveCount(0);
  expect(await doc(page)).toBe("#issues $ti");
  expect(errors).toEqual([expect.stringContaining("Querydown completion failed")]);
});

for (const theme of ["light", "dark"] as const) {
  test.describe(`${theme} theme`, () => {
    test.beforeEach(async ({ page }) => {
      await page.goto(`/completion.html?theme=${theme}`);
      await page.locator("html[data-ready=true]").waitFor();
    });

    /** Screenshots the editor and its tooltip, which hangs below it. */
    async function screenshot(page: Page, name: string): Promise<void> {
      await page.mouse.move(0, 0);
      await expect(page).toHaveScreenshot(`${name}-${theme}.png`, {
        clip: { x: 0, y: 0, width: 800, height: 280 },
      });
    }

    test("columns look right", async ({ page }) => {
      await setup(page, { doc: "#issues status:open $title " });
      await page.keyboard.type("$author.");
      await expect(labels(page)).toHaveText(USERS);
      await page.keyboard.press("ArrowDown");
      await screenshot(page, "completion-columns");
    });

    test("functions look right", async ({ page }) => {
      await setup(page, { doc: "#issues status:open $title " });
      await page.keyboard.type("$due_date|da");
      await expect(labels(page)).toHaveText(["day", "days"]);
      await page.keyboard.press("ArrowDown");
      await expect(page.locator(".cm-completionInfo")).toBeVisible();
      await screenshot(page, "completion-functions");
    });
  });
}
