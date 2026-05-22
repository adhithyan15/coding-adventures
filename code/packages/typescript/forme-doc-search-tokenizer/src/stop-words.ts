/**
 * stop-words.ts — built-in English stop-word list.
 *
 * Small (~35-word) list of high-frequency English function words
 * that carry little search signal.  Filtering them shrinks the
 * search index meaningfully without hurting recall for typical
 * docs-site queries.
 *
 * The list is curated for technical-documentation context — it
 * deliberately KEEPS words like `not`, `no`, `do`, `if`,
 * `then` (often appearing in API descriptions where the
 * polarity matters).  It also keeps `how`, `what`, `when`,
 * `where`, `why` (question words common in FAQ/tutorial
 * queries).  Add more via `options.customStopWords` if your
 * domain calls for it.
 *
 * @module stop-words
 */

/**
 * The built-in stop-word set.  Frozen at module load so callers
 * can't accidentally pollute it (e.g. by treating the read-only
 * type contract as advisory).
 */
const STOP_WORDS_INTERNAL = new Set<string>([
  "a", "an", "and", "are", "as", "at",
  "be", "but", "by",
  "for", "from",
  "has", "have", "had", "he", "her", "him", "his",
  "in", "is", "it", "its",
  "of", "on", "or",
  "she",
  "that", "the", "their", "them", "these", "they", "this", "those", "to",
  "was", "were", "will", "with",
  "you", "your",
]);

/**
 * Read-only view of the built-in stop-word list.  Iteration
 * order is insertion order per ES spec.
 *
 * Note: `ReadonlySet<string>` is a TypeScript-only contract;
 * the underlying `Set` is technically mutable at runtime.  This
 * is the same trade-off as `forme-doc-syntax-highlighter`'s
 * `SUPPORTED_LANGUAGES` — callers casting away the type to
 * mutate the set are on their own; no security boundary
 * depends on its immutability.
 */
export const STOP_WORDS: ReadonlySet<string> = STOP_WORDS_INTERNAL;
