/**
 * @coding-adventures/forme-transform-typography
 *
 * FM00 v0 §5.3 transform — apply smart-quote / em-dash / en-dash /
 * ellipsis (and optional ligature) substitution to every
 * `TextNode` in a `DocumentNode`.
 *
 * Pure transform: walks the input document and returns a fresh
 * `DocumentNode` with typography corrections applied to prose
 * text.  Code blocks, code spans, raw HTML, URLs, and image
 * alt-text pass through unchanged (smart-quoting code samples
 * would break syntax).
 *
 * ```ts
 * import { typography } from "@coding-adventures/forme-transform-typography";
 *
 * const prettified = typography(doc);
 * // doc:        "He said \"hello\" -- don't worry, it's fine..."
 * // prettified: "He said “hello” – don’t worry, it’s fine…"
 * ```
 *
 * Implementation: single-pass O(N) character loop with
 * `charCodeAt`-based lookahead.  Zero regex backtracking
 * surface — trivially passes ReDoS analysis.
 *
 * Seventh FM00 v0 stage package — joins `forme-feeds`,
 * `forme-opengraph`, `forme-index-renderer`, `forme-transforms`,
 * `forme-transform-autolink-headings`, `forme-transform-toc`.
 *
 * @module index
 */

export { typography } from "./walk.js";
export { typeset } from "./typeset.js";
export type { TypographyOptions } from "./types.js";
