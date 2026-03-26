import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "jsdom",
    globals: true,
    coverage: {
      exclude: ["**/*-tokens.ts", "**/*-grammar.ts"],
      provider: "v8",
      include: ["src/**/*.ts", "src/**/*.tsx"],
      exclude: ["src/main.tsx"],
    },
  },
});
