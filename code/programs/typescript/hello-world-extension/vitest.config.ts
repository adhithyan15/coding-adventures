import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "jsdom",
    coverage: {
      provider: "v8",
      exclude: [
        "vite.config.ts",
        "vitest.config.ts",
        "src/background/service-worker.ts",
      ],
      thresholds: {
        lines: 80,
      },
    },
  },
});
