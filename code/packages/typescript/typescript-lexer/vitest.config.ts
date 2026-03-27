import { defineConfig, configDefaults } from "vitest/config";

export default defineConfig({
  test: {
    coverage: {
      // This package uses a thin-wrapper pattern: both tokenizer.ts and index.ts
      // re-export from ../../../../src/typescript/typescript-lexer/. Include only
      // src/**/*.ts so the real source files are measured, and exclude:
      //   - src/index.ts   — a single re-export line, no testable logic
      //   - src/_grammar.ts — a generated data file, no testable logic
      include: ["src/**/*.ts"],
      exclude: [
        ...(configDefaults.coverage.exclude ?? []),
        "src/index.ts",
        "src/_grammar.ts",
      ],
      provider: "v8",
      thresholds: {
        lines: 80,
      },
    },
  },
});
