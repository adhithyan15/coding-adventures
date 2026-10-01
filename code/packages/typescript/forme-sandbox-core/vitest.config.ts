import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    exclude: ["dist/**", "node_modules/**"],
    coverage: {
      provider: "v8",
      reporter: ["text", "json", "html"],
      thresholds: { lines: 90, functions: 90, branches: 85, statements: 90 },
      include: ["src/**/*.ts"],
    },
  },
});
