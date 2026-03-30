import { defineConfig } from "vite";

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
});
