import { defineConfig } from "vitest/config";
import path from "path";

export default defineConfig({
  test: {
    environment: "jsdom",
    globals: true,
    coverage: {
      exclude: ["**/*-tokens.ts", "**/*-grammar.ts"],
      provider: "v8",
      thresholds: {
        lines: 80,
      },
    },
  },
  resolve: {
    // Deduplicate React — file: protocol deps may bundle their own copy,
    // which causes "Invalid hook call" errors. Force all imports of react
    // and react-dom to resolve to this project's single copy.
    dedupe: ["react", "react-dom"],
    alias: {
      react: path.resolve(__dirname, "node_modules/react"),
      "react-dom": path.resolve(__dirname, "node_modules/react-dom"),
    },
  },
});
