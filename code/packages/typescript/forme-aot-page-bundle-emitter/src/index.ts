/**
 * @coding-adventures/forme-aot-page-bundle-emitter
 *
 * Deploy-stage emitter: take an array of HTML pages with their
 * routes, produce a deterministic JSON manifest describing
 * where each page should be written and its SHA-256 content
 * hash.  The downstream deploy tool reads the manifest and
 * writes the actual files.
 *
 * Pure transform.  Uses Node's built-in `node:crypto` for
 * SHA-256 hashing.  No I/O, no fs, no network, no env, no
 * shell.  Capabilities: `[]`.
 *
 * ```ts
 * import { generatePageBundle } from "@coding-adventures/forme-aot-page-bundle-emitter";
 *
 * const manifest = generatePageBundle({
 *   baseUrl: "https://example.com",
 *   pages: [
 *     { route: "/",         html: indexHtml },
 *     { route: "/about",    html: aboutHtml },
 *     { route: "/feed.xml", html: rssXml, contentType: "application/rss+xml" },
 *   ],
 * });
 * // → JSON string with sorted routes, each entry having
 * //   { route, outputPath, contentType, sizeBytes, sha256 [, lastmod] }
 * ```
 *
 * Nineteenth FM00 v0 stage package.
 *
 * @module index
 */

export { generatePageBundle } from "./generate.js";
export { routeToOutputPath } from "./path.js";
export { sha256Base64, utf8ByteLength } from "./hash.js";
export {
  validateRoute,
  validateBaseUrl,
  validateString,
} from "./validate.js";
export type {
  PageBundleConfig,
  PageEntry,
  RouteEntry,
  PageBundleManifest,
} from "./types.js";
