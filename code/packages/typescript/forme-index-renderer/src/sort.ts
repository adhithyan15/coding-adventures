/**
 * sort.ts — deterministic comparators for IndexItem arrays.
 *
 * All comparators break ties with `id` ascending so reordered-but-
 * equivalent inputs produce byte-identical output (FM03
 * reproducibility).
 *
 * @module sort
 */

import type { IndexItem, IndexOptions } from "./types.js";

/** Return a sorted COPY of items per `sortBy`. */
export function sortItems(
  items: readonly IndexItem[],
  sortBy: NonNullable<IndexOptions["sortBy"]>,
): readonly IndexItem[] {
  const copy = [...items];
  switch (sortBy) {
    case "pubDate-desc":
      copy.sort(combine(byPubDateDesc, byIdAsc));
      break;
    case "pubDate-asc":
      copy.sort(combine(byPubDateAsc, byIdAsc));
      break;
    case "title-asc":
      copy.sort(combine(byTitleAsc, byIdAsc));
      break;
  }
  return copy;
}

// ─── Comparators ─────────────────────────────────────────────────────────

type Cmp = (a: IndexItem, b: IndexItem) => number;

function combine(primary: Cmp, secondary: Cmp): Cmp {
  return (a, b) => {
    const r = primary(a, b);
    return r === 0 ? secondary(a, b) : r;
  };
}

function byIdAsc(a: IndexItem, b: IndexItem): number {
  return a.id < b.id ? -1 : a.id > b.id ? 1 : 0;
}

/**
 * pubDate-desc: newer first.  Items without pubDate sort to the END.
 * Items with malformed pubDate (Date.parse → NaN) also sort to the END.
 */
function byPubDateDesc(a: IndexItem, b: IndexItem): number {
  const ta = parseDate(a.pubDate);
  const tb = parseDate(b.pubDate);
  if (ta === null && tb === null) return 0;
  if (ta === null) return 1;   // a goes after b
  if (tb === null) return -1;
  return tb - ta;              // descending
}

function byPubDateAsc(a: IndexItem, b: IndexItem): number {
  const ta = parseDate(a.pubDate);
  const tb = parseDate(b.pubDate);
  if (ta === null && tb === null) return 0;
  if (ta === null) return 1;   // a goes after b
  if (tb === null) return -1;
  return ta - tb;
}

function byTitleAsc(a: IndexItem, b: IndexItem): number {
  return a.title < b.title ? -1 : a.title > b.title ? 1 : 0;
}

function parseDate(iso: string | undefined): number | null {
  if (iso === undefined) return null;
  const t = Date.parse(iso);
  return Number.isFinite(t) ? t : null;
}
