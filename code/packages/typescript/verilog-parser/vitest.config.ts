import { defineConfig, configDefaults } from "vitest/config";

export default defineConfig({
  test: {
    coverage: {
      provider: "v8",
      // Extend vitest's default exclusions with all `_grammar*.ts` files —
      // generated data with no testable logic (includes the unversioned
      // `_grammar.ts` plus the per-standard `_grammar_1995.ts`, `_grammar_2001.ts`,
      // `_grammar_2005.ts`).
      exclude: [...(configDefaults.coverage.exclude ?? []), "src/_grammar*.ts"],
      thresholds: {
        lines: 80,
      },
    },
  },
});
