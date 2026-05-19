/**
 * filter-map.ts — the workhorse pair, plus flatMap.
 *
 * All three functions:
 *
 *   - accept `readonly T[]` (do not mutate the input)
 *   - return a fresh `T[]` (so callers can mutate safely)
 *   - pass the index as the second arg to the callback (consistent
 *     with `Array.prototype.filter` / `.map`)
 *
 * Why re-export the obvious operations?  Three reasons:
 *
 *   1. **Uniform `readonly T[]` discipline.**  `Array.prototype.filter`
 *      accepts `readonly` but the callback can technically still mutate
 *      the input by reference.  Wrapping enforces the no-mutation
 *      invariant that every other helper in this package shares.
 *   2. **Pipe-shaped signature.**  Our wrappers can be partially
 *      applied (`(items) => map(items, fn)`) and dropped into `pipe`
 *      without the awkward `Array.prototype` dance.
 *   3. **Documentation surface.**  When a junior reader sees
 *      `filter(posts, isPublished)` in a Forme stage they know
 *      exactly which package the helper comes from and what its
 *      reproducibility guarantees are.
 *
 * @module filter-map
 */

import type { FlatMapper, Mapper, Predicate } from "./types.js";

/**
 * Return a new array of items for which `predicate` returned true.
 * Input order preserved.  Input not mutated.
 *
 * ```ts
 * const published = filter(posts, (p) => !p.draft);
 * ```
 */
export function filter<T>(items: readonly T[], predicate: Predicate<T>): T[] {
  const out: T[] = [];
  for (let i = 0; i < items.length; i++) {
    if (predicate(items[i]!, i)) out.push(items[i]!);
  }
  return out;
}

/**
 * Return a new array where each item has been replaced by
 * `mapper(item, index)`.  Input order preserved.  Input not mutated.
 *
 * ```ts
 * const slugs = map(posts, (p) => p.slug);
 * ```
 */
export function map<T, U>(items: readonly T[], mapper: Mapper<T, U>): U[] {
  const out: U[] = new Array(items.length);
  for (let i = 0; i < items.length; i++) {
    out[i] = mapper(items[i]!, i);
  }
  return out;
}

/**
 * Like `map`, but the mapper returns an array per item and the
 * result is flattened by one level.  Input order preserved.  Input
 * not mutated.
 *
 * ```ts
 * const allTags = flatMap(posts, (p) => p.tags);
 * ```
 *
 * Note: this is a single-level flatten (like `Array.prototype.flatMap`).
 * For deeper flattening, compose `flatMap` repeatedly via `pipe`.
 */
export function flatMap<T, U>(items: readonly T[], mapper: FlatMapper<T, U>): U[] {
  const out: U[] = [];
  for (let i = 0; i < items.length; i++) {
    const chunk = mapper(items[i]!, i);
    for (let j = 0; j < chunk.length; j++) out.push(chunk[j]!);
  }
  return out;
}
