import { fileURLToPath } from "node:url";

import { defineConfig } from "vite";

const packageDir = fileURLToPath(new URL("..", import.meta.url));

/** Serves the visual test harness (`test/harness`). */
export default defineConfig({
  root: fileURLToPath(new URL("harness", import.meta.url)),
  server: {
    port: 4183,
    strictPort: true,
    // The harness imports the package's source and the bindings built into `test/wasm`.
    fs: { allow: [packageDir] },
  },
});
