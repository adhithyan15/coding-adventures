import { defineConfig } from "vitest/config";
import path from "path";

export default defineConfig({
  test: {
    // jsdom has no IndexedDB, so openWorkspaceStorage() takes its in-memory
    // fallback — precisely the path we want these tests to cover.
    environment: "jsdom",
    globals: true,
    include: ["__tests__/**/*.test.ts"],
    coverage: {
      provider: "v8",
      // The host's own logic seams — persistence and theme selection. main.tsx is DOM
      // boot glue verified live in a browser, and TaskApp.{light,dark}.tsx are
      // generated. A module with tests but missing from this list is silently exempt
      // from the threshold below, so add new seams here as they appear.
      include: ["src/persistence.ts", "src/theme.ts", "src/timeline.ts"],
      thresholds: { lines: 90 },
    },
  },
  resolve: {
    // Deduplicate React — file: deps can bundle their own copy, causing
    // "Invalid hook call". Force a single react / react-dom.
    dedupe: ["react", "react-dom"],
    alias: {
      react: path.resolve(__dirname, "node_modules/react"),
      "react-dom": path.resolve(__dirname, "node_modules/react-dom"),
    },
  },
});
