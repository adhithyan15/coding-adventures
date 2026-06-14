/**
 * group.ts — `partition`, `groupBy`, `unique`.
 *
 * Three operations that bucket items by a derived key.  All return
 * fresh containers; inputs are never mutated.
 *
 * Why `Map` (not plain `Object`) for `groupBy`?
 *
 *   - **Prototype pollution defence.**  A hostile `keyFn` returning
 *     `"__proto__"` would otherwise reach `Object.prototype` and
 *     pollute every object in the running process.  `Map` is
 *     immune — its keys are stored in a separate internal slot
 *     with no `__proto__` lookup.
 *   - **Non-string keys.**  Numeric, bigint, boolean, and object
 *     keys round-trip without `String(...)` coercion.  Useful for
 *     `groupBy(items, i => i.year)` returning numeric-keyed buckets.
 *   - **Insertion-order iteration.**  `Map` iterates in insertion
 *     order, which combined with our input traversal order makes
 *     the output deterministic regardless of key type.
 *
 * @module group
 */

import type { KeyFn, Partition, Predicate } from "./types.js";

/**
 * Split `items` into a `{ yes, no }` pair based on the predicate.
 * Both arrays preserve input order.  Same as
 * `{ yes: filter(items, p), no: filter(items, (x,i) => !p(x,i)) }`
 * but does one pass.
 *
 * ```ts
 * const { yes: published, no: drafts } = partition(posts, (p) => !p.draft);
 * ```
 */
export function partition<T>(items: readonly T[], predicate: Predicate<T>): Partition<T> {
  const yes: T[] = [];
  const no: T[] = [];
  for (let i = 0; i < items.length; i++) {
    if (predicate(items[i]!, i)) yes.push(items[i]!);
    else no.push(items[i]!);
  }
  return { yes, no };
}

/**
 * Bucket items by a derived key.  Returns a `Map<K, T[]>` whose
 * iteration order is the order in which each bucket's first item
 * appeared in the input.  Within each bucket, items preserve input
 * order.
 *
 * ```ts
 * const byCategory = groupBy(posts, (p) => p.category ?? "uncategorised");
 * for (const [cat, posts] of byCategory) { ... }
 * ```
 *
 * Note: returns a `Map`, not an `Object`.  Use `Object.fromEntries`
 * if a plain-object view is needed — but be aware that doing so
 * loses non-string keys and re-opens the `__proto__` injection
 * vector.
 */
export function groupBy<T, K>(items: readonly T[], keyFn: KeyFn<T, K>): Map<K, T[]> {
  const out = new Map<K, T[]>();
  for (let i = 0; i < items.length; i++) {
    const k = keyFn(items[i]!);
    const bucket = out.get(k);
    if (bucket) bucket.push(items[i]!);
    else out.set(k, [items[i]!]);
  }
  return out;
}

/**
 * Remove duplicate items.  First occurrence wins (preserves input
 * order for the surviving items).
 *
 * Without `keyFn`: deduplicates by identity (`===` / `SameValueZero`)
 * via a `Set`.  Works for primitives and references.
 *
 * With `keyFn`: two items are duplicates iff their keys are equal.
 * Useful for "dedupe posts by slug" where two source files
 * accidentally produced the same canonical URL.
 *
 * ```ts
 * const uniqueTags  = unique(["js", "ts", "js"]);              // ["js","ts"]
 * const uniqueSlugs = unique(posts, (p) => p.slug);            // first wins
 * ```
 */
export function unique<T>(items: readonly T[]): T[];
export function unique<T, K>(items: readonly T[], keyFn: KeyFn<T, K>): T[];
export function unique<T, K>(items: readonly T[], keyFn?: KeyFn<T, K>): T[] {
  const out: T[] = [];
  if (keyFn === undefined) {
    const seen = new Set<T>();
    for (let i = 0; i < items.length; i++) {
      const it = items[i]!;
      if (!seen.has(it)) {
        seen.add(it);
        out.push(it);
      }
    }
    return out;
  }
  const seen = new Set<K>();
  for (let i = 0; i < items.length; i++) {
    const it = items[i]!;
    const k = keyFn(it);
    if (!seen.has(k)) {
      seen.add(k);
      out.push(it);
    }
  }
  return out;
}
