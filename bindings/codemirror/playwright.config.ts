import { defineConfig, devices } from "@playwright/test";

const port = 4183;

export default defineConfig({
  testDir: "test",
  // Committed alongside the tests, one PNG per sample and theme.
  snapshotPathTemplate: "{testDir}/snapshots/{arg}{ext}",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  reporter: "list",
  expect: {
    // Syntax colors are the whole point, so compare strictly: any changed pixel fails.
    toHaveScreenshot: { maxDiffPixels: 0, threshold: 0 },
  },
  use: {
    baseURL: `http://localhost:${port}`,
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"], viewport: { width: 800, height: 1200 }, deviceScaleFactor: 1 },
    },
  ],
  webServer: {
    command: "npm run test:visual:serve",
    port,
    reuseExistingServer: !process.env.CI,
  },
});
