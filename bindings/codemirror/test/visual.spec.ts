import { expect, test } from "@playwright/test";

import { samples } from "./harness/samples.js";

for (const theme of ["light", "dark"] as const) {
  test.describe(`${theme} theme`, () => {
    test.beforeEach(async ({ page }) => {
      await page.goto(`/?theme=${theme}`);
      await page.locator("html[data-ready=true]").waitFor();
    });

    for (const sample of samples) {
      test(sample.name, async ({ page }) => {
        const section = page.locator(`section[data-sample="${sample.name}"]`);
        // Check that each sample demonstrates what it means to, since valid and invalid source are
        // highlighted by different means.
        const syntaxError = await section.getAttribute("data-syntax-error");
        if (sample.valid) {
          expect(syntaxError, "the sample should be valid Querydown").toBe("");
        } else {
          expect(syntaxError, "the sample should be invalid Querydown").not.toBe("");
        }
        await expect(section.locator(".cm-editor")).toHaveScreenshot(`${sample.name}-${theme}.png`);
      });
    }
  });
}
