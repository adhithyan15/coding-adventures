import { defineConfig } from "vitest/config";

// Preserve the numerical suites while removing Jest's vulnerable coverage graph.
export default defineConfig({
  test: { globals: true, environment: "node", include: ["tests/**/*.test.ts"] },
});
