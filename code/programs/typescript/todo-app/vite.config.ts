import { defineConfig } from "vite";
import { resolve } from "path";
import react from "@vitejs/plugin-react";
import { latticePlugin } from "../../../packages/typescript/vite-plugin-lattice/src/index.js";

// Base path for all packages in the monorepo
const pkgsBase = resolve(__dirname, "../../../packages/typescript");

export default defineConfig({
  plugins: [react(), latticePlugin()],
  base: "./",

  resolve: {
    alias: {
      // Map @coding-adventures/* bare specifiers to their actual source paths.
      // This is needed because the Vite plugin uses ssrLoadModule to load
      // the lattice-transpiler, which imports from these packages.
      // Without aliases, Vite's SSR resolver only looks in the todo-app's
      // node_modules (where these aren't direct deps).
      "@coding-adventures/grammar-tools": resolve(pkgsBase, "grammar-tools/src/index.ts"),
      "@coding-adventures/lexer": resolve(pkgsBase, "lexer/src/index.ts"),
      "@coding-adventures/parser": resolve(pkgsBase, "parser/src/index.ts"),
      "@coding-adventures/lattice-ast-to-css": resolve(pkgsBase, "lattice-ast-to-css/src/index.ts"),
      "@coding-adventures/lattice-lexer": resolve(pkgsBase, "lattice-lexer/src/index.ts"),
      "@coding-adventures/lattice-parser": resolve(pkgsBase, "lattice-parser/src/index.ts"),
      "@coding-adventures/lattice-transpiler": resolve(pkgsBase, "lattice-transpiler/src/index.ts"),
      "@coding-adventures/directed-graph": resolve(pkgsBase, "directed-graph/src/index.ts"),
      "@coding-adventures/state-machine": resolve(pkgsBase, "state-machine/src/index.ts"),
    },
  },
});
