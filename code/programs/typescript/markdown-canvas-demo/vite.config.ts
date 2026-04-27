import { defineConfig } from "vite";

export default defineConfig({
  server: { port: 5173 },
  resolve: {
    // Preserve symlinks so that imports inside local file: dependencies
    // resolve against this app's node_modules (where npm installed all the
    // transitive deps) rather than the real package directory's (empty)
    // node_modules. This is required for the chained file: deps pattern
    // used repo-wide.
    preserveSymlinks: true,
  },
});
