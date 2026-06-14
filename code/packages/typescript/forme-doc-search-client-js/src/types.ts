/**
 * types.ts — public signatures for the browser search client.
 *
 * @module types
 */

import type {
  IndexShard,
  IndexManifest,
} from "@coding-adventures/forme-doc-search-index-builder";

/**
 * Async callback the caller provides to fetch one shard.  In
 * the browser this typically wraps `fetch()`; on Node (for
 * tests) it can wrap `fs.readFile` or just look up an
 * in-memory map.  The `net:fetch` / `fs:read` capability lives
 * with the CALLER — this package itself stays `[]`.
 *
 * @param shardKey - The shard key from the manifest.
 * @returns The IndexShard for that key.  Reject with an Error
 *          if the shard can't be loaded; the search client
 *          catches and continues with whatever it has.
 */
export type ShardFetcher = (shardKey: string) => Promise<IndexShard>;

/**
 * Options for constructing a `SearchClient`.
 */
export interface SearchClientOptions {
  /** The bootstrap manifest from `forme-doc-search-index-builder`. */
  readonly manifest: IndexManifest;
  /** Async callback to fetch one shard.  See `ShardFetcher`. */
  readonly fetchShard: ShardFetcher;
  /**
   * LRU cache size for loaded shards.  Default: `50` (most
   * sites have far fewer shards than this; sites with more get
   * least-recently-used eviction when the cap is hit).
   */
  readonly maxCachedShards?: number;
  /**
   * Score multiplier applied to postings with `titleHit: true`.
   * Default: `2.0` — title matches double a query's contribution
   * to the page's score.  Set to `1.0` to disable title
   * boosting; set higher for stronger title-bias rankings.
   */
  readonly titleBoost?: number;
}

/**
 * Options for a single `.search()` call.
 */
export interface SearchQueryOptions {
  /**
   * Maximum number of results to return.  Default: `20`.
   * Results are sorted by descending score; the top `limit`
   * are returned.
   */
  readonly limit?: number;
}

/**
 * One search result.
 */
export interface SearchResult {
  /** The page id (from the index manifest's `pages` list). */
  readonly pageId: string;
  /** Aggregate score across all matched query tokens. */
  readonly score: number;
  /**
   * Which of the user's (normalised) query tokens matched
   * this page.  Useful for highlighting in the UI.
   */
  readonly matchedTokens: readonly string[];
}
