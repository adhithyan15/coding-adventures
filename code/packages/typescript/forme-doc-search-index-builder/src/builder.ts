/**
 * builder.ts — main `buildSearchIndex` entry.
 *
 * =============================================================================
 * THE INVERTED INDEX
 * =============================================================================
 *
 * Search engines store an "inverted" index — the inverse of the
 * obvious "document → list of terms" layout:
 *
 *     forward:   "intro.md" → ["welcome", "guide", "setup"]
 *     inverted:  "welcome" → [{pageId: "intro.md", freq: 1}, ...]
 *                "guide"   → [{pageId: "intro.md", freq: 1}, ...]
 *                "setup"   → [{pageId: "intro.md", freq: 1}, ...]
 *
 * The inverted form is what queries actually need.  Looking up
 * pages matching "setup" is O(1) in the index; looking it up by
 * scanning all pages would be O(N) per query.
 *
 * =============================================================================
 * SHARDING
 * =============================================================================
 *
 * For browser-side search, we DON'T want to ship the whole index
 * upfront.  Instead, we shard by token prefix: shard "ge" holds
 * all tokens starting with "ge".  When the user types "getting",
 * the client tokenises it, looks up "ge" in the manifest's
 * `shardKeys`, fetches just that shard, and searches within it.
 *
 * Shard size scales with vocabulary distribution.  For
 * `shardPrefix=2`, English produces ~600 non-empty shards
 * (most 2-letter prefixes occur somewhere); each shard is
 * proportional to the number of tokens starting with that
 * prefix.  Typical docs sites: a few KB to a few tens of KB
 * per shard.
 *
 * =============================================================================
 * MEMORY BOUNDS
 * =============================================================================
 *
 * Adversarial or just very large inputs can produce indexes
 * far larger than the browser will tolerate.  We cap THREE
 * dimensions:
 *
 *   - `maxPages` (default 100k): refuse inputs with too many pages.
 *   - `maxTokensPerPage` (default 10k): drop tokens beyond this
 *     per page — silently, since downstream consumers don't
 *     care.
 *   - `maxPostingsPerToken` (default 10k): drop additional
 *     postings beyond this per token — silently.  This is the
 *     "the" mitigation: extremely common tokens that match
 *     essentially every page would otherwise produce huge
 *     postings lists with low search value.
 *
 * All caps configurable per call.
 *
 * @module builder
 */

import { tokenize } from "@coding-adventures/forme-doc-search-tokenizer";

import type {
  IndexPageInput,
  BuildIndexOptions,
  BuildIndexOutput,
  IndexShard,
  IndexManifest,
  IndexStats,
  Posting,
} from "./types.js";

// ─────────────────────────────────────────────────────────────────────
// Defaults
// ─────────────────────────────────────────────────────────────────────

const DEFAULT_SHARD_PREFIX = 2;
const DEFAULT_FILTER_STOP_WORDS = true;
const DEFAULT_STEM = true;
const DEFAULT_MAX_PAGES = 100_000;
const DEFAULT_MAX_TOKENS_PER_PAGE = 10_000;
const DEFAULT_MAX_POSTINGS_PER_TOKEN = 10_000;

// ─────────────────────────────────────────────────────────────────────
// Internal mutable types
// ─────────────────────────────────────────────────────────────────────

interface MutablePosting {
  pageId: string;
  freq: number;
  titleHit: boolean;
}

// ─────────────────────────────────────────────────────────────────────
// Public entry
// ─────────────────────────────────────────────────────────────────────

/**
 * Build a sharded inverted search index from a list of pages.
 *
 * @param pages - One entry per page.  Page ids must be unique
 *                (duplicates throw `TypeError`).
 * @param options - `BuildIndexOptions` — all fields optional.
 * @returns `{ shards, manifest }`.  Shards are keyed by the
 *          token-prefix `shardKey`; the manifest lists all
 *          pages and shard keys (sorted for stable output).
 * @throws `TypeError` if `pages.length > maxPages` or if two
 *         pages share the same `id`.
 */
export function buildSearchIndex(
  pages: readonly IndexPageInput[],
  options: BuildIndexOptions = {},
): BuildIndexOutput {
  const shardPrefix = options.shardPrefix ?? DEFAULT_SHARD_PREFIX;
  const filterStopWords = options.filterStopWords ?? DEFAULT_FILTER_STOP_WORDS;
  const stem = options.stem ?? DEFAULT_STEM;
  const maxPages = options.maxPages ?? DEFAULT_MAX_PAGES;
  const maxTokensPerPage = options.maxTokensPerPage ?? DEFAULT_MAX_TOKENS_PER_PAGE;
  const maxPostingsPerToken = options.maxPostingsPerToken ?? DEFAULT_MAX_POSTINGS_PER_TOKEN;

  // Input-validation guardrails.  We validate each numeric
  // option independently rather than just relying on the cap
  // comparison, because `pages.length > NaN`, `Set.size >= NaN`,
  // and similar comparisons are ALL `false` — so passing
  // `maxPages: NaN` (or any other cap as NaN) would silently
  // disable the cap and let an adversarial input through.
  // Same for negative / non-finite values.
  if (!Number.isFinite(maxPages) || maxPages < 0) {
    throw new TypeError(
      `forme-doc-search-index-builder: maxPages must be a non-negative finite number (got ${maxPages})`,
    );
  }
  if (!Number.isFinite(maxTokensPerPage) || maxTokensPerPage < 0) {
    throw new TypeError(
      `forme-doc-search-index-builder: maxTokensPerPage must be a non-negative finite number (got ${maxTokensPerPage})`,
    );
  }
  if (!Number.isFinite(maxPostingsPerToken) || maxPostingsPerToken < 0) {
    throw new TypeError(
      `forme-doc-search-index-builder: maxPostingsPerToken must be a non-negative finite number (got ${maxPostingsPerToken})`,
    );
  }
  if (!Number.isInteger(shardPrefix) || shardPrefix < 1) {
    throw new TypeError(
      `forme-doc-search-index-builder: shardPrefix must be an integer >= 1 (got ${shardPrefix})`,
    );
  }
  if (pages.length > maxPages) {
    throw new TypeError(
      `forme-doc-search-index-builder: ${pages.length} pages exceeds maxPages cap (${maxPages})`,
    );
  }

  // Phase 1: tokenise + build a global inverted index.
  const seenPageIds = new Set<string>();
  // token → (pageId → mutable posting)
  const invIndex = new Map<string, Map<string, MutablePosting>>();
  let totalTokens = 0;

  for (const page of pages) {
    if (seenPageIds.has(page.id)) {
      throw new TypeError(
        `forme-doc-search-index-builder: duplicate page id ${JSON.stringify(page.id)}`,
      );
    }
    seenPageIds.add(page.id);

    // Tokenise body + title separately so we can record `titleHit`.
    const bodyTokens = tokenize(page.body, {
      filterStopWords,
      stem,
      customStopWords: options.customStopWords,
    });
    const titleTokens = page.title !== undefined
      ? tokenize(page.title, {
          filterStopWords,
          stem,
          customStopWords: options.customStopWords,
        })
      : [];

    // Cap per-page unique tokens to bound memory.
    const pageUnique = new Set<string>();
    const titleSet = new Set<string>(titleTokens);

    for (const tok of bodyTokens) {
      if (pageUnique.size >= maxTokensPerPage && !pageUnique.has(tok)) continue;
      pageUnique.add(tok);
      // Only count toward `totalTokens` if the posting was
      // actually accepted (i.e. NOT dropped by the per-token
      // cap).  Otherwise `stats.totalTokens` would over-count.
      if (addPosting(invIndex, tok, page.id, titleSet.has(tok), maxPostingsPerToken)) {
        totalTokens++;
      }
    }
    for (const tok of titleTokens) {
      if (pageUnique.size >= maxTokensPerPage && !pageUnique.has(tok)) continue;
      pageUnique.add(tok);
      if (addPosting(invIndex, tok, page.id, true, maxPostingsPerToken)) {
        totalTokens++;
      }
    }
  }

  // Phase 2: shard the inverted index by token prefix.
  const shards = new Map<string, Map<string, Posting[]>>();
  for (const [token, pageMap] of invIndex) {
    const key = shardKeyFor(token, shardPrefix);
    let shard = shards.get(key);
    if (shard === undefined) {
      shard = new Map<string, Posting[]>();
      shards.set(key, shard);
    }
    // Materialise postings as a sorted array (descending freq,
    // then ascending pageId for stable ordering).
    const postings = Array.from(pageMap.values()).map(toReadonlyPosting);
    postings.sort(compareByFreqDesc);
    shard.set(token, postings);
  }

  // Phase 3: emit shards + manifest.
  const sortedShardKeys = Array.from(shards.keys()).sort();
  const outputShards = new Map<string, IndexShard>();
  for (const key of sortedShardKeys) {
    outputShards.set(key, {
      shardKey: key,
      postings: shards.get(key)!,
    });
  }

  const stats: IndexStats = {
    totalTokens,
    uniqueTokens: invIndex.size,
    pageCount: pages.length,
    shardCount: outputShards.size,
  };

  const manifest: IndexManifest = {
    pages: Array.from(seenPageIds).sort(),
    shardKeys: sortedShardKeys,
    shardPrefix,
    filterStopWords,
    stem,
    stats,
  };

  return { shards: outputShards, manifest };
}

// ─────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────

/**
 * Add/increment a posting for `token` in `pageId`.  Increments
 * `freq` if a posting for this page already exists.  Respects
 * the per-token postings cap.
 *
 * @returns `true` if the posting was accepted (created or
 *          incremented); `false` if it was silently dropped
 *          because the per-token cap was reached for a new
 *          pageId.  Callers use this to keep their token
 *          counts accurate.
 */
function addPosting(
  invIndex: Map<string, Map<string, MutablePosting>>,
  token: string,
  pageId: string,
  titleHit: boolean,
  maxPostingsPerToken: number,
): boolean {
  let pageMap = invIndex.get(token);
  if (pageMap === undefined) {
    pageMap = new Map();
    invIndex.set(token, pageMap);
  }
  const existing = pageMap.get(pageId);
  if (existing !== undefined) {
    existing.freq++;
    if (titleHit) existing.titleHit = true;
    return true;
  }
  // New posting — check cap.
  if (pageMap.size >= maxPostingsPerToken) {
    // Silently drop (the "the" mitigation).
    return false;
  }
  pageMap.set(pageId, { pageId, freq: 1, titleHit });
  return true;
}

/**
 * Compute the shard key for a token: the first `shardPrefix`
 * characters of the token, or the whole token if it's shorter
 * than `shardPrefix`.
 */
function shardKeyFor(token: string, shardPrefix: number): string {
  // String.prototype.slice safely truncates if the token is
  // shorter than `shardPrefix`, returning the whole token.
  return token.slice(0, shardPrefix);
}

function toReadonlyPosting(p: MutablePosting): Posting {
  return { pageId: p.pageId, freq: p.freq, titleHit: p.titleHit };
}

function compareByFreqDesc(a: Posting, b: Posting): number {
  if (a.freq !== b.freq) return b.freq - a.freq;
  // Stable tiebreak: ascending pageId.
  if (a.pageId < b.pageId) return -1;
  if (a.pageId > b.pageId) return 1;
  /* v8 ignore start */
  // Unreachable: postings within one shard are keyed by pageId
  // (one posting per page per token), and duplicate pageIds
  // throw at insert time.  Kept as defensive total-ordering.
  return 0;
  /* v8 ignore stop */
}
