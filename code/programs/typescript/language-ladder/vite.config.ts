import { defineConfig } from "vite";
import path from "node:path";

// The per-letter script data (glyph, components, stroke order) is the SAME data
// the HL01 layer publishes, at code/learning/human-languages/data/scripts/*.json.
// We import those canonical files directly rather than copying them, so the app
// can never drift from the curriculum. They live outside this package's folder,
// so the dev server must be told the repo root is a legal place to read from.
const repoRoot = path.resolve(__dirname, "../../../..");

export default defineConfig({
  // Relative base → the built index.html works when opened from any path
  // (local file, GitHub Pages sub-path, etc.) without a deploy-specific prefix.
  base: "./",
  server: {
    fs: { allow: [repoRoot] },
  },
});
