/**
 * slice.ts — `take`, `drop`, `slice`, `chunk`.
 *
 * Window-into-sequence operations.  All are pure (input not
 * mutated), return fresh arrays, and clamp out-of-range indices
 * to the array bounds rather than throwing — making them safe to
 * chain in a pipeline where the upstream length is unknown.
 *
 * ```
 *   take(items, 3)    — first 3 items
 *   drop(items, 3)    — everything except the first 3
 *   slice(items, 1, 4)— items[1..4)  (start inclusive, end exclusive)
 *   chunk(items, 10)  — split into batches of 10
 * ```
 *
 * @module slice
 */

/**
 * First `n` items.  Negative `n` is clamped to 0 (returns empty
 * array); `n` larger than the input length returns a copy of the
 * input.
 *
 * Why clamp instead of throw?  `take(posts, options.maxRecent)`
 * is the common shape, and `options.maxRecent` is often
 * caller-controlled.  A defensive clamp lets the pipeline stage
 * stay simple — "give me at most N" works whether N is 0, 5, or
 * a million.
 */
export function take<T>(items: readonly T[], n: number): T[] {
  if (!Number.isFinite(n) || n <= 0) return [];
  const end = Math.min(items.length, Math.floor(n));
  const out: T[] = new Array(end);
  for (let i = 0; i < end; i++) out[i] = items[i]!;
  return out;
}

/**
 * Everything after the first `n` items.  Negative `n` is treated
 * as 0 (returns a copy of the input); `n` larger than the input
 * length returns an empty array.
 */
export function drop<T>(items: readonly T[], n: number): T[] {
  if (!Number.isFinite(n) || n <= 0) {
    return items.slice();
  }
  const start = Math.min(items.length, Math.floor(n));
  return items.slice(start);
}

/**
 * `items[start..end)` — start inclusive, end exclusive, both
 * defaulting to the array bounds.  Negative indices are clamped to
 * 0 (no negative-from-end behaviour, unlike `Array.prototype.slice`).
 *
 * Why drop the negative-from-end overload?  `pipe` works best when
 * step parameters are unambiguous — "the 3rd item from the end"
 * needs `items.length - 3` to be computed by the caller, who
 * already knows whether they want a fixed-position window or a
 * tail-relative one.
 */
export function slice<T>(items: readonly T[], start = 0, end: number = items.length): T[] {
  const s = Math.max(0, Math.floor(start));
  const e = Math.min(items.length, Math.max(s, Math.floor(end)));
  return items.slice(s, e);
}

/**
 * Split into consecutive batches of `size`.  The final batch may
 * be smaller if the input length is not a multiple of `size`.
 *
 * Pagination uses this: `chunk(posts, 10)[pageIndex]` gives the
 * items on page N (0-indexed).
 *
 * Throws `RangeError` if `size <= 0` — the operation is undefined
 * for non-positive batch sizes (would loop forever or divide by
 * zero).
 */
export function chunk<T>(items: readonly T[], size: number): T[][] {
  if (!Number.isFinite(size) || size <= 0 || !Number.isInteger(size)) {
    throw new RangeError(`chunk size must be a positive integer (got ${size})`);
  }
  const out: T[][] = [];
  for (let i = 0; i < items.length; i += size) {
    out.push(items.slice(i, i + size));
  }
  return out;
}
