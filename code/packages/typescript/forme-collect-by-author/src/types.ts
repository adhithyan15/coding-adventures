/**
 * types.ts — public signatures for the author collector.
 *
 * Mirrors `forme-collect-by-tag` but the accessor surface is
 * subtly different: an item commonly has a single primary author
 * (`author: "Ada Lovelace"`) but may also have co-authors
 * (`authors: ["Ada", "Charles"]`).  The `authorOf` accessor
 * returns either form so the collector can handle both without
 * forcing callers to normalise upstream.
 *
 * @module types
 */

/**
 * Accessor used by `collectByAuthor` to pull the author name(s)
 * off an item.  Should be pure (same item → same authors every
 * call) for the collector's output to be deterministic.
 *
 * Return shape:
 *   - `string` — single author (most common case).
 *   - `readonly string[]` — co-authors (each gets its own
 *     bucket; the item appears in every author's bucket).
 *   - `null` / `undefined` — anonymous (no author).
 *   - empty `""` or `[]` — also anonymous (no meaningful name).
 *
 * The collector treats `null` / `undefined` / `""` / `[]`
 * identically: the item is anonymous.  Whether anonymous items
 * appear in the output depends on
 * `CollectByAuthorOptions.includeAnonymous`.
 */
export type AuthorOf<T> = (item: T) => string | readonly string[] | null | undefined;

/**
 * Options controlling the grouping behaviour.
 *
 *   - `authorOf` (required) — see `AuthorOf`.
 *   - `sortBy` (optional) — comparator used to sort items WITHIN
 *     each bucket.  Defaults to "preserve input order".  Common
 *     choice: newest-first by publication date.
 *   - `includeAnonymous` (optional, default `false`) — when
 *     `true`, items with no author land in a special bucket
 *     whose key is `anonymousBucketName`.
 *   - `anonymousBucketName` (optional, default `"anonymous"`) —
 *     the synthetic bucket name for items with no author.
 */
export interface CollectByAuthorOptions<T> {
  readonly authorOf: AuthorOf<T>;
  readonly sortBy?: (a: T, b: T) => number;
  readonly includeAnonymous?: boolean;
  readonly anonymousBucketName?: string;
}

/**
 * Result of `collectByAuthor`.
 *
 *   - `byAuthor` — `Map<normalisedAuthor, items[]>`.  Iteration
 *     order is first-seen-author order (matches input traversal).
 *   - `authorNames` — alphabetically-sorted array of all bucket
 *     names.  Useful for "browse all authors" navigation
 *     widgets.
 *
 * Why both?  `byAuthor` preserves input-traversal ordering;
 * `authorNames` is the deterministic-alphabetical view.  Either
 * alone would force callers to redo work.
 */
export interface CollectByAuthorResult<T> {
  readonly byAuthor: ReadonlyMap<string, readonly T[]>;
  readonly authorNames: readonly string[];
}
