import { defineConfig, type Plugin } from "vite";
import * as fs from "node:fs";
import * as path from "node:path";

/**
 * Vite Configuration — Hello World Extension
 * ===========================================
 *
 * How Vite builds a browser extension
 * ------------------------------------
 * Browser extensions are loaded by the browser as plain HTML, CSS, and JS.
 * Vite's job is to transform our TypeScript source into JavaScript the
 * browser can execute.
 *
 * The key difference from a web app: extensions have MULTIPLE entry points.
 * A web app has one `index.html`. An extension has:
 * - A popup (popup.html + popup.ts)
 * - A background service worker (service-worker.ts)
 * - Optionally, content scripts (injected into web pages)
 *
 * We configure Vite's Rollup options to handle these multiple entries.
 *
 * Copying static files
 * --------------------
 * Vite doesn't know about `manifest.json` or icon files — those aren't
 * imported by any JavaScript module. We use a small plugin to copy them
 * into `dist/` after the build completes. This ensures the output
 * directory is a complete, loadable extension.
 *
 * Build output
 * ------------
 * The build produces a `dist/` directory that can be loaded directly as
 * an unpacked extension in Chrome or Firefox:
 *
 * ```
 * dist/
 * ├── manifest.json     (copied from project root)
 * ├── popup.html        (processed popup page)
 * ├── popup.js          (compiled popup script)
 * ├── service-worker.js (compiled background script)
 * └── icons/            (copied icon files)
 * ```
 */

/**
 * A Vite plugin that copies static extension files (manifest.json, icons)
 * into the output directory after the build.
 *
 * Why a plugin instead of Vite's `publicDir`?
 * Because we need to transform the manifest — the base manifest references
 * source paths like `src/popup/popup.html`, but the built extension has
 * them at the root (`popup.html`). We rewrite the paths during copy.
 */
function copyExtensionFiles(): Plugin {
  return {
    name: "copy-extension-files",
    writeBundle(options) {
      const outDir = options.dir ?? "dist";

      // Read and transform the manifest for the built output.
      // The source manifest references `src/popup/popup.html` and
      // `src/background/service-worker.ts`, but after Vite builds,
      // these become `src/popup/popup.html` (Vite preserves the HTML
      // path structure) and `service-worker.js`.
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
    // Production extensions should enable minification.
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
