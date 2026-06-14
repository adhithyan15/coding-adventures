/**
 * client.ts — the `SearchClient` class.
 *
 * =============================================================================
 * THE LIFECYCLE
 * =============================================================================
 *
 *   construct  →  manifest in memory, no shards loaded yet
 *   .search(q) →  tokenise q via search-tokenizer (using flags
 *                 baked into the manifest);
 *                 for each unique token:
 *                   - derive shardKey via token.slice(0, manifest.shardPrefix)
 *                   - if shard cached, look it up;
 *                     else if shard key is in manifest.shardKeys,
 *                       call fetchShard(shardKey), cache it (LRU);
 *                     else (shard key not in manifest) skip;
 *                   - look up postings for the token in the shard;
 *                   - accumulate score per pageId
 *                     (score += freq * (titleHit ? titleBoost : 1));
 *                 sort pageIds by descending score;
 *                 return top `limit`.
 *
 * =============================================================================
 * CAPABILITY MODEL
 * =============================================================================
 *
 * Critical: this package has capabilities `[]`.  The
 * shard-fetcher is INJECTED — the caller provides the actual
 * `fetch()` (browser) or `fs.readFile` (Node test) wrapper.
 * The SearchClient itself doesn't instantiate any I/O primitive.
 *
 * That keeps this library testable (mock fetcher in tests) AND
 * means it can be reused in non-browser contexts (a CLI search
 * tool, a Node server-side search endpoint, etc.) without
 * forking.
 *
 * =============================================================================
 * SHARD-FETCH FAILURES
 * =============================================================================
 *
 * If a `fetchShard` call rejects (network error, 404, bad JSON,
 * etc.), the client logs nothing (no logger dependency) and
 * SKIPS that shard.  Search continues with whatever other shards
 * loaded successfully.  This is the "degrade gracefully"
 * principle — a single broken shard shouldn't blank out an
 * entire search session.
 *
 * Callers who want failure visibility can wrap their own
 * `fetchShard` to log/report errors before re-throwing.
 *
 * @module client
 */

import { tokenize } from "@coding-adventures/forme-doc-search-tokenizer";
import type {
  IndexShard,
  Posting,
} from "@coding-adventures/forme-doc-search-index-builder";

import type {
  ShardFetcher,
  SearchClientOptions,
  SearchQueryOptions,
  SearchResult,
} from "./types.js";

// ─────────────────────────────────────────────────────────────────────
// Defaults
// ─────────────────────────────────────────────────────────────────────

const DEFAULT_MAX_CACHED_SHARDS = 50;
const DEFAULT_TITLE_BOOST = 2.0;
const DEFAULT_QUERY_LIMIT = 20;

// ─────────────────────────────────────────────────────────────────────
// SearchClient class
// ─────────────────────────────────────────────────────────────────────

export class SearchClient {
  private readonly manifest: SearchClientOptions["manifest"];
  private readonly fetchShard: ShardFetcher;
  private readonly maxCachedShards: number;
  private readonly titleBoost: number;
  private readonly shardKeys: Set<string>;

  /**
   * LRU shard cache.  `Map` insertion-order doubles as the LRU
   * recency order: every `.get()` deletes-and-reinserts so the
   * most-recently-used shard becomes the newest in the iterator.
   * Eviction removes the oldest (first) entry.
   */
  private readonly cache: Map<string, IndexShard>;

  /**
   * In-flight shard fetches.  Multiple concurrent `.search()`
   * calls for the same shard share one Promise so we don't
   * fire duplicate network requests.
   */
  private readonly inflight: Map<string, Promise<IndexShard | null>>;

  constructor(options: SearchClientOptions) {
    this.manifest = options.manifest;
    this.fetchShard = options.fetchShard;
    this.maxCachedShards = options.maxCachedShards ?? DEFAULT_MAX_CACHED_SHARDS;
    this.titleBoost = options.titleBoost ?? DEFAULT_TITLE_BOOST;
    // Validate numeric options at construction so misconfigurations
    // surface immediately (not on the first search call).
    if (!Number.isFinite(this.maxCachedShards) || this.maxCachedShards < 1) {
      throw new TypeError(
        `forme-doc-search-client-js: maxCachedShards must be a finite number >= 1 (got ${this.maxCachedShards})`,
      );
    }
    if (!Number.isFinite(this.titleBoost) || this.titleBoost < 0) {
      throw new TypeError(
        `forme-doc-search-client-js: titleBoost must be a finite non-negative number (got ${this.titleBoost})`,
      );
    }
    this.shardKeys = new Set(this.manifest.shardKeys);
    this.cache = new Map();
    this.inflight = new Map();
  }

  /**
   * Run a search.  Tokenises the query using the same flags
   * the index was built with, fetches the relevant shards (via
   * the injected `fetchShard`), merges postings, and returns
   * the top results sorted by descending score.
   *
   * @param query - The user's query string.
   * @param options - `{ limit? }`.
   * @returns Ranked search results.  An empty array if the
   *          query produced no recognised tokens (e.g. all
   *          stop-words) or no matched postings.
   */
  async search(query: string, options: SearchQueryOptions = {}): Promise<SearchResult[]> {
    const limit = options.limit ?? DEFAULT_QUERY_LIMIT;
    if (!Number.isFinite(limit) || limit < 0) {
      throw new TypeError(
        `forme-doc-search-client-js: limit must be a finite non-negative number (got ${limit})`,
      );
    }

    // Tokenise the query using the manifest's flags so client
    // and index agree.
    const tokens = tokenize(query, {
      filterStopWords: this.manifest.filterStopWords,
      stem: this.manifest.stem,
    });
    if (tokens.length === 0) return [];

    // Deduplicate query tokens — repeating "install install" in
    // the query shouldn't double-count.
    const uniqueTokens = Array.from(new Set(tokens));

    // Score accumulator: pageId → { score, matchedTokens }.
    const scores = new Map<string, { score: number; matchedTokens: Set<string> }>();

    // Fetch all relevant shards in parallel.  Distinct shards
    // are fetched concurrently; the same shard requested
    // multiple times shares an in-flight Promise.
    const shardKeysNeeded = new Set<string>();
    for (const tok of uniqueTokens) {
      const key = this.shardKeyFor(tok);
      if (this.shardKeys.has(key)) {
        shardKeysNeeded.add(key);
      }
    }
    const shardEntries = await Promise.all(
      Array.from(shardKeysNeeded).map(
        async (key) => [key, await this.loadShard(key)] as const,
      ),
    );
    const shardsByKey = new Map<string, IndexShard>();
    for (const [key, shard] of shardEntries) {
      if (shard !== null) shardsByKey.set(key, shard);
    }

    // For each query token, look up its postings and accumulate
    // scores.
    for (const tok of uniqueTokens) {
      const key = this.shardKeyFor(tok);
      const shard = shardsByKey.get(key);
      if (shard === undefined) continue; // shard missing or fetch failed
      const postings = shard.postings.get(tok);
      if (postings === undefined) continue; // token not in index
      for (const p of postings) {
        const contribution = this.scoreFor(p);
        const entry = scores.get(p.pageId);
        if (entry === undefined) {
          scores.set(p.pageId, {
            score: contribution,
            matchedTokens: new Set([tok]),
          });
        } else {
          entry.score += contribution;
          entry.matchedTokens.add(tok);
        }
      }
    }

    // Materialise + sort.
    const results: SearchResult[] = [];
    for (const [pageId, { score, matchedTokens }] of scores) {
      results.push({
        pageId,
        score,
        matchedTokens: Array.from(matchedTokens).sort(),
      });
    }
    results.sort(compareResults);
    return results.slice(0, limit);
  }

  /**
   * Pre-warm the cache with a set of shards.  Useful for
   * predictable load — e.g. fetching common shards eagerly
   * after page load so the first user query feels instant.
   *
   * Shards not in the manifest are silently skipped.
   * Failures from `fetchShard` are silently swallowed.
   */
  async prefetchShards(shardKeys: readonly string[]): Promise<void> {
    await Promise.all(shardKeys.map((key) => this.loadShard(key)));
  }

  /**
   * Empty the shard cache.  Useful if the caller wants to
   * force a fresh fetch on the next query (e.g. after a
   * background index rebuild).
   */
  clearCache(): void {
    this.cache.clear();
  }

  /** Current number of cached shards. */
  get cacheSize(): number {
    return this.cache.size;
  }

  // ───────────────────────────────────────────────────────────
  // Internal helpers
  // ───────────────────────────────────────────────────────────

  /**
   * Compute the shard key for a token, matching the
   * index-builder's algorithm exactly.
   */
  private shardKeyFor(token: string): string {
    return token.slice(0, this.manifest.shardPrefix);
  }

  /**
   * Fetch a shard, using the LRU cache + in-flight dedup.
   * Returns `null` on shard-key-not-in-manifest or fetch
   * failure (caller skips the shard).
   */
  private async loadShard(shardKey: string): Promise<IndexShard | null> {
    if (!this.shardKeys.has(shardKey)) return null;
    // Cache hit?  Promote to most-recently-used.
    const cached = this.cache.get(shardKey);
    if (cached !== undefined) {
      this.cache.delete(shardKey);
      this.cache.set(shardKey, cached);
      return cached;
    }
    // Already fetching?  Share the promise.
    const existing = this.inflight.get(shardKey);
    if (existing !== undefined) return existing;
    // New fetch.
    const promise = this.fetchAndCache(shardKey);
    this.inflight.set(shardKey, promise);
    try {
      return await promise;
    } finally {
      this.inflight.delete(shardKey);
    }
  }

  private async fetchAndCache(shardKey: string): Promise<IndexShard | null> {
    try {
      const shard = await this.fetchShard(shardKey);
      // Defensive: the fetcher could return a malformed shape.
      // We trust its type at the boundary but verify the
      // structural properties we need.
      if (!isLikelyShard(shard)) return null;
      // Evict LRU if at capacity.
      if (this.cache.size >= this.maxCachedShards) {
        const oldest = this.cache.keys().next().value;
        if (oldest !== undefined) {
          this.cache.delete(oldest);
        }
      }
      this.cache.set(shardKey, shard);
      return shard;
    } catch {
      // Degrade gracefully: skip this shard, let search continue.
      return null;
    }
  }

  private scoreFor(p: Posting): number {
    return p.titleHit ? p.freq * this.titleBoost : p.freq;
  }
}

/**
 * Compare two `SearchResult`s for descending-score, then
 * ascending-pageId stable ordering.
 */
function compareResults(a: SearchResult, b: SearchResult): number {
  if (a.score !== b.score) return b.score - a.score;
  if (a.pageId < b.pageId) return -1;
  if (a.pageId > b.pageId) return 1;
  /* v8 ignore next 2 */
  // Unreachable: pageIds are unique within `scores` (Map keyed by pageId).
  return 0;
}

/**
 * Defensive shape check on a fetched shard.  Lets us skip
 * malformed responses without crashing the client.
 *
 * @internal
 */
function isLikelyShard(s: unknown): s is IndexShard {
  if (s === null || typeof s !== "object") return false;
  const obj = s as { shardKey?: unknown; postings?: unknown };
  if (typeof obj.shardKey !== "string") return false;
  if (!(obj.postings instanceof Map)) return false;
  return true;
}
