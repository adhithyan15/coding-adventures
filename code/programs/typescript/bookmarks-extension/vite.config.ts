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
 * Extensions have MULTIPLE entry points (unlike a single-page web app):
 * - A popup (popup.html + popup.ts)
 * - A background service worker (service-worker.ts)
 *
 * We configure Vite's Rollup options to handle both entries.
 *
 * Static file handling
 * --------------------
 * Vite doesn't know about manifest.json or icon files — they aren't
 * imported by any module. A small plugin copies them into dist/ after
 * the build, rewriting manifest paths to match the compiled output.
 *
 * Build output
 * ------------
 * ```
 * dist/
 * ├── manifest.json      (copied + path-rewritten)
 * ├── src/popup/popup.html (processed popup page)
 * ├── popup.js            (compiled popup script)
 * ├── popup.css            (extracted styles)
 * ├── service-worker.js   (compiled background script)
 * └── icons/              (copied icon files)
 * ```
 */

/**
 * A Vite plugin that copies static extension files (manifest.json, icons)
 * into the output directory after the build.
 *
 * Why a plugin instead of Vite's publicDir?
 * Because we need to transform the manifest — the base manifest references
 * source paths like src/popup/popup.html, but the built extension has
 * them at the root (popup.html). We rewrite the paths during copy.
 */
function copyExtensionFiles(): Plugin {
  return {
    name: "copy-extension-files",
    writeBundle(options) {
      const outDir = options.dir ?? "dist";

      // Read the base manifest and rewrite paths for the built output.
      const manifest = JSON.parse(
        fs.readFileSync("manifest.json", "utf-8"),
      );

      // Update paths to match built output
      if (manifest.action?.default_popup) {
        manifest.action.default_popup = "src/popup/popup.html";
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
        popup: "src/popup/popup.html",
        "service-worker": "src/background/service-worker.ts",
      },
      output: {
        // Use flat file names instead of Vite's default hash-based names.
        // Extensions reference files by exact path in the manifest, so
        // we need predictable names.
        entryFileNames: "[name].js",
        chunkFileNames: "[name].js",
        assetFileNames: "[name].[ext]",
      },
    },
  },
  plugins: [copyExtensionFiles()],
});
