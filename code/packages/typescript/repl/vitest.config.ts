import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    coverage: {
      provider: "v8",
      // Only measure coverage on source files under src/.
      // types.ts is excluded because it contains only TypeScript type
      // declarations — the compiler strips them entirely, leaving zero
      // runnable JavaScript statements to measure.
      include: ["src/**/*.ts"],
      exclude: ["src/types.ts"],
      thresholds: {
        lines: 80,
      },
    },
  },
});
