import { defineConfig, type Plugin } from "vite";
import * as fs from "node:fs";
import * as path from "node:path";

/**
 * Vite Configuration — Bookmarks Extension
 * ==========================================
 *
 * How Vite builds this extension
 * ------------------------------
 * Browser extensions are loaded by the browser as plain HTML, CSS, and JS.
 * Vite transforms our TypeScript source into JavaScript the browser can run.
 *
 * This extension has two entry points:
 * - A side panel (panel.html + panel.ts) — the main UI
 * - A background service worker (service-worker.ts) — opens the side panel
 *
 * Build output
 * ------------
 * ```
 * dist/
 * ├── manifest.json          (copied + path-rewritten)
 * ├── src/panel/panel.html   (processed panel page)
 * ├── panel.js               (compiled panel script)
 * ├── panel.css              (extracted styles)
 * ├── service-worker.js      (compiled background script)
 * └── icons/                 (copied icon files)
 * ```
 */

/**
 * A Vite plugin that copies static extension files (manifest.json, icons)
 * into the output directory after the build.
 *
 * Rewrites manifest paths to match the compiled output:
 * - side_panel.default_path: src/panel/panel.html (preserved by Vite)
 * - sidebar_action.default_panel: src/panel/panel.html (preserved)
 * - background.service_worker: service-worker.js (compiled from .ts)
 */
function copyExtensionFiles(): Plugin {
  return {
    name: "copy-extension-files",
    writeBundle(options) {
      const outDir = options.dir ?? "dist";

      const manifest = JSON.parse(
        fs.readFileSync("manifest.json", "utf-8"),
      );

      // Rewrite paths for the built output
      if (manifest.side_panel?.default_path) {
        manifest.side_panel.default_path = "src/panel/panel.html";
      }
      if (manifest.sidebar_action?.default_panel) {
        manifest.sidebar_action.default_panel = "src/panel/panel.html";
      }
      if (manifest.background?.service_worker) {
        manifest.background.service_worker = "service-worker.js";
      }

      fs.writeFileSync(
        path.join(outDir, "manifest.json"),
        JSON.stringify(manifest, null, 2),
      );

      // Copy icons
      const iconsDir = path.join(outDir, "icons");
      if (!fs.existsSync(iconsDir)) {
        fs.mkdirSync(iconsDir, { recursive: true });
      }
      for (const file of fs.readdirSync("icons")) {
        fs.copyFileSync(
          path.join("icons", file),
          path.join(iconsDir, file),
        );
      }
    },
  };
}

export default defineConfig({
  build: {
    outDir: "dist",
    // Disable minification for readability — this is a learning project.
    minify: false,
    rollupOptions: {
      input: {
        panel: "src/panel/panel.html",
        "service-worker": "src/background/service-worker.ts",
      },
      output: {
        entryFileNames: "[name].js",
        chunkFileNames: "[name].js",
        assetFileNames: "[name].[ext]",
      },
    },
  },
  plugins: [copyExtensionFiles()],
});
