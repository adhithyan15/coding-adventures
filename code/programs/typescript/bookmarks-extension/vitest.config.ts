import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "jsdom",
    setupFiles: ["tests/setup.ts"],
    coverage: {
      provider: "v8",
      exclude: [
        "vite.config.ts",
        "vitest.config.ts",
        "src/background/service-worker.ts",
        "dist/**",
        "scripts/**",
        "tests/**",
      ],
      thresholds: {
        lines: 80,
      },
    },
  },
});
