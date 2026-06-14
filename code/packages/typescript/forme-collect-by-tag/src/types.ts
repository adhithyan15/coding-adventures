/**
 * types.ts — public signatures for the tag collector.
 *
 * The collector is generic over the item type `T` so it works with
 * `Document`, `IndexItem`, `ContentNode`, or any caller-defined
 * shape — only requirement is a `tagsOf(item)` accessor returning
 * `readonly string[] | undefined`.
 *
 * @module types
 */

/**
 * Accessor used by `collectByTag` to pull the raw tag list off an
 * item.  Should be pure (same item → same tag list) for the
 * collector's output to be deterministic.
 *
 * Return shape:
 *   - `readonly string[]` — the item's declared tags, in any order.
 *   - `undefined` (or `null`) — item has no `tags` field at all.
 *   - empty array `[]` — item explicitly declares no tags.
 *
 * The collector treats `undefined` / `null` / `[]` identically:
 * the item is "untagged".  Whether untagged items appear in the
 * output depends on `CollectByTagOptions.includeUntagged`.
 */
export type TagsOf<T> = (item: T) => readonly string[] | undefined | null;

/**
 * Options controlling the grouping behaviour.
 *
 *   - `tagsOf` (required) — see `TagsOf`.
 *   - `sortBy` (optional) — comparator used to sort items WITHIN
 *     each bucket.  Defaults to "preserve input order".  Common
 *     choice for blog posts: `(a, b) => b.pubDate.localeCompare(a.pubDate)`
 *     (newest first).
 *   - `includeUntagged` (optional, default `false`) — when `true`,
 *     items with no tags land in a special bucket whose key is
 *     `untaggedBucketName`.
 *   - `untaggedBucketName` (optional, default `"untagged"`) —
 *     the synthetic bucket name for items with no tags.
 */
export interface CollectByTagOptions<T> {
  readonly tagsOf: TagsOf<T>;
  readonly sortBy?: (a: T, b: T) => number;
  readonly includeUntagged?: boolean;
  readonly untaggedBucketName?: string;
}

/**
 * Result of `collectByTag`.
 *
 *   - `byTag` — `Map<normalisedTag, items[]>`.  Iteration order is
 *     first-seen-tag order (matches input traversal).
 *   - `tagNames` — alphabetically-sorted array of all bucket
 *     names.  Useful for "browse all tags" navigation widgets;
 *     keeps callers from re-sorting `byTag.keys()` themselves.
 *
 * Why both?  `byTag` preserves the input-traversal ordering
 * (useful for "first 5 buckets we encountered" pagination);
 * `tagNames` is the deterministic-alphabetical view for tag
 * indexes.  Either alone would force callers to redo work.
 */
export interface CollectByTagResult<T> {
  readonly byTag: ReadonlyMap<string, readonly T[]>;
  readonly tagNames: readonly string[];
}
