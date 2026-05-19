/**
 * @coding-adventures/forme-aot-manifest-emitter
 *
 * Emit web app `manifest.json` from a structured
 * `WebAppManifest` config per https://www.w3.org/TR/appmanifest/.
 * Pure transform — returns JSON.stringify-ed string with
 * sorted keys for deterministic, diff-friendly output.
 *
 * ```ts
 * import { generateManifest } from "@coding-adventures/forme-aot-manifest-emitter";
 *
 * const json = generateManifest({
 *   name: "My App",
 *   short_name: "App",
 *   start_url: "/",
 *   display: "standalone",
 *   theme_color: "#0066cc",
 *   background_color: "#ffffff",
 *   icons: [
 *     { src: "/icon-192.png", sizes: "192x192", type: "image/png" },
 *     { src: "/icon-512.png", sizes: "512x512", type: "image/png" },
 *   ],
 * });
 *
 * fs.writeFileSync("dist/manifest.json", json);
 * ```
 *
 * Validation runs BEFORE emission.  Bad URL schemes, bad
 * display values, bad hex colours all throw `TypeError`
 * synchronously; callers never see a partial manifest.
 *
 * Thirteenth FM00 v0 stage package — joins the FM00 v0 cluster.
 *
 * @module index
 */

export { generateManifest } from "./generate.js";
export {
  validateManifestUrl,
  validateDisplay,
  validateColor,
} from "./validate.js";
export type {
  DisplayMode,
  ManifestIcon,
  WebAppManifest,
} from "./types.js";
