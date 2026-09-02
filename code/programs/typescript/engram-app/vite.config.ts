import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  resolve: {
    preserveSymlinks: true,
    // Force ONE copy of React into the bundle.
    //
    // `preserveSymlinks` above makes Vite resolve a `file:`-linked package's
    // imports from inside that package's own `node_modules`. The deploy
    // workflow runs `npm install` in `store` and `ui-components` BEFORE the
    // app's install, so each of them ends up with its own `node_modules/react`
    // -- three copies in total, and Vite bundles all three.
    //
    // React's hooks read from module-level internals, so a component rendered
    // by one copy calling a hook from another gets null:
    //
    //   TypeError: Cannot read properties of null (reading 'useSyncExternalStore')
    //
    // which is what the deployed app threw, leaving a blank page. It never
    // reproduced from a plain `npm install` in this directory, because npm
    // dedupes when it installs the whole tree at once -- only the workflow's
    // install order produces it.
    dedupe: ["react", "react-dom"],
  },
  // Base path is configurable via VITE_BASE environment variable.
  //
  // Default "./" uses relative paths — works everywhere:
  //   - `npm run dev`  → Vite dev server (http://localhost:5173)
  //   - Electron       → file:// protocol, no web server needed
  //
  // Set VITE_BASE=/coding-adventures/engram/ for the GitHub Pages build
  // (the deploy workflow sets this automatically).
  base: process.env.VITE_BASE ?? "./",
});
