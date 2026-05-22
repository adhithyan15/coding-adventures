# Changelog — @coding-adventures/forme-doc-search-index-builder

## 0.1.0 — 2026-05-22

Initial release.  Ninth concrete DOC00 v0 package — build-time
inverted-index builder for the documentation-site search.
Takes per-page text + metadata, builds a `token → postings`
inverted index, shards by token-prefix for incremental browser
loading, and emits the shards plus a small bootstrap manifest.

Pure transform: `{ pages: IndexPageInput[] }` →
`{ shards: Map<string, IndexShard>, manifest: IndexManifest }`.
Capabilities `[]`.  One transitive dependency
(`@coding-adventures/forme-doc-search-tokenizer`, itself
`[]`-capability and zero-dep).

### Added

- `buildSearchIndex(pages, options?): BuildIndexOutput` — main
  entry.  Validates input, tokenises each page, builds the
  inverted index, shards by token-prefix, emits sorted
  manifest.
- Types: `IndexPageInput`, `BuildIndexOptions`,
  `BuildIndexOutput`, `IndexShard`, `IndexManifest`,
  `IndexStats`, `Posting`.

### Spec adherence

Implements DOC00 v0's `forme-doc-search-index-builder` per
`code/specs/DOC00-docs-vision.md` — build-time inverted index,
sharded for incremental browser loading, with a small
bootstrap manifest.

v0 decision: postings record `freq` only (no positions).  v1
may add position lists if phrase-query support proves needed.

### Pipeline

1. **Validate** — reject if `pages.length > maxPages`; throw
   on duplicate page ids.
2. **Tokenise** each page's body + title via
   `forme-doc-search-tokenizer`.  Title hits set
   `titleHit: true` so query-time ranking can boost them.
   Per-page unique tokens capped at `maxTokensPerPage`.
3. **Inverted-index build** — `Map<token, Map<pageId, Posting>>`.
   Repeated token occurrences in the same page increment that
   posting's `freq`.  Per-token postings list capped at
   `maxPostingsPerToken`.
4. **Shard** by token prefix (`token.slice(0, shardPrefix)`).
   Within each shard, postings sorted by descending freq
   (ascending pageId tiebreak) for cheap top-k retrieval.
5. **Manifest** — sorted lists of all pages and all shard keys,
   plus the option flags used (so the query-side tokeniser
   can mirror them) and aggregate stats.

### Memory bounds

Three explicit caps protect against adversarial / overly-large
inputs:

- `maxPages` (default `100_000`): rejects `pages` longer than
  this with `TypeError`.
- `maxTokensPerPage` (default `10_000`): silently drops unique
  tokens beyond this per page.
- `maxPostingsPerToken` (default `10_000`): silently drops
  additional postings beyond this per token — the "the"
  mitigation.

### Behavioural notes

- **Pure transform.**  Input `pages` array and input page
  objects never mutated.  Verified by JSON snapshot test.
- **Deterministic.**  Same input bytes → identical output
  structure.  Stable sort (V8's `Array.prototype.sort` is
  stable since ES2019); stable shard-key derivation; sorted
  manifest lists.
- **Title hits surface in postings** (`titleHit: true`).  The
  builder doesn't implement weighting itself — it surfaces the
  signal for query-time ranking (TF-IDF / BM25 / custom).

### Security posture

- **No `eval` / `new Function` / `JSON.parse`-with-reviver** —
  pure data construction.
- **No mutation** of input (verified).
- **Bounded memory** via three explicit caps.
- **No regex DoS** — the tokeniser dependency uses explicit
  index loops (per the project-wide convention established by
  `forme-doc-sidebar-builder` and `forme-doc-page-shell`);
  this package adds no regex of its own.
- **No I/O** — capabilities `[]`.  Single transitive dep
  (`forme-doc-search-tokenizer`) is also `[]` and zero-dep.
  Caller is responsible for serialising the output to disk.
- **Total-ordering comparator** — within one shard, no two
  postings share both `freq` and `pageId` (duplicates throw
  at insert time), so the comparator is a strict total order.

### Tests

31 tests in `builder.test.ts`:

- Degenerate inputs (empty, empty-body page).
- Basic indexing (single page single token, freq counts,
  title hits, combined title + body).
- Multi-page (postings across pages, sorted by freq desc,
  alphabetical tiebreak).
- Sharding (default `shardPrefix=2`, `=1`, `=3`, short tokens
  use whole token as key).
- Options forwarding (filterStopWords on/off, stem on/off,
  customStopWords).
- Memory caps (maxPages throws, maxTokensPerPage caps,
  maxPostingsPerToken caps).
- Validation (duplicate id throws, shardPrefix < 1 throws).
- Manifest (sorted pages, sorted shardKeys, accurate stats,
  records option flags).
- Determinism + immutability.
- Realistic 5-page docs site.

Coverage: **99.3% line / 93.6% branch / 100% function** across
all source files with logic (`types.ts` is type-only).  The
remaining branch is a defensive `return 0` in the comparator
marked `/* v8 ignore */` — unreachable because duplicate
pageIds throw upstream.

### v0 simplifications (documented)

- **No positions** in postings (just freq).  v1 may add
  position lists for phrase queries.
- **No TF-IDF / BM25** — surfaces `titleHit` for query-time
  ranking but doesn't implement weighting itself.
- **No incremental rebuild** — every build rebuilds from
  scratch.  v1 may add a "rebuild only changed pages" mode
  with delta-merging of postings.
- **No multi-language index sharding** — all pages go into
  one index.  Multi-language sites build per-language indexes
  and route queries by detected language.
- **No fuzzy match indexing** — strict exact-token postings;
  typo tolerance is a query-time concern for
  `forme-doc-search-client-js`.
