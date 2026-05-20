/**
 * @coding-adventures/forme-aot-html-doc-emitter
 *
 * Final assembly stage of the FM00 v0 head / body pipeline:
 * wrap pre-built `<head>` + `<body>` string chunks (already
 * produced by the sibling emitters — meta-link-tags, style-tag,
 * script-tag, rss-discovery-link, etc.) into a complete
 * `<!doctype html>...</html>` document with optional `lang`,
 * `dir`, and extra `<html>` / `<body>` attribute maps.
 *
 * Pure transform.  `head` and `body` are passthrough strings
 * (already trusted FM00 output); attribute maps DO get full
 * validation — keys constrained to lowercase ASCII identifiers
 * + dashes, `on*` event-handler namespace rejected outright,
 * reserved attrs (`lang`, `dir`, `xmlns`) rejected so the
 * dedicated config fields stay the single source of truth.
 *
 * ```ts
 * import { generateHtmlDocument } from "@coding-adventures/forme-aot-html-doc-emitter";
 *
 * generateHtmlDocument({
 *   lang: "en-US",
 *   dir: "ltr",
 *   head: "<title>Hello</title>",
 *   body: "<h1>Hello, world!</h1>",
 *   htmlAttrs: { "data-theme": "dark" },
 *   bodyAttrs: { class: "page" },
 * });
 * ```
 *
 * Eighteenth FM00 v0 stage package.
 *
 * @module index
 */

export { generateHtmlDocument } from "./generate.js";
export { escapeHtmlAttr, stripAsciiControl } from "./escape.js";
export {
  validateLang,
  validateDir,
  validateAttrKey,
  validateAttrValue,
} from "./validate.js";
export type { HtmlDocConfig, DocDirection } from "./types.js";
