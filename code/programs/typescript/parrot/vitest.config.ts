import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    coverage: {
      provider: "v8",
      reporter: ["text", "lcov"],
      exclude: ["vitest.config.ts", "dist/**"],
      thresholds: { lines: 80, functions: 80, branches: 80 }
    }
  }
});
