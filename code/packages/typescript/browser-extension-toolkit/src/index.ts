/**
 * Browser Extension Toolkit
 * =========================
 *
 * A cross-browser toolkit for building browser extensions that ship to
 * Chrome, Firefox, and Safari from a single codebase.
 *
 * The Problem
 * -----------
 * Browser extensions are fundamentally just HTML, CSS, and JavaScript. But
 * each browser has slightly different APIs and manifest requirements:
 *
 * - Chrome uses `chrome.*` namespace, Firefox/Safari use `browser.*`
 * - Firefox requires `browser_specific_settings.gecko` in the manifest
 * - Safari needs the extension wrapped in a native macOS/iOS app
 *
 * The Solution
 * ------------
 * Write your extension once, then use this toolkit to produce
 * browser-specific builds:
 *
 * 1. **Browser API Shim** — Import `browserAPI` instead of `chrome` or
 *    `browser`. Works everywhere.
 *
 * 2. **Manifest Transformer** — Write one `manifest.json`. The transformer
 *    produces Chrome, Firefox, and Safari variants automatically.
 *
 * 3. **Vite Plugin** — Orchestrates the multi-browser build. Compiles
 *    TypeScript, transforms manifests, outputs to `dist/<browser>/`.
 *
 * 4. **Scaffold Generator** — Creates a new extension project with all
 *    the boilerplate already wired up.
 *
 * @example
 * ```typescript
 * import {
 *   getBrowserAPI,
 *   transformManifest,
 *   webExtensionPlugin,
 *   scaffold,
 * } from "@coding-adventures/browser-extension-toolkit";
 * ```
 */

export { getBrowserAPI } from "./browser-api.js";
export type { BrowserAPI } from "./browser-api.js";

export { transformManifest } from "./manifest-transformer.js";
export type { ManifestV3, Browser } from "./manifest-transformer.js";

export { webExtensionPlugin } from "./vite-plugin.js";
export type { WebExtensionPluginOptions } from "./vite-plugin.js";

export { scaffold } from "./scaffold/scaffold.js";
export type { ScaffoldOptions } from "./scaffold/scaffold.js";
