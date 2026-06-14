# @coding-adventures/forme-doc-search-index-builder

> Ninth DOC00 v0 package — build-time inverted-index builder
> for the documentation-site search. Takes per-page text +
> metadata, builds a `token → postings` inverted index, shards
> by token-prefix for incremental browser loading, and emits
> the shards plus a small bootstrap manifest.

Pure transform. Capabilities: `[]`. Depends only on
`@coding-adventures/forme-doc-search-tokenizer` (itself
`[]`-capability and zero-dep).

## What it does

```ts
import { buildSearchIndex } from "@coding-adventures/forme-doc-search-index-builder";

const { shards, manifest } = buildSearchIndex([
  { id: "/intro",       body: "Welcome to the docs", title: "Introduction" },
  { id: "/guide/setup", body: "Install via npm install foo", title: "Setup Guide" },
]);

// manifest.pages       = ["/guide/setup", "/intro"]                  (sorted)
// manifest.shardKeys   = ["do", "fo", "gu", "in", "no", "se", "we"]  (sorted)
// shards.get("fo").postings.get("foo") = [{pageId:"/guide/setup", freq:1, titleHit:false}]
```

The caller is responsible for **serialising the output to disk**.
This package has no fs access (capabilities `[]`); it returns
in-memory data structures that the `forme-doc-site-emitter`
package writes out as JSON.

## What is an inverted index?

The straightforward "page → terms" layout is the inverse of
what queries need:

```
forward:   "intro.md" → ["welcome", "guide", "setup"]
inverted:  "welcome" → [{pageId: "intro.md", freq: 1}, ...]
           "guide"   → [{pageId: "intro.md", freq: 1}, ...]
           "setup"   → [{pageId: "intro.md", freq: 1}, ...]
```

Looking up pages matching "setup" is O(1) in the inverted
index; scanning all pages would be O(N) per query.

## Sharding

For browser-side search, we DON'T want to ship the whole index
upfront. The output is sharded by token prefix: shard `"ge"`
holds all tokens starting with `"ge"`. When the user types
`"getting"`, the client tokenises it, looks up `"ge"` in the
manifest's `shardKeys`, fetches just that shard, and searches
within it.

| `shardPrefix` | Typical shard count (English) | Per-shard size | Trade-off                        |
|---------------|-------------------------------|----------------|----------------------------------|
| `1`           | ~26                           | Larger          | Few shards, simpler routing      |
| `2` (default) | ~600                          | Medium          | Good balance for most sites      |
| `3`           | ~5k                           | Smaller         | Many shards, finer-grained load  |

## Postings

Each posting records that a token appears in a page:

```ts
interface Posting {
  readonly pageId: string;
  readonly freq: number;       // term frequency in body + title
  readonly titleHit: boolean;  // true if token also appeared in title
}
```

Postings within each token's list are sorted by **descending
frequency** (then ascending pageId for stable tiebreak), so
top-k retrieval at query time is cheap (just take the first
k entries).

## Memory bounds

Adversarial or just very large inputs can produce indexes far
larger than the browser will tolerate. Three caps protect this:

| Cap                    | Default   | What happens beyond                                    |
|------------------------|-----------|--------------------------------------------------------|
| `maxPages`             | `100_000` | Throws `TypeError` (fail fast at build time)            |
| `maxTokensPerPage`     | `10_000`  | Silently drops tokens beyond cap for that page          |
| `maxPostingsPerToken`  | `10_000`  | Silently drops additional postings for that token       |

The `maxPostingsPerToken` cap is the "the" mitigation:
extremely common tokens that match essentially every page
would otherwise produce huge postings lists with low search
value.

## Public API

| Export                  | Purpose                                                                  |
|-------------------------|--------------------------------------------------------------------------|
| `buildSearchIndex(pages, options?)` | Main entry. Returns `{ shards, manifest }`.                  |
| Types                   | `IndexPageInput`, `BuildIndexOptions`, `BuildIndexOutput`, `IndexShard`, `IndexManifest`, `IndexStats`, `Posting`. |

## Pipeline summary

1. **Validate** input — reject if `pages.length > maxPages`,
   throw on duplicate page ids.
2. **Tokenise** each page's body (and title, if present) via
   `@coding-adventures/forme-doc-search-tokenizer`. Tokens
   beyond `maxTokensPerPage` are dropped.
3. **Build inverted index** — `Map<token, Map<pageId, Posting>>`.
   For each token-occurrence, increment the matching posting's
   `freq`; for title hits, also set `titleHit: true`. Per-token
   posting list capped at `maxPostingsPerToken`.
4. **Shard by token prefix** — `Map<shardKey, IndexShard>`.
   Within each shard, postings lists are sorted by descending
   freq (ascending pageId tiebreak) for cheap top-k retrieval.
5. **Emit manifest** — sorted lists of `pages` and `shardKeys`,
   plus option flags and aggregate stats.

## Security posture

- **No `eval` / `new Function` / `JSON.parse`-with-reviver.**
  Pure data construction.
- **No mutation of input.** Verified by JSON snapshot test.
- **Bounded memory.** Three explicit caps (pages, tokens-per-page,
  postings-per-token) prevent adversarial inputs from
  producing pathologically large indexes.
- **No regex DoS.** The tokeniser dependency follows the
  project-wide convention of explicit index loops; this
  package adds no regex of its own.
- **No I/O.** Capabilities `[]`. Single transitive dep
  (`forme-doc-search-tokenizer`) is also `[]` with zero deps.
- **Deterministic.** Same input bytes → identical output
  bytes. Stable sort, stable shard-key derivation, stable
  manifest ordering.
- **Total-ordering comparator** — within one shard, no two
  postings share both `freq` and `pageId` (duplicates throw
  upstream), so the comparator gives a strict total order.

## Tests

31 tests in `builder.test.ts`:

- **Degenerate** — empty input, empty-body page.
- **Basic indexing** — single page with single token, freq
  counts, title hits, title-and-body combined.
- **Multi-page** — token in multiple pages, postings sorted
  by freq desc, alphabetical tiebreak.
- **Sharding** — default `shardPrefix=2`, `shardPrefix=1`,
  `shardPrefix=3`, short tokens use whole token as key.
- **Options forwarding** — `filterStopWords` on/off, `stem`
  on/off, `customStopWords`.
- **Memory caps** — `maxPages` throws, default allows 100k,
  `maxTokensPerPage` silently caps, `maxPostingsPerToken`
  silently caps.
- **Validation** — duplicate page id throws,
  `shardPrefix < 1` throws.
- **Manifest** — sorted pages, sorted shardKeys, accurate stats,
  records option flags.
- **Determinism + immutability** — same input identical
  output; no mutation of input.
- **Realistic** — typical 5-page docs site.

Coverage: **99.3% line / 93.6% branch / 100% function** on all
source files with logic (`types.ts` is type-only). The
remaining branch is a defensive `return 0` in the comparator
marked `/* v8 ignore */` — unreachable because duplicate
pageIds throw upstream.

## How it fits in the stack

Ninth concrete DOC00 v0 package. Sits at build time, between
the tokeniser and the site emitter:

```
page bodies + titles
        ↓
  search-tokenizer (per page)
        ↓
  index-builder (this package)
        ↓
  { shards, manifest } ─► written to disk by forme-doc-site-emitter
                              ↓
                      browser fetches manifest at boot
                      browser fetches shards on demand
                              ↓
                      forme-doc-search-client-js (runtime)
```

Remaining DOC00 v0 packages: `forme-doc-search-client-js`,
`forme-doc-site-emitter`.

## v0 simplifications (documented)

- **No positions** in postings (just freq). v1 may add
  position lists for phrase queries.
- **No field weighting beyond `titleHit`** — body and title
  hits both count toward freq equally; the `titleHit` flag is
  surfaced for query-time ranking but the builder doesn't
  implement TF-IDF or BM25.
- **No incremental rebuild** — every build rebuilds from
  scratch. v1 may add a "rebuild only changed pages" mode.
- **No multi-language index sharding** — all pages go into one
  index. Multi-language sites should build per-language
  indexes and route queries by detected language.
- **No fuzzy match indexing** — strict exact-token postings.
  Typo tolerance is a query-time concern for
  `forme-doc-search-client-js`.
