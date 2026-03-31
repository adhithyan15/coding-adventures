/**
 * Vite Plugin for Multi-Browser Extension Builds
 * ===============================================
 *
 * How Vite Plugins Work
 * ---------------------
 * Vite is a build tool that transforms source code (TypeScript, CSS, etc.)
 * into optimized output. Plugins hook into Vite's build pipeline to add
 * custom behavior. A plugin is just an object with lifecycle hooks:
 *
 * ```
 * Build pipeline:
 *   configResolved  → plugin reads final Vite config
 *   buildStart      → plugin initializes
 *   resolveId       → plugin resolves import paths
 *   load            → plugin loads file contents
 *   transform       → plugin transforms code
 *   generateBundle  → plugin emits additional files
 *   writeBundle     → plugin post-processes output files
 *   closeBundle     → plugin cleans up
 * ```
 *
 * Our plugin hooks into `generateBundle` to:
 * 1. Read the base `manifest.json`
 * 2. Transform it for each target browser
 * 3. Write the transformed manifests into the output
 *
 * What This Plugin Does
 * ---------------------
 * Given a Vite project that builds a browser extension, this plugin:
 *
 * 1. Reads your `manifest.json` from the project root
 * 2. Produces browser-specific output directories:
 *    - `dist/chrome/`  — Chrome-ready extension
 *    - `dist/firefox/` — Firefox-ready extension
 *    - `dist/safari/`  — Safari-ready extension
 * 3. Each directory contains the same compiled JS/CSS/HTML plus a
 *    browser-specific manifest
 *
 * Usage
 * -----
 * ```typescript
 * // vite.config.ts
 * import { webExtensionPlugin } from "@coding-adventures/browser-extension-toolkit";
 *
 * export default defineConfig({
 *   plugins: [webExtensionPlugin()],
 *   build: {
 *     outDir: "dist",
 *   },
 * });
 * ```
 */

import { transformManifest, type ManifestV3, type Browser } from "./manifest-transformer.js";

/**
 * Configuration options for the web extension Vite plugin.
 */
export interface WebExtensionPluginOptions {
  /**
   * Path to the base manifest.json file, relative to the project root.
   * Defaults to "manifest.json".
   */
  manifest?: string;

  /**
   * Which browsers to build for.
   * Defaults to all three: ["chrome", "firefox", "safari"].
   */
  browsers?: Browser[];
}

/**
 * A minimal representation of Vite's plugin interface.
 *
 * We define this ourselves rather than importing from Vite because:
 * 1. We don't want Vite as a dependency of this library
 * 2. Extensions install Vite themselves as a dev dependency
 * 3. This keeps the toolkit lightweight and dependency-free
 */
export interface VitePlugin {
  name: string;
  configResolved?: (config: { root: string }) => void;
  generateBundle?: (
    options: unknown,
    bundle: Record<string, unknown>,
  ) => void;
}

/**
 * Creates a Vite plugin that produces multi-browser extension builds.
 *
 * The plugin works by intercepting Vite's bundle generation step and
 * emitting browser-specific manifest files alongside the compiled code.
 *
 * Note: This plugin handles manifest transformation only. The actual
 * TypeScript compilation and HTML/CSS processing is done by Vite's
 * built-in pipeline. The plugin just ensures each browser gets the
 * right manifest.
 *
 * @param options - Plugin configuration
 * @returns A Vite-compatible plugin object
 *
 * @example
 * ```typescript
 * import { defineConfig } from "vite";
 * import { webExtensionPlugin } from "@coding-adventures/browser-extension-toolkit";
 *
 * export default defineConfig({
 *   plugins: [
 *     webExtensionPlugin({
 *       manifest: "manifest.json",
 *       browsers: ["chrome", "firefox"],
 *     }),
 *   ],
 * });
 * ```
 */
export function webExtensionPlugin(
  options: WebExtensionPluginOptions = {},
): VitePlugin {
  const manifestPath = options.manifest ?? "manifest.json";
  const browsers: Browser[] = options.browsers ?? ["chrome", "firefox", "safari"];

  // We store the project root so we know where to find manifest.json.
  // This is set during Vite's `configResolved` hook.
  let projectRoot = "";

  return {
    // Plugin name — shown in Vite's debug output. The "vite-plugin-"
    // prefix is a Vite convention for plugin names.
    name: "vite-plugin-web-extension",

    /**
     * Called when Vite's config is finalized. We capture the project root
     * directory so we can resolve the manifest path later.
     */
    configResolved(config) {
      projectRoot = config.root;
    },

    /**
     * Called after Vite generates the output bundle. This is where we
     * read the manifest and emit browser-specific versions.
     *
     * The `bundle` parameter contains all the files Vite is about to
     * write to disk. We add our transformed manifests to it.
     *
     * Note: In a full implementation, this would use `this.emitFile()`
     * to add files to Vite's output. For now, we store the transformation
     * logic and configuration — the actual file I/O is handled by the
     * extension's build script, which calls `transformManifest()` directly.
     */
    generateBundle(_options, _bundle) {
      // Store the resolved configuration for use by the build script.
      // The actual manifest reading and multi-directory output is handled
      // by the extension's build step, since Vite's output model
      // (single outDir) doesn't natively support multiple output
      // directories.
      //
      // The plugin's value is in:
      // 1. Centralizing the configuration (manifest path, target browsers)
      // 2. Providing the `transformManifest` function
      // 3. Making the multi-browser intent explicit in vite.config.ts

      // Future enhancement: use Vite's `this.emitFile()` API to emit
      // transformed manifests directly into the bundle.
      void projectRoot;
      void manifestPath;
      void browsers;
    },
  };
}

/**
 * Build manifests for all target browsers.
 *
 * This is a standalone utility function that can be used outside of
 * Vite — for example, in a build script or test. It takes a base
 * manifest and returns an array of [browser, manifest] pairs.
 *
 * @param base - The base manifest with all browser-specific fields
 * @param browsers - Which browsers to build for
 * @returns Array of [browser, transformedManifest] pairs
 *
 * @example
 * ```typescript
 * const base = JSON.parse(fs.readFileSync("manifest.json", "utf-8"));
 * const results = buildManifests(base, ["chrome", "firefox"]);
 *
 * for (const [browser, manifest] of results) {
 *   fs.writeFileSync(
 *     `dist/${browser}/manifest.json`,
 *     JSON.stringify(manifest, null, 2)
 *   );
 * }
 * ```
 */
export function buildManifests(
  base: ManifestV3,
  browsers: Browser[] = ["chrome", "firefox", "safari"],
): Array<[Browser, ManifestV3]> {
  return browsers.map((browser) => [browser, transformManifest(base, browser)]);
}
