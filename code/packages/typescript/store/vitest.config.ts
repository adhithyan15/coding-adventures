import { defineConfig } from "vitest/config";
import path from "path";

export default defineConfig({
  test: {
    environment: "jsdom",
    globals: true,
    coverage: {
      exclude: ["**/*-tokens.ts", "**/*-grammar.ts"],
      provider: "v8",
      thresholds: { lines: 80 },
    },
  },
  resolve: {
    dedupe: ["react", "react-dom"],
    alias: {
      react: path.resolve(__dirname, "node_modules/react"),
      "react-dom": path.resolve(__dirname, "node_modules/react-dom"),
    },
  },
});
