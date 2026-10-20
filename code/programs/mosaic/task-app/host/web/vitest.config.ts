import { defineConfig } from "vitest/config";
import path from "path";

export default defineConfig({
  test: {
    // jsdom has no IndexedDB, so openWorkspaceStorage() takes its in-memory
    // fallback — precisely the path we want these tests to cover.
    environment: "jsdom",
    // Node 25 turned its own Web Storage on by default. Without a
    // --localstorage-file, Node's global `localStorage` is a stub with no
    // setItem, and it shadows jsdom's working one, so every theme-persistence
    // test fails on Node 25 while passing on CI's Node 22. Switching Node's copy
    // off restores jsdom's on every Node version (the flag is a no-op where the
    // feature was already off).
    execArgv: ["--no-experimental-webstorage"],
    globals: true,
    // .tsx is listed explicitly: the previous glob was .test.ts only, so a
    // component test would have been collected by nobody and reported as a
    // pass. Startup states are components, so this had to widen with them.
    include: ["__tests__/**/*.test.ts", "__tests__/**/*.test.tsx"],
    coverage: {
      provider: "v8",
      // The host's own logic seams — persistence, theme selection, and the
      // startup states. main.tsx is DOM boot glue verified live in a browser,
      // and TaskApp.{light,dark}.tsx are generated. A module with tests but
      // missing from this list is silently exempt from the threshold below, so
      // add new seams here as they appear.
      include: [
        "src/persistence.ts",
        "src/startup.tsx",
        "src/theme.ts",
        "src/timeline.ts",
      ],
      thresholds: { lines: 90 },
    },
  },
  resolve: {
    // Deduplicate React — file: deps can bundle their own copy, causing
    // "Invalid hook call". Force a single react / react-dom.
    dedupe: ["react", "react-dom"],
    alias: {
      react: path.resolve(import.meta.dirname, "node_modules/react"),
      "react-dom": path.resolve(import.meta.dirname, "node_modules/react-dom"),
      "./TaskApp.light": path.resolve(import.meta.dirname, "__tests__/stubs/TaskApp.tsx"),
      "./TaskApp.dark": path.resolve(import.meta.dirname, "__tests__/stubs/TaskApp.tsx"),
      "./task-engine.mjs": path.resolve(import.meta.dirname, "__tests__/stubs/task-engine.mjs"),
    },
  },
});
