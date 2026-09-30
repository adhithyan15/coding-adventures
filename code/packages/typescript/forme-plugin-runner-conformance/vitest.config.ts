import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    testTimeout: 15_000,
    coverage: {
      provider: "v8",
      reporter: ["text", "json-summary"],
      thresholds: { lines: 94, branches: 85, functions: 90, statements: 90 },
    },
  },
});
