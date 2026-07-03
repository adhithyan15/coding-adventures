import { defineConfig, configDefaults } from "vitest/config";

export default defineConfig({
  test: {
    coverage: {
      provider: "v8",
      // Extend vitest's default exclusions with all `_grammar*.ts` files —
      // generated data with no testable logic (the unversioned `_grammar.ts`
      // plus per-standard `_grammar_1987.ts`, `_grammar_1993.ts`,
      // `_grammar_2002.ts`, `_grammar_2008.ts`, `_grammar_2019.ts`).
      exclude: [...(configDefaults.coverage.exclude ?? []), "src/_grammar*.ts"],
      thresholds: {
        lines: 80,
      },
    },
  },
});
