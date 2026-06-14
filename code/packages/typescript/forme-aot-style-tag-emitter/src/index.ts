/**
 * @coding-adventures/forme-aot-style-tag-emitter
 *
 * Emit HTML `<link rel="stylesheet">` and inline `<style>` tag
 * strings from a structured `StyleConfig`.  Pure transform.
 *
 * External `<link>` entries support SRI integrity (sha256 /
 * sha384 / sha512 with per-algo length + padding validation,
 * Map-backed algo lookup), crossorigin, media query, and the
 * `disabled` boolean.  Inline `<style>` entries support a media
 * query and reject any literal `</style>` sequence in the body.
 *
 * ```ts
 * import { generateStyleTags } from "@coding-adventures/forme-aot-style-tag-emitter";
 *
 * generateStyleTags({
 *   stylesheets: [
 *     { href: "/main.css" },
 *     { href: "https://cdn.example.com/print.css", media: "print",
 *       integrity: "sha384-...", crossorigin: "anonymous" },
 *   ],
 *   inline: [
 *     { css: ":root { --c: blue; }" },
 *     { media: "(prefers-color-scheme: dark)", css: ":root { --c: lightblue; }" },
 *   ],
 * });
 * ```
 *
 * Seventeenth FM00 v0 stage package.
 *
 * @module index
 */

export { generateStyleTags } from "./generate.js";
export { escapeHtmlAttr, stripAsciiControl } from "./escape.js";
export {
  validateStyleHref,
  validateIntegrity,
  validateCrossOrigin,
  validateInlineCss,
  validateOptionalString,
} from "./validate.js";
export type {
  StyleConfig,
  StylesheetLink,
  InlineStyle,
  CrossOrigin,
} from "./types.js";
