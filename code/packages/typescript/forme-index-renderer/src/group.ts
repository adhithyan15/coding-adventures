/**
 * group.ts — partition items into headed sections.
 *
 * Group headings:
 *   "none"     → single group with empty heading (caller renders flat)
 *   "category" → item.category (missing → "Uncategorized")
 *   "year"     → 4-digit year from pubDate (missing pubDate → "Undated")
 *   "month"    → "YYYY-MM" from pubDate (missing pubDate → "Undated")
 *
 * Group order:
 *   "category" — alphabetical (uncategorised sorts last)
 *   "year" / "month" — reverse chronological (newest first; Undated last)
 *
 * Within each group, items keep the order the caller passed in (so
 * the caller's sortItems() result is preserved).
 *
 * @module group
 */

import type { IndexItem, IndexOptions, ItemGroup } from "./types.js";

const UNCATEGORISED = "Uncategorized";
const UNDATED       = "Undated";

/** Partition items into groups per `groupBy`.  Items keep relative
 *  order within each group; group order is deterministic per the
 *  rules above. */
export function groupItems(
  items: readonly IndexItem[],
  groupBy: NonNullable<IndexOptions["groupBy"]>,
): readonly ItemGroup[] {
  if (groupBy === "none") {
    return [{ heading: "", items }];
  }

  const buckets = new Map<string, IndexItem[]>();
  for (const item of items) {
    const key = bucketKey(item, groupBy);
    let arr = buckets.get(key);
    if (!arr) {
      arr = [];
      buckets.set(key, arr);
    }
    arr.push(item);
  }

  const keys = [...buckets.keys()];
  keys.sort(groupSort(groupBy));
  return keys.map((heading) => ({ heading, items: buckets.get(heading)! }));
}

function bucketKey(item: IndexItem, groupBy: "category" | "year" | "month"): string {
  if (groupBy === "category") {
    const c = item.category;
    return (typeof c === "string" && c.length > 0) ? c : UNCATEGORISED;
  }
  // year / month
  const iso = item.pubDate;
  if (iso === undefined) return UNDATED;
  const t = Date.parse(iso);
  if (!Number.isFinite(t)) return UNDATED;
  const d = new Date(t);
  const year = d.getUTCFullYear();
  if (groupBy === "year") return String(year);
  // groupBy === "month"
  const month = String(d.getUTCMonth() + 1).padStart(2, "0");
  return `${year}-${month}`;
}

function groupSort(groupBy: "category" | "year" | "month"): (a: string, b: string) => number {
  if (groupBy === "category") {
    // Alphabetical; UNCATEGORISED last regardless of letter.
    return (a, b) => {
      if (a === UNCATEGORISED && b === UNCATEGORISED) return 0;
      if (a === UNCATEGORISED) return 1;
      if (b === UNCATEGORISED) return -1;
      return a < b ? -1 : a > b ? 1 : 0;
    };
  }
  // year / month: reverse chronological; UNDATED last.
  return (a, b) => {
    if (a === UNDATED && b === UNDATED) return 0;
    if (a === UNDATED) return 1;
    if (b === UNDATED) return -1;
    return a < b ? 1 : a > b ? -1 : 0;
  };
}
