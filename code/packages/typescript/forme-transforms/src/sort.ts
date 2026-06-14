/**
 * sort.ts — deterministic, stable `sortBy` over key functions.
 *
 * V8's `Array.prototype.sort` has been stable since 2018 (Node ≥10),
 * but we re-implement on top of it with two non-default behaviours
 * that the Forme pipeline requires:
 *
 *   1. **`null` / `undefined` keys sort LAST** regardless of
 *      direction.  Asking for "newest first" should put undated
 *      items at the end of the list, not at the top (where they
 *      would land if `null` were compared as zero or NaN).
 *   2. **Deterministic tiebreaker via the original index.**  Even
 *      though V8 sort IS stable, we lock that in explicitly so the
 *      function is portable to engines whose stability we have not
 *      independently verified (e.g. ancient embedded V8 forks, or
 *      custom builds).  Cost is one integer compare per tied pair.
 *
 * @module sort
 */

import type { KeyFn, SortDirection } from "./types.js";

/**
 * Internal helper: 3-way compare of two keys with `null` /
 * `undefined` sorting last.
 *
 * Truth table (for `dir = "asc"`):
 *
 * | a           | b           | result            |
 * |-------------|-------------|-------------------|
 * | null        | null        | 0 (tie)           |
 * | null        | anything    | +1 (a after b)    |
 * | anything    | null        | -1 (a before b)   |
 * | x           | y, x < y    | -1                |
 * | x           | y, x > y    | +1                |
 * | x           | y, x === y  | 0                 |
 *
 * For `dir = "desc"` the non-null comparison flips.  Null
 * placement stays the same — undated/missing always last.
 */
function compareKeys<K>(a: K, b: K, dir: SortDirection): number {
  const aMissing = a === null || a === undefined;
  const bMissing = b === null || b === undefined;
  if (aMissing && bMissing) return 0;
  if (aMissing) return 1;
  if (bMissing) return -1;
  // Both present.  `<` / `>` work for strings, numbers, bigints
  // (with same-type comparands).  NaN propagates as "not less, not
  // greater" — both comparisons return false, so result is 0,
  // which combined with the index tiebreaker keeps NaN-keyed items
  // in stable input order.
  if (a < b) return dir === "asc" ? -1 : 1;
  if (a > b) return dir === "asc" ? 1 : -1;
  return 0;
}

/**
 * Stable sort by an extracted key.  Returns a fresh array; input
 * is never mutated.
 *
 * ```ts
 * // Posts newest-first; undated to the end.
 * const sorted = sortBy(posts, (p) => p.pubDate, "desc");
 *
 * // Stable alphabetical (ties by input order).
 * const alpha  = sortBy(authors, (a) => a.name);
 * ```
 *
 * Reproducibility (FM03): same input array produces byte-identical
 * output.  The index tiebreaker is what makes two inputs that
 * differ only in caller-array order *also* produce the same output
 * *when paired with a deterministic key*.  If you want order-
 * independent output, sort by a unique stable key (e.g. an `id`).
 */
export function sortBy<T, K>(
  items: readonly T[],
  keyFn: KeyFn<T, K>,
  dir: SortDirection = "asc",
): T[] {
  // Decorate-sort-undecorate.  Computing keys once is both faster
  // (avoids re-invocation per compare in an N log N loop) and safer
  // (keyFn side-effects, if any, fire exactly once per item).
  const decorated = items.map((item, index) => ({
    item,
    key: keyFn(item),
    index,
  }));
  decorated.sort((a, b) => {
    const c = compareKeys(a.key, b.key, dir);
    if (c !== 0) return c;
    return a.index - b.index;
  });
  return decorated.map((d) => d.item);
}

/**
 * Like `sortBy`, but with two key functions.  The second is used
 * only as a tiebreaker when the first ties.  Equivalent to
 * `sortBy(sortBy(items, secondary), primary)` but does one pass.
 *
 * Common Forme use: `sortBy2(posts, p => p.pubDate, p => p.id, "desc", "asc")`
 * gives reverse-chrono with id as the within-same-date tiebreaker —
 * exactly what `forme-index-renderer` does internally.
 */
export function sortBy2<T, K1, K2>(
  items: readonly T[],
  primary: KeyFn<T, K1>,
  secondary: KeyFn<T, K2>,
  primaryDir: SortDirection = "asc",
  secondaryDir: SortDirection = "asc",
): T[] {
  const decorated = items.map((item, index) => ({
    item,
    k1: primary(item),
    k2: secondary(item),
    index,
  }));
  decorated.sort((a, b) => {
    const c1 = compareKeys(a.k1, b.k1, primaryDir);
    if (c1 !== 0) return c1;
    const c2 = compareKeys(a.k2, b.k2, secondaryDir);
    if (c2 !== 0) return c2;
    return a.index - b.index;
  });
  return decorated.map((d) => d.item);
}
