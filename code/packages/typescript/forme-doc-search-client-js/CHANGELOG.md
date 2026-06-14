# Changelog — @coding-adventures/forme-doc-search-client-js

## 0.1.0 — 2026-05-22

Initial release.  Tenth concrete DOC00 v0 package —
browser-side search client logic.  Consumes the manifest from
`forme-doc-search-index-builder` plus an INJECTED shard-fetcher
callback; loads shards on demand, tokenises queries via
`forme-doc-search-tokenizer`, merges postings, ranks results.

Pure transform / class.  Capabilities: `[]`.  The shard-fetcher
capability (`net:fetch` in the browser, `fs:read` on Node) lives
with the CALLER — this package itself never instantiates any
I/O primitive.

### Added

- `SearchClient` class.  Constructor takes
  `{ manifest, fetchShard, maxCachedShards?, titleBoost? }`.
- `client.search(query, options?): Promise<SearchResult[]>` —
  the main entry.
- `client.prefetchShards(keys): Promise<void>` — pre-warm the
  cache.
- `client.clearCache(): void` — empty the cache.
- `client.cacheSize: number` — getter for the current cache size.
- Types: `ShardFetcher`, `SearchClientOptions`,
  `SearchQueryOptions`, `SearchResult`.

### Spec adherence

Implements DOC00 v0's `forme-doc-search-client-js` per
`code/specs/DOC00-docs-vision.md` — browser-side search client
that loads the manifest at boot, fetches shards on-demand,
runs the query through the same tokeniser pipeline, returns
ranked results.

Decision: ship as a PURE LIBRARY with an INJECTED fetcher
rather than a self-contained browser bundle.  The browser-side
glue (script tag, IIFE wrapper, fetch wrapper) belongs in
`forme-doc-site-emitter` / `forme-aot-page-bundle-emitter`,
not here.  Keeping the fetcher injected makes the class
trivially testable (in-memory mock) and reusable in non-browser
contexts.

### Behavioural notes

- **Pure transform / class.**  No global state; no I/O
  primitives instantiated; no `eval`.
- **Capability `[]`.**  The shard-fetcher capability lives
  with the caller.  This package only ever calls the
  user-supplied callback.
- **Manifest-driven tokenisation.**  Client tokenises queries
  using the manifest's `filterStopWords` and `stem` flags —
  so client and index agree without the caller needing to
  pass them twice.
- **LRU shard cache.**  Default `maxCachedShards = 50`.
  Implemented via Map insertion-order: every `.get()` does
  `delete-then-insert` to refresh recency; eviction removes
  the iterator's first entry.
- **In-flight deduplication.**  Concurrent searches for the
  same shard share one Promise — no duplicate network requests.
- **Degrade gracefully.**  If `fetchShard` rejects or returns
  a malformed shape, the client skips that shard.  Search
  continues with whatever other shards loaded successfully.
- **Deterministic** for a fixed manifest + shard set.

### Ranking (v0)

Simple sum-of-(freq × titleBoost-when-titleHit) per page.
No TF-IDF, no BM25, no recency boost.  v1 may add weighting
schemes; v0 surfaces only the raw inputs.  Tied scores break
alphabetically by `pageId`.

### Security posture

- **No `eval` / `new Function` / `JSON.parse`-with-reviver** —
  pure data manipulation; the caller's `fetchShard` is
  responsible for deserialisation.
- **No I/O primitives instantiated.**  Capabilities `[]`.
- **Numeric option validation at construction** —
  `maxCachedShards` and `titleBoost` checked with
  `Number.isFinite`; NaN/Infinity/negative throws.  Same for
  `limit` at search time.
- **Bounded memory** via LRU cache cap.
- **No prototype pollution** — all `Map<string, *>` lookups use
  internal slots; pageId/shardKey treated as opaque strings.
- **Malformed-shard guard** — `isLikelyShard` verifies basic
  shape; a malformed response is treated as a fetch failure
  (no type-confusion at use site).
- **Deterministic comparator** with total ordering — defensive
  `return 0` for impossible (same pageId twice in scores) case
  marked `/* v8 ignore */`.

### Tests

32 tests in `client.test.ts`:

- Construction (happy + 4 invalid-option-type throws).
- Basic search (matching page, title boost, multi-token
  accumulation, matchedTokens).
- Empty results (empty query, all-stop-words, no-shard match,
  shard match but no token match).
- Limit option (default 20, custom, < 0 throws, NaN throws).
- Caching (caches, LRU evicts, clearCache, prefetchShards
  warms, prefetch unknown skipped).
- In-flight dedup (concurrent searches share fetch).
- Graceful failure (rejecting fetcher → empty results, malformed
  shard → treated as missing, partial failure → partial results).
- Ranking (freq → score, custom titleBoost, tied → alphabetical).
- Index/query option consistency (manifest's filterStopWords
  and stem flags honoured).
- Realistic 5-page docs query.

Coverage: **99.3% line / 94.4% branch / 100% function** across
all source files with logic (`types.ts` is type-only).  The
remaining branch is a defensive `return 0` in the result
comparator marked `/* v8 ignore */`.

### v0 simplifications (documented)

- **No fuzzy matching** — exact-token match only.
- **No query syntax** (`+`/`-`/`"..."`/field-specific).
- **No incremental streaming** — single Promise per `.search()`.
- **No "instant search" / debouncing** — caller's UI concern.
- **No analytics** — left to v1+.
- **No result highlighting** — `matchedTokens` is surfaced;
  the caller does the actual HTML markup.
