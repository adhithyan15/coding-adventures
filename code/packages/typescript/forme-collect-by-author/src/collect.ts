/**
 * collect.ts — the main `collectByAuthor` entry point.
 *
 * Mirrors `collectByTag` from `forme-collect-by-tag` but
 * accepts an `authorOf` accessor that may return either a
 * single string OR a string array (co-authors).  The string
 * case is treated as a single-element array internally.
 *
 * Algorithm:
 *
 *   1. For each input item:
 *        a. Call `authorOf(item)`.
 *        b. Normalise to an array of raw author strings.
 *           - `null` / `undefined` → `[]`
 *           - `"name"` → `["name"]`
 *           - `["name1", "name2"]` → as-is
 *        c. For each raw name: normalise to bucket key.
 *           Skip empty results.
 *        d. Push the item into each (deduped per-item) bucket.
 *        e. If the item is anonymous AND `includeAnonymous` is
 *           true, push into the synthetic anonymous bucket.
 *   2. After all items are bucketed:
 *        a. Sort items within each bucket via `sortBy` if
 *           supplied.
 *        b. Compute `authorNames`: alphabetically-sorted array
 *           of all bucket keys.
 *
 * Per-item dedup is critical for co-authors who appear under
 * multiple spellings: `["Ada Lovelace", "ada lovelace"]` both
 * normalise to `"ada-lovelace"`, but the item should appear in
 * that bucket only once.
 *
 * @module collect
 */

import { normaliseAuthor } from "./normalise.js";
import type { CollectByAuthorOptions, CollectByAuthorResult } from "./types.js";

/**
 * Group `items` by author.
 *
 * ```ts
 * const result = collectByAuthor(posts, {
 *   authorOf: (p) => p.author ?? p.authors,
 *   sortBy: (a, b) => b.pubDate.localeCompare(a.pubDate),
 *   includeAnonymous: true,
 * });
 *
 * for (const author of result.authorNames) {
 *   const items = result.byAuthor.get(author)!;
 *   // render author archive page...
 * }
 * ```
 *
 * Reproducibility: same `items` + same `authorOf` + same
 * `sortBy` → byte-identical output.
 *
 * Input `items` array is never mutated.  Output buckets are
 * fresh arrays.
 */
export function collectByAuthor<T>(
  items: readonly T[],
  options: CollectByAuthorOptions<T>,
): CollectByAuthorResult<T> {
  const includeAnonymous = options.includeAnonymous === true;
  const anonymousName = options.anonymousBucketName ?? "anonymous";

  // Map<bucketKey, items[]> — uses Map (not Object) so attacker
  // author names like `__proto__` cannot pollute Object.prototype.
  const byAuthor = new Map<string, T[]>();

  for (let i = 0; i < items.length; i++) {
    const item = items[i]!;
    const raw = options.authorOf(item);
    const rawList = normaliseToList(raw);

    // Compute the set of normalised bucket keys for THIS item.
    // Set deduplicates within-item collisions like
    // `["Ada", "ada"]` (both → "ada").
    const seen = new Set<string>();
    for (let j = 0; j < rawList.length; j++) {
      const key = normaliseAuthor(rawList[j]!);
      if (key === "") continue;  // unrenderable author — drop silently
      seen.add(key);
    }

    if (seen.size === 0) {
      // Anonymous: null / undefined / [] / "" / all-stripped.
      if (includeAnonymous) {
        addToBucket(byAuthor, anonymousName, item);
      }
      continue;
    }

    for (const key of seen) {
      addToBucket(byAuthor, key, item);
    }
  }

  // Sort within each bucket if a comparator was supplied.
  if (options.sortBy) {
    const cmp = options.sortBy;
    for (const bucket of byAuthor.values()) {
      bucket.sort(cmp);
    }
  }

  // Deterministic alphabetical author-name list.
  const authorNames = [...byAuthor.keys()].sort();

  return { byAuthor, authorNames };
}

/**
 * Internal: coerce the accessor's return value to a uniform
 * `readonly string[]`.  Single string becomes a one-element
 * array; null/undefined become an empty array.  Non-string
 * array elements are dropped defensively (someone passing
 * `[null, "Ada"]` shouldn't crash the collector).
 */
function normaliseToList(value: string | readonly string[] | null | undefined): readonly string[] {
  if (value === null || value === undefined) return [];
  if (typeof value === "string") return value === "" ? [] : [value];
  // Array case — filter non-strings defensively.
  const out: string[] = [];
  for (let i = 0; i < value.length; i++) {
    const v = value[i];
    if (typeof v === "string" && v !== "") out.push(v);
  }
  return out;
}

/**
 * Internal helper: append `item` to `byAuthor[key]`, creating
 * the bucket if absent.
 */
function addToBucket<T>(byAuthor: Map<string, T[]>, key: string, item: T): void {
  const existing = byAuthor.get(key);
  if (existing) {
    existing.push(item);
  } else {
    byAuthor.set(key, [item]);
  }
}
