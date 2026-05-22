# @coding-adventures/forme-doc-search-client-js

> Tenth DOC00 v0 package — browser-side search client logic.
> Consumes the manifest from `forme-doc-search-index-builder`
> plus an injected shard-fetcher callback; loads shards on
> demand, tokenises queries, merges postings, ranks results.

Pure transform. Capabilities: `[]`. The shard-fetcher
capability (`net:fetch` in the browser, `fs:read` on Node) lives
with the **caller** — this package itself never instantiates
any I/O primitive.

## What it does

```ts
import { SearchClient } from "@coding-adventures/forme-doc-search-client-js";

// Caller injects fetch — capability lives with them, not us.
const client = new SearchClient({
  manifest,                                 // from build-time index-builder
  fetchShard: async (key) => {
    const r = await fetch(`/search/${key}.json`);
    return await r.json();
  },
});

const results = await client.search("install");
// → [
//     { pageId: "/guide/setup", score: 4, matchedTokens: ["instal"] },
//     { pageId: "/intro",       score: 2, matchedTokens: ["instal"] },
//   ]
```

## Why injected fetch?

The standard advice "to do a network request, call `fetch()`" is
fine for app code. For a **library** in this repo's
capability-system, it would force the library to declare
`net:fetch` — which would then cascade as a required capability
into every consumer of the search client.

By injecting the fetcher, this package stays `[]`. The browser
caller (and only the browser caller) deals with `net:fetch`. A
Node-side caller can use `fs.readFile` instead. A unit-test
caller can use an in-memory map. Same code, different fetchers.

## Capability flow

```
Browser app code        ── has net:fetch ──┐
                                            ▼
                                  forme-aot-page-bundle-emitter
                                            ▼
                                       <script> wrapper
                                            ▼
                            new SearchClient({ fetchShard: fetch-wrapper, ... })
                                            ▼
                                     SearchClient class ── capability [] ──┐
                                            ▼                              │
                                    .search(query) ────────────────────────┘
```

The arrow into `SearchClient` is the capability boundary. The
class itself is pure.

## Lifecycle

```
construct  →  manifest in memory; no shards loaded.
.search(q) →  tokenise q (using manifest.filterStopWords and
              manifest.stem so client tokens match the index);
              for each unique query token:
                derive shardKey via token.slice(0, manifest.shardPrefix);
                if shard cached, look up;
                else if shard key is in manifest.shardKeys:
                  call fetchShard(shardKey), cache (LRU);
                else (shard key not in manifest), skip;
                look up postings, accumulate score per pageId
                  (score += freq * (titleHit ? titleBoost : 1));
              sort pageIds desc by score (ascending pageId tiebreak);
              return top `limit`.
```

## Public API

| Export                  | Purpose                                                                  |
|-------------------------|--------------------------------------------------------------------------|
| `SearchClient`          | Main class. `new SearchClient({ manifest, fetchShard, ... })`.            |
| `client.search(q, opts?)` | Run a query.  Returns `SearchResult[]`.                                  |
| `client.prefetchShards(keys)` | Pre-warm the cache.                                                |
| `client.clearCache()`   | Empty the cache.                                                          |
| `client.cacheSize`      | Current cache size (read-only).                                           |
| Types                   | `ShardFetcher`, `SearchClientOptions`, `SearchQueryOptions`, `SearchResult`. |

## Ranking (v0)

Simple and predictable:

```
score(page) = Σ over matched query tokens:
                postings[token][page].freq × (titleHit ? titleBoost : 1)
```

- `titleBoost` default: `2.0` — title matches count double.
- No TF-IDF, no BM25, no recency boost. v1 may add weighting
  schemes; v0 surfaces only the inputs (`freq` + `titleHit`).
- Tied scores break alphabetically by `pageId`.

## LRU cache

- Default `maxCachedShards = 50` (most sites have fewer
  shards than this; sites with more get LRU eviction).
- Cache is implemented as a `Map` — JS's insertion-order
  iteration doubles as LRU recency: every `.get()` does
  `delete-then-insert` to mark the entry as most-recently-used.
- Eviction removes the iterator's first (oldest) entry.

## Degrade gracefully

If `fetchShard` rejects (network error, 404, bad JSON, etc.) or
returns a malformed shape, the client **skips that shard**.
Search continues with whatever other shards loaded
successfully. A single broken shard doesn't blank out an
entire search session.

Callers wanting visibility into failures should wrap their own
`fetchShard` with logging/reporting before re-throwing.

## In-flight deduplication

Two concurrent `.search()` calls that both need shard `"in"`
share **one** outstanding fetch (via the `inflight` map).
Duplicate network requests for the same shard never fire.

## Security posture

- **No `eval` / `new Function` / `JSON.parse`-with-reviver.**
  Pure data manipulation; the caller's `fetchShard` is
  responsible for deserialisation.
- **No I/O primitives instantiated.** This is the whole point
  of the injected-fetcher pattern. Capabilities `[]`.
- **Numeric option validation at construction** — `maxCachedShards`
  and `titleBoost` are checked with `Number.isFinite`; NaN /
  Infinity / negative values throw immediately. Same for
  `limit` at search time.
- **Bounded memory.** LRU cap prevents the cache from growing
  unboundedly even on adversarial input.
- **No prototype pollution.** All `Map<string, *>` lookups use
  internal slots (not bracket-notation on plain objects);
  pageId / shardKey values are treated as opaque strings.
- **Malformed-shard guard.** Returning the wrong shape from
  `fetchShard` is treated as a fetch failure (caller's bug
  fails closed, not via type-confusion at use site).
- **Deterministic** for a fixed manifest + shard set — same
  query gives the same ranked output, modulo fetcher latency
  which only affects ordering of concurrent fetches not
  search results themselves.

## Tests

32 tests in `client.test.ts`:

- **Construction** — happy path, `maxCachedShards` < 1 / NaN
  throws, `titleBoost` negative / NaN throws.
- **Basic search** — matching page, title boost, multi-token
  score accumulation, `matchedTokens` populated.
- **Empty results** — empty query, all-stop-words query,
  query matching no shards, query matching shard but no token.
- **Limit option** — default 20, custom limit honoured, < 0
  throws, NaN throws.
- **Caching** — caches fetched shards, LRU evicts at cap,
  `clearCache` empties, `prefetchShards` warms, unknown shard
  keys silently skipped.
- **In-flight dedup** — concurrent searches for same shard
  share fetch.
- **Graceful failure** — rejecting fetcher → empty results
  (no throw), malformed shard treated as missing, partial
  failure → partial results.
- **Ranking** — higher freq → higher score, custom
  `titleBoost` overrides default, tied scores alphabetical
  pageId tiebreak.
- **Index/query option consistency** — uses manifest's
  `filterStopWords` and `stem` flags so client and index
  agree.
- **Realistic** — typical 5-page docs query.

Coverage: **99.3% line / 94.4% branch / 100% function** on all
source files with logic (`types.ts` is type-only). The
remaining branch is a defensive `return 0` in the result
comparator marked `/* v8 ignore */` — unreachable because
pageIds are unique within the score accumulator.

## How it fits in the stack

Tenth concrete DOC00 v0 package. Sits at runtime in the
browser, consuming both the tokeniser (for query
normalisation) and the index builder (for type shapes only —
the actual index data arrives at runtime via `fetchShard`):

```
build time                      runtime (browser)
────────────────────             ───────────────────────
search-index-builder          ┌─►  manifest.json  ──►   SearchClient (this package)
        │                     │                              │
        ▼                     │                              │
   shards/*.json   ◄──────────┘    user query  ──────────────┤
                                                             ▼
                                               results UI (consumer)
```

Final DOC00 v0 package: `forme-doc-site-emitter` (writes shards
to disk and wires the SearchClient into the page-shell's
search input).

## v0 simplifications (documented)

- **No fuzzy matching** — exact-token match only. Typo
  tolerance (Levenshtein distance, etc.) is left to v1.
- **No query syntax** — no `+`, `-`, `"..."`, field-specific
  searches. Just whitespace-separated keywords.
- **No incremental result streaming** — `.search()` returns a
  single Promise resolving with the full result list.
- **No "instant search" / debouncing** — caller's UI concern.
- **No analytics / popular-queries tracking** — caller's
  concern; v1+ may add a privacy-preserving variant.
- **No search-result highlighting** — `matchedTokens` is
  surfaced but the caller does the actual HTML markup.
