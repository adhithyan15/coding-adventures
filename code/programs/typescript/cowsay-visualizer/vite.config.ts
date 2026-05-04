import { defineConfig } from "vite";

export default defineConfig({
  server: { port: 5175 },
  resolve: {
    // Preserve symlinks so that imports inside local file: dependencies
    // resolve against this app's node_modules (where npm installed all the
    // transitive deps) rather than the real package directory's (empty)
    // node_modules. Required for the chained file: deps pattern repo-wide.
    preserveSymlinks: true,
  },
});
