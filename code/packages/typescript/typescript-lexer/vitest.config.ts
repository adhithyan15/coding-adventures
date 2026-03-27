import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    coverage: {
      provider: "v8",
      // _grammar.ts is a generated data file — exclude it from coverage
      // so the threshold applies only to hand-written logic.
      exclude: ["src/_grammar.ts"],
      thresholds: {
        lines: 80,
      },
    },
  },
});
