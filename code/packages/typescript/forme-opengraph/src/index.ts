/**
 * @coding-adventures/forme-opengraph
 *
 * OpenGraph + Twitter Card + basic `<meta>` tag generators for the
 * Forme pipeline (FM00 v0 SEO stage).
 *
 * Pure transform — meta records → reproducible HTML
 * `<meta>` / `<link>` / `<title>` tag sequence.
 *
 * ```ts
 * import { generateMetaTags } from "@coding-adventures/forme-opengraph";
 *
 * const head = generateMetaTags({
 *   basic: {
 *     title: "Hello World",
 *     description: "A first post",
 *     canonical: "https://example.com/hello",
 *   },
 *   og: {
 *     title: "Hello World",
 *     type: "article",
 *     image: "https://example.com/og.png",
 *     url: "https://example.com/hello",
 *   },
 *   twitter: {
 *     card: "summary_large_image",
 *     site: "@example",
 *   },
 * });
 * ```
 *
 * @module index
 */

export { generateOpenGraphTags } from "./opengraph.js";
export { generateTwitterCardTags } from "./twitter.js";
export { generateBasicTags } from "./basic.js";
export { generateMetaTags } from "./combined.js";
export type { CombinedMeta } from "./combined.js";
export type { OpenGraphMeta, TwitterCardMeta, BasicMeta } from "./types.js";
export { escapeHtmlAttr, escapeHtmlText, assertAbsoluteUrl } from "./escape.js";
