/**
 * @coding-adventures/forme-doc-search-tokenizer
 *
 * Text → tokens pipeline for the documentation-site search index.
 *
 *   1. Lowercase (locale-independent — `toLowerCase`, NOT
 *      `toLocaleLowerCase`, so the index stays stable across
 *      machines).
 *   2. Strip non-alphanumeric Unicode (keep `\p{L}`, `\p{N}`,
 *      and `_`).
 *   3. Split on whitespace/punctuation runs.
 *   4. (Optional) Filter stop-words from a ~35-word English
 *      built-in list — or a caller-supplied custom list.
 *   5. (Optional) Reduce each surviving token to its Porter stem.
 *
 * Pure transform.  Capabilities: `[]`.  **Zero runtime
 * dependencies.**
 *
 * Used at BUILD time by `forme-doc-search-index-builder` to
 * tokenise document text into postings, and at RUNTIME (in the
 * browser) by `forme-doc-search-client-js` to tokenise user
 * queries.  Both sides MUST agree on the `stem` /
 * `filterStopWords` flags, or query tokens won't match the
 * index.
 *
 * ```ts
 * import { tokenize } from "@coding-adventures/forme-doc-search-tokenizer";
 *
 * tokenize("Hello, World!");
 * // → ["hello", "world"]
 *
 * tokenize("the quick brown fox", { filterStopWords: true });
 * // → ["quick", "brown", "fox"]      (drops "the")
 *
 * tokenize("running and walking", { stem: true });
 * // → ["run", "and", "walk"]
 *
 * tokenize("running and walking", { filterStopWords: true, stem: true });
 * // → ["run", "walk"]
 * ```
 *
 * Eighth concrete DOC00 v0 package (after frontmatter,
 * heading-anchors, toc-extractor, code-block-decorator,
 * syntax-highlighter, sidebar-builder, page-shell).
 *
 * @module index
 */

export { tokenize } from "./tokenize.js";
export { normaliseToTokens } from "./normalise.js";
export { porterStem } from "./porter.js";
export { STOP_WORDS } from "./stop-words.js";
export type { TokenizeOptions } from "./types.js";
