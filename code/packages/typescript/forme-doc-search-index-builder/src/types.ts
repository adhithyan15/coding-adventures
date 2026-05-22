/**
 * types.ts — public signatures for the search-index builder.
 *
 * @module types
 */

/**
 * One input page — what the caller hands in for each `.md`
 * page in the site.
 */
export interface IndexPageInput {
  /**
   * Unique page identifier — typically the URL path or
   * relative file path.  Used as the key in postings and the
   * manifest's `pages` list.  Caller's responsibility to keep
   * unique; duplicates throw `TypeError`.
   */
  readonly id: string;

  /**
   * The page's body content as PLAIN TEXT (markdown stripped,
   * HTML stripped).  Will be tokenised via
   * `@coding-adventures/forme-doc-search-tokenizer`.  Empty
   * string is fine — produces no postings for this page (but
   * the page still appears in the manifest).
   */
  readonly body: string;

  /**
   * Optional page title.  When present, the title text is
   * tokenised separately and its tokens are recorded with a
   * `titleHit: true` flag in postings — query-time ranking
   * can boost hits in titles.
   */
  readonly title?: string;
}

/**
 * Options for `buildSearchIndex`.
 */
export interface BuildIndexOptions {
  /**
   * Number of leading characters of each token used as the
   * shard key.  Smaller values produce fewer / larger shards
   * (better cold-start cache hits, worse incremental loading);
   * larger values produce many smaller shards.  Default: `2`.
   *
   * For sites < 1k pages, `1` (26 shards if alphabet-dominant)
   * is plenty.  For 10k+ pages, `2` or `3` distributes load
   * better.  Practical range: `1..4`.
   */
  readonly shardPrefix?: number;

  /**
   * Forwarded to the tokeniser.  Default: `true` (drops common
   * stop-words to shrink the index).  Query tokenisation MUST
   * use the same flag.
   */
  readonly filterStopWords?: boolean;

  /**
   * Forwarded to the tokeniser.  Default: `true` (Porter
   * stemming collapses morphological variants).  Query
   * tokenisation MUST use the same flag.
   */
  readonly stem?: boolean;

  /**
   * Override the tokeniser's stop-word set.  Only consulted
   * when `filterStopWords` is `true`.
   */
  readonly customStopWords?: ReadonlySet<string>;

  /**
   * Maximum number of pages the index will accept.  Inputs
   * beyond this throw `TypeError`.  Default: `100_000`.
   *
   * Why a cap: the index data structure scales as
   * O(uniqueTokens × averagePostingsPerToken), and adversarial
   * (or just very large) inputs can produce indexes far larger
   * than the browser will tolerate.  Failing fast at build
   * time is preferable to shipping a too-big index.
   */
  readonly maxPages?: number;

  /**
   * Maximum unique tokens accepted from any single page.
   * Tokens beyond this are silently dropped.  Default:
   * `10_000`.  Real-world doc pages essentially never exceed
   * ~1000 unique tokens; 10k leaves a wide margin.
   */
  readonly maxTokensPerPage?: number;

  /**
   * Maximum postings entries kept for any single token.  When
   * a token would exceed this, additional pages are silently
   * dropped from its postings list (the page itself is still
   * in the manifest).  Default: `10_000`.  Mitigates the
   * "the" problem — extremely common tokens that match
   * essentially every page would otherwise produce huge
   * postings lists with low search value.
   */
  readonly maxPostingsPerToken?: number;
}

// ─────────────────────────────────────────────────────────────────────
// Output shapes
// ─────────────────────────────────────────────────────────────────────

/** One posting — a record that a token appears in a page. */
export interface Posting {
  /** The page's id (from `IndexPageInput.id`). */
  readonly pageId: string;
  /** Number of occurrences of the token in the page body + title. */
  readonly freq: number;
  /** True if the token appeared in the page's title. */
  readonly titleHit: boolean;
}

/**
 * One shard of the inverted index.  A shard groups postings
 * for all tokens sharing a common prefix (the `shardKey`),
 * so the browser can load just the shards relevant to a query
 * instead of the whole index.
 */
export interface IndexShard {
  /** The shared prefix all tokens in this shard start with. */
  readonly shardKey: string;
  /**
   * `token → postings`.  Postings are sorted by descending
   * `freq` to make top-k retrieval cheap at query time.
   */
  readonly postings: ReadonlyMap<string, readonly Posting[]>;
}

/**
 * The bootstrap manifest — small JSON the browser loads first
 * to learn which shards exist and which pages are indexed.
 */
export interface IndexManifest {
  /** Sorted list of all page ids in the index. */
  readonly pages: readonly string[];
  /** Sorted list of all shard keys. */
  readonly shardKeys: readonly string[];
  /** The `shardPrefix` option used at build time. */
  readonly shardPrefix: number;
  /** Whether `filterStopWords` was enabled at build time. */
  readonly filterStopWords: boolean;
  /** Whether `stem` was enabled at build time. */
  readonly stem: boolean;
  /** Aggregate statistics — useful for build-time reporting. */
  readonly stats: IndexStats;
}

export interface IndexStats {
  /** Total tokens fed into the index across all pages (incl. duplicates). */
  readonly totalTokens: number;
  /** Number of unique tokens in the index. */
  readonly uniqueTokens: number;
  /** Number of pages indexed. */
  readonly pageCount: number;
  /** Number of shards produced. */
  readonly shardCount: number;
}

/**
 * The output of `buildSearchIndex`.
 */
export interface BuildIndexOutput {
  readonly shards: ReadonlyMap<string, IndexShard>;
  readonly manifest: IndexManifest;
}
