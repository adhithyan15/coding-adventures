/**
 * types.ts — public types for the index renderer.
 *
 * @module types
 */

/** A single item in an index/archive list. */
export interface IndexItem {
  /** Stable identifier — used as the secondary sort key (tiebreaker)
   *  to ensure deterministic ordering when primary sort keys tie. */
  readonly id: string;
  /** Display title. */
  readonly title: string;
  /** Link target.  Absolute http(s) or root-relative `/path`. */
  readonly url: string;
  /** ISO-8601 datetime.  Used by `pubDate-*` sorts and `year`/`month`
   *  grouping.  Items without a pubDate sort to the END of the list
   *  in pubDate-desc, and to the START in pubDate-asc. */
  readonly pubDate?: string;
  /** Optional one-line summary shown under the title when
   *  `options.showSummary === true`. */
  readonly summary?: string;
  /** Category name — used by `groupBy: "category"`.  Items with no
   *  category land in a synthetic `"Uncategorized"` bucket. */
  readonly category?: string;
  /** Tags — currently exposed in the IndexItem type for forward
   *  compatibility, but the v0 renderer doesn't display them. */
  readonly tags?: readonly string[];
}

/** Options controlling render layout. */
export interface IndexOptions {
  /**
   * How to partition items into sections.  Defaults to `"none"`
   * (flat `<ul>` with no headings).
   */
  readonly groupBy?: "none" | "category" | "year" | "month";
  /**
   * Sort order.  Defaults to `"pubDate-desc"` (newest first).
   * Stable: ties broken by `id` ascending.
   */
  readonly sortBy?: "pubDate-desc" | "pubDate-asc" | "title-asc";
  /** Show item.summary under each title.  Default `false`. */
  readonly showSummary?: boolean;
  /** Show item.pubDate inline.  Default `false`. */
  readonly showDate?: boolean;
  /**
   * Format function for dates when `showDate === true`.
   * Default: ISO-8601 string passed through unchanged.
   */
  readonly dateFormat?: (iso: string) => string;
}

/** Result of `groupItems()` — exposed for callers that want the
 *  grouping logic without the HTML rendering. */
export interface ItemGroup {
  readonly heading: string;
  readonly items: readonly IndexItem[];
}
