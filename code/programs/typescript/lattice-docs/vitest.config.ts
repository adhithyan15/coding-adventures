import { defineConfig } from "vitest/config";
import path from "path";

export default defineConfig({
  server: {
    fs: {
      // Allow Vite to access files outside the package directory.
      // browser-transpiler.ts imports lattice.tokens and lattice.grammar
      // via `?raw` — these files live at code/grammars/ which is 4 levels
      // above this package (code/programs/typescript/lattice-docs/).
      // By default Vite only serves files within the project root; this
      // setting explicitly allows the repo root so the ?raw imports resolve.
      allow: [path.resolve(__dirname, "../../../.."), __dirname],
    },
  },
  test: {
    environment: "jsdom",
    globals: true,
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
