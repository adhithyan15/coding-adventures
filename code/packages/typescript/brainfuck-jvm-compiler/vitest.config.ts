import { defineConfig } from "vitest/config";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const __dirname = dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  server: {
    fs: {
      allow: [resolve(__dirname, "..")],
    },
  },
  test: {
    coverage: {
      provider: "v8",
      thresholds: {
        lines: 80,
      },
    },
  },
});
