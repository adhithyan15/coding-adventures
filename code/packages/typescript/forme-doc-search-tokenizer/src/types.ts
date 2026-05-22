/**
 * types.ts — public signatures for the search tokeniser.
 *
 * @module types
 */

/**
 * Optional tokenisation configuration.
 */
export interface TokenizeOptions {
  /**
   * When `true`, tokens matching the (built-in or custom)
   * stop-word list are filtered out.  Default: `false` — the
   * caller decides whether to drop high-frequency function
   * words.  Indexing typically wants `true` to shrink the
   * index; query-time tokenising might want `false` to preserve
   * exact-match semantics for explicit-keyword queries.
   */
  readonly filterStopWords?: boolean;

  /**
   * When `true`, each surviving token is reduced to its Porter
   * stem (e.g. `"running"` → `"run"`, `"happiness"` → `"happi"`).
   * Default: `false`.  Indexing and query-tokenising sides MUST
   * agree on this option, or queries won't match the index.
   */
  readonly stem?: boolean;

  /**
   * Override the built-in English stop-word list.  Only consulted
   * when `filterStopWords` is `true`.  Pass `new Set()` to filter
   * NO words (effectively disabling the filter).  Word
   * comparison is exact-match on the already-lowercased token.
   *
   * Default: the built-in `STOP_WORDS` set.
   */
  readonly customStopWords?: ReadonlySet<string>;
}
