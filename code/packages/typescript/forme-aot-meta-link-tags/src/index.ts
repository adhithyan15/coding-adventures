/**
 * @coding-adventures/forme-aot-meta-link-tags
 *
 * Emit HTML `<head>` tags from a structured config — canonical,
 * pagination (prev/next), favicons (icons), resource hints
 * (preload/prefetch/preconnect/dns-prefetch/modulepreload),
 * and arbitrary `<meta name|http-equiv>` pairs.
 *
 * Pure transform.  Validates URLs (http(s):// or
 * root-relative), allowlists rel / as / crossorigin,
 * HTML-attribute-escapes every value.  Two-pass fail-fast:
 * any validation failure throws `TypeError` BEFORE any tag
 * string is emitted.
 *
 * ```ts
 * import { generateMetaLinkTags } from "@coding-adventures/forme-aot-meta-link-tags";
 *
 * const head = generateMetaLinkTags({
 *   canonical: "https://example.com/post-1",
 *   prev: "/post-0",
 *   next: "/post-2",
 *   meta: [
 *     { name: "viewport", content: "width=device-width, initial-scale=1" },
 *     { name: "description", content: "A blog post." },
 *   ],
 *   icons: [
 *     { href: "/favicon.svg", type: "image/svg+xml" },
 *     { href: "/apple-touch-icon.png", rel: "apple-touch-icon", sizes: "180x180" },
 *   ],
 *   preload: [
 *     { href: "/main.js", rel: "preload", as: "script" },
 *     { href: "https://fonts.example.com", rel: "preconnect", crossorigin: "anonymous" },
 *   ],
 * });
 * // <meta name="viewport" content="...">
 * // <meta name="description" content="A blog post.">
 * // <link rel="canonical" href="https://example.com/post-1">
 * // <link rel="prev" href="/post-0">
 * // <link rel="next" href="/post-2">
 * // <link rel="icon" type="image/svg+xml" href="/favicon.svg">
 * // <link rel="apple-touch-icon" sizes="180x180" href="/apple-touch-icon.png">
 * // <link rel="preload" as="script" href="/main.js">
 * // <link rel="preconnect" crossorigin="anonymous" href="https://fonts.example.com">
 * ```
 *
 * Fifteenth FM00 v0 stage package.
 *
 * @module index
 */

export { generateMetaLinkTags } from "./generate.js";
export { escapeHtmlAttr, stripAsciiControl } from "./escape.js";
export {
  validateUrl,
  validateIconRel,
  validateHintRel,
  validateHintAs,
  validateCrossOrigin,
  validateOptionalString,
} from "./validate.js";
export type {
  MetaLinkConfig,
  IconLink,
  IconRel,
  ResourceHint,
  ResourceHintRel,
  ResourceHintAs,
  CrossOrigin,
  MetaTag,
} from "./types.js";
