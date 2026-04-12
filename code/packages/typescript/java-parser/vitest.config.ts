import { defineConfig, configDefaults } from "vitest/config";

export default defineConfig({
  test: {
    coverage: {
      provider: "v8",
      // Extend vitest's default exclusions with _grammar.ts — a generated data
      // file that has no testable logic and should not count toward the threshold.
      exclude: [...(configDefaults.coverage.exclude ?? []), "src/_grammar.ts"],
      thresholds: {
        lines: 80,
      },
    },
  },
});
