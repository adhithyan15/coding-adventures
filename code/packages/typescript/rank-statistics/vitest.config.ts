import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    include: ["tests/**/*.test.ts"],
    coverage: {
      provider: "v8",
      include: ["src/**/*.ts"],
      reporter: ["text", "json-summary"],
      thresholds: {
        lines: 95,
        branches: 95,
        functions: 95,
        statements: 95,
      },
    },
  },
});
