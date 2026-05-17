# Changelog — @coding-adventures/forme-aot-incremental-cache

## 0.1.0 — 2026-05-17

Initial release.  Second package of the FM06 AOT compiler family.
Wraps `forme-aot-css-slicer` with content-addressed caching for
fast incremental rebuilds.

### Added

- `createIncrementalCache(io: CacheIO): IncrementalCache` — factory
  that wires the cache to an injectable storage backend.
- `IncrementalCache.sliceWithCache(doc, pages, options)` — same
  shape as `slicePerPage` but consults cache before recomputing.
  Returns `CacheArtifact[]` carrying `cacheHit: boolean` and
  `cacheKey: string` per page.
- `IncrementalCache.cacheKey(doc, usedRuleIds, activeContexts)` —
  expose the deterministic key derivation so callers can pre-check
  / prime entries / build dependency graphs without invoking the
  slicer.
- `createMemoryCacheIO()` — convenience in-memory IO with
  `clear()` / `size()` (tests + dev mode).
- Types: `CacheIO`, `CachePutMeta`, `CacheArtifact`,
  `IncrementalCache`, `SliceWithCacheResult`.

### Cache-key shape

```
sha256(canonicalStyleDocument(doc) + "\n"
     + JSON(sort(usedRuleIds))     + "\n"
     + JSON(sort(activeContexts)))
```

- `canonicalStyleDocument` — FM04 §12 byte-stable JSON.
- `usedRuleIds`, `activeContexts` — sorted before JSON-stringify.

Permutations of the same input collapse to one key.

### Spec adherence

Implements FM06 §4 (incremental rebuilds) and FM03 reproducibility.
No spec divergences.

### Behavioural notes

- **Storage is the UNSCOPED CSS bytes.**  Computed by calling
  `translateToCss(doc, { activeContexts, usedRuleIds })` directly
  (no scope) — same shape as the slicer's own internal unscoped
  pass.  On hit, the package calls the translator a second time
  WITH the scope to produce the scoped deliverable.  No
  reverse-engineering of CSS selector strings (a footgun for any
  future widening of the property set with `content: "..."` or
  attribute-selector quoted values).  Two pages with identical
  `usedRuleIds` share one cache entry while still shipping
  per-page-scoped deliverables.
- **`emittedRules`, `warnings`, `sha256` round-trip via JSON
  serialisation.**  Cache values are JSON blobs (small per page);
  compression is left to the storage backend.
- **Order-independence.**  `usedRuleIds` and `activeContexts` are
  sorted before hashing; canonical doc serialisation handles
  token-key reordering.  Reshuffled inputs hit cache on second
  call.
- **Malformed cache entries fall through to fresh compute.**  Two
  defensive paths: `JSON.parse` failure → null; missing required
  fields → null.  Tests pin both.
- **Concurrent slices race-tolerant.**  Two simultaneous misses
  both write; second write idempotently overwrites with identical
  content.  Subsequent calls hit.
- **Page iteration order preserved.**  Returned `Map` iterates in
  caller's input array order; no sort.

### Security posture

- **Cache-key collision resistance.**  sha256 over a canonical
  input.  Cache-key collisions would cause two distinct documents
  to silently serve each other's CSS — sha256 makes this
  computationally infeasible.
- **No credential leakage.**  Cache values are pure
  StyleDocument-derived CSS; no environment variables, file
  paths, or other caller-side data captured.  Injectable
  `CacheIO` means production callers own storage ACLs.
- **Defensive cache-entry parsing.**  Tests prime malformed entries
  (invalid JSON; valid JSON missing fields) and confirm the cache
  falls through to fresh compute rather than throwing.

### Capabilities

`["hash"]` — `node:crypto.createHash("sha256")` for cache keys.
The FS / network IO is **injected** via `CacheIO`; this package
declares NO `fs` capability.  Production callers that wire
`CacheIO` to disk declare `"fs"` in their own package.

### Tests

20 tests in `cache.test.ts`:

- Miss → hit (4 — first miss; second hit; CSS round-trip equality;
  metadata round-trip)
- Change invalidation (4 — usedRuleIds; activeContexts; doc;
  canonical-equal docs hash identically)
- Order independence (3 — usedRuleIds; activeContexts; end-to-end
  reshuffled inputs)
- Same-key sharing across pages (1)
- Manual clear + frozen list (2)
- Concurrency (1)
- Warning propagation (1)
- Malformed cache entry (2)
- byteSize round-trip equality (1)
- Page iteration order preserved (1)

Coverage: **100% line / 88.63% branch** — line well above the FM04
§14.4 ≥95% line target.

### v0 simplifications (documented)

- **No eviction policy.**  Production callers' `CacheIO`
  implementations own eviction.
- **No concurrent-write coordination.**  Idempotent overwrites
  are sufficient; no double-execute on subsequent hits.
- **CSS only.**  Same pattern works for LaTeX / terminal slicers
  but isn't shipped yet (no slicers for those targets either).
