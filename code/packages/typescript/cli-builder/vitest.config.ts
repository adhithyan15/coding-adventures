import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    coverage: {
      exclude: ["**/*-tokens.ts", "**/*-grammar.ts"],
      provider: "v8",
      thresholds: {
        lines: 90,
      },
    },
  },
});
