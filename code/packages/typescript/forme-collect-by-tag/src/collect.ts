/**
 * collect.ts — the main `collectByTag` entry point.
 *
 * Algorithm:
 *
 *   1. For each input item:
 *        a. Call `tagsOf(item)`.  Treat `null` / `undefined` /
 *           empty array as "untagged".
 *        b. For each raw tag string: normalise to bucket key.
 *           Skip empty results (a tag of `"@@@"` normalises to
 *           empty — silently drop rather than synthesise a
 *           bucket).
 *        c. Push the item into each (deduped per-item) bucket.
 *        d. If the item was untagged AND `includeUntagged` is
 *           true, push into the synthetic untagged bucket.
 *   2. After all items are bucketed:
 *        a. Sort items within each bucket via `sortBy` (or skip
 *           if no comparator).
 *        b. Compute `tagNames`: alphabetically-sorted array of
 *           all bucket keys (including the untagged bucket if
 *           present).
 *
 * Per-item tag dedup (step c above) handles the
 * `tags: ["TypeScript", "typescript"]` case — both normalise to
 * `"typescript"`, but the item should appear in that bucket only
 * once, not twice.  Done via a per-item `Set<string>`.
 *
 * @module collect
 */

import { normaliseTag } from "./normalise.js";
import type { CollectByTagOptions, CollectByTagResult } from "./types.js";

/**
 * Group `items` by tag.
 *
 * ```ts
 * const result = collectByTag(posts, {
 *   tagsOf: (p) => p.tags,
 *   sortBy: (a, b) => b.pubDate.localeCompare(a.pubDate),  // newest first
 *   includeUntagged: true,
 * });
 *
 * for (const tag of result.tagNames) {
 *   const items = result.byTag.get(tag)!;
 *   // render tag archive page...
 * }
 * ```
 *
 * Reproducibility: same `items` array + same `tagsOf` + same
 * `sortBy` → byte-identical output (Map iteration order
 * preserved by V8 in insertion order, then we expose a sorted
 * `tagNames` alongside).
 *
 * Input `items` array is never mutated.  Output buckets are
 * fresh arrays — caller may mutate them without affecting the
 * collector's internal state (though the types declare them
 * `readonly`).
 */
export function collectByTag<T>(
  items: readonly T[],
  options: CollectByTagOptions<T>,
): CollectByTagResult<T> {
  const includeUntagged = options.includeUntagged === true;
  const untaggedName = options.untaggedBucketName ?? "untagged";

  // Map<bucketKey, items[]> — uses Map (not Object) so attacker
  // tag names like `__proto__` cannot pollute Object.prototype.
  const byTag = new Map<string, T[]>();

  for (let i = 0; i < items.length; i++) {
    const item = items[i]!;
    const rawTags = options.tagsOf(item);
    const tags = rawTags ?? [];

    // Compute the set of normalised bucket keys for THIS item.
    // Using a Set dedups within-item collisions like
    // `["TypeScript", "typescript"]` (both → "typescript").
    const seen = new Set<string>();
    for (let j = 0; j < tags.length; j++) {
      const key = normaliseTag(tags[j]!);
      if (key === "") continue;  // unrenderable tag — drop silently
      seen.add(key);
    }

    if (seen.size === 0) {
      // Untagged: empty input tags OR all tags normalised to empty.
      if (includeUntagged) {
        addToBucket(byTag, untaggedName, item);
      }
      continue;
    }

    for (const key of seen) {
      addToBucket(byTag, key, item);
    }
  }

  // Sort within each bucket if a comparator was supplied.
  if (options.sortBy) {
    const cmp = options.sortBy;
    for (const bucket of byTag.values()) {
      bucket.sort(cmp);
    }
  }

  // Deterministic alphabetical tag-name list (independent of
  // first-seen Map iteration order).  Done as a separate array
  // so callers don't have to re-sort `byTag.keys()` themselves.
  const tagNames = [...byTag.keys()].sort();

  return { byTag, tagNames };
}

/**
 * Internal helper: append `item` to `byTag[key]`, creating the
 * bucket if absent.  Pulled out so the main flow above reads as
 * "for each tag, add to its bucket" without the array-init
 * noise.
 */
function addToBucket<T>(byTag: Map<string, T[]>, key: string, item: T): void {
  const existing = byTag.get(key);
  if (existing) {
    existing.push(item);
  } else {
    byTag.set(key, [item]);
  }
}
