import { defineConfig, configDefaults } from "vitest/config";

export default defineConfig({
  test: {
    coverage: {
      provider: "v8",
      // Exclude generated data files and the barrel re-export — neither
      // contains testable logic, so they should not count toward the threshold.
      exclude: [
        ...(configDefaults.coverage.exclude ?? []),
        "src/index.ts",
        "src/_grammar.ts",
      ],
      thresholds: {
        lines: 80,
      },
    },
  },
});
