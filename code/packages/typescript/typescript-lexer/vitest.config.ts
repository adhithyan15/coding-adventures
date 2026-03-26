import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    coverage: {
      exclude: ["**/*-tokens.ts", "**/*-grammar.ts"],
      include: [
        "src/**/*.ts",
      ],
      exclude: [
        "src/index.ts",
      ],
      provider: "v8",
      thresholds: {
        lines: 80,
      },
    },
  },
});
