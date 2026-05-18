# @coding-adventures/forme-aot-incremental-cache

Content-addressed incremental rebuild cache wrapping
[`forme-aot-css-slicer`](../forme-aot-css-slicer).  FM06 §4.

Second package of the FM06 AOT compiler family.  Wraps the per-page
slicer with a deterministic cache so identical inputs across rebuild
cycles short-circuit to a cache lookup instead of re-running the
translator.

## Quick start

```ts
import {
  createIncrementalCache, createMemoryCacheIO,
} from "@coding-adventures/forme-aot-incremental-cache";

const cache = createIncrementalCache(createMemoryCacheIO());

// First call — all cache misses; slicer runs for every page.
const r1 = await cache.sliceWithCache(doc, pages, { activeContexts: ["screen"] });
for (const [pageId, art] of r1.artefacts) {
  console.log(pageId, art.cacheHit, art.cacheKey.slice(0, 8));
}

// Second call with the SAME inputs — every page is a cache hit.
const r2 = await cache.sliceWithCache(doc, pages, { activeContexts: ["screen"] });
for (const art of r2.artefacts.values()) {
  console.log(art.cacheHit);  // true
}
```

## Cache-key shape

```
sha256(canonicalStyleDocument(doc) + "\n"
     + JSON(sort(usedRuleIds))     + "\n"
     + JSON(sort(activeContexts)))
```

- **`canonicalStyleDocument`** — FM04 §12 byte-stable JSON; sorted
  keys at every depth.  Two semantically-equal docs hash identically
  regardless of token-bucket ordering.
- **`usedRuleIds`** — sorted lexicographically before hashing.
- **`activeContexts`** — sorted lexicographically before hashing.

So permutations of the same input collapse to one key.  Callers
don't need to canonicalise themselves.

## Storage IO — injected

`createIncrementalCache(io: CacheIO)` takes a storage backend:

```ts
interface CacheIO {
  get(key: string): Promise<string | null>;
  put(key: string, value: string, meta: CachePutMeta): Promise<void>;
  list(): Promise<readonly string[]>;
}
```

Production callers wire `CacheIO` to disk or network in their own
package — declaring the matching capability (`"fs"`, `"net"`) there,
**not here**.  This package stays `capabilities: ["hash"]` only.

For tests / dev mode, the package ships a one-line
`createMemoryCacheIO()` helper (no eviction, no persistence — just
a `Map`).

## What's in a `CacheArtifact`?

```ts
interface CacheArtifact {
  pageId: string;
  css: string;                     // per-page-scoped CSS deliverable
  emittedRules: readonly StyleRuleId[];
  warnings: readonly StyleWarning[];
  byteSize: number;
  sha256: string;                  // content-addressed fingerprint
  cacheHit: boolean;               // ← cache-aware addition
  cacheKey: string;                // ← deterministic key (debugging)
}
```

The `cacheHit` flag and `cacheKey` are the only fields beyond
the upstream `CssArtifact`; everything else round-trips.

## Same-key sharing across pages

Two pages with identical `usedRuleIds` share **one cache entry**:

```ts
const pages = [
  { id: "/a.html", usedRuleIds: ["body"] },
  { id: "/b.html", usedRuleIds: ["body"] },   // same key
];
const r = await cache.sliceWithCache(doc, pages, { activeContexts: [] });
// io.size() === 1  — one stored entry
// r.artefacts.get("/a.html").cacheHit === false
// r.artefacts.get("/b.html").cacheHit === true   ← observed A's put
```

The stored entry is the **unscoped** CSS bytes; on serve we re-apply
the per-page scope.  So two pages share storage cost but ship
distinct per-page-scoped deliverables.

## Capabilities — `["hash"]`

Uses `node:crypto.createHash("sha256")` for the cache key.  No
filesystem, no network — `CacheIO` is the only side-effect surface
and it's injected.

## Security posture

Three concerns explicitly addressed:

1. **Cache-key collision resistance.**  sha256 over a canonical
   input.  Cache-key collisions would cause two distinct documents
   to silently serve each other's CSS — sha256 makes this
   computationally infeasible.
2. **Malformed cache-entry defence.**  `JSON.parse` failures and
   missing fields fall back to fresh compute (no exception
   surfaces to the caller).  Tests pin both paths.
3. **No credential leakage via cache.**  Cache values are pure
   StyleDocument-derived CSS; they don't capture environment
   variables, file paths, or any caller-side secrets.  The
   injectable `CacheIO` means production callers control storage
   ACLs — this package never reads from disk directly.

## Reproducibility (FM03)

`sliceWithCache(doc, pages, options)` is deterministic — same
triple → byte-identical artefact map (both fresh and from cache).
The `cacheKey` field is exposed so external systems (build
fingerprints, telemetry, dedup pipelines) can use it without
re-deriving.

## Tests

20 tests in `cache.test.ts`:

- Miss → hit round-trip (4)
- Change-driven invalidation: usedRuleIds, activeContexts, doc;
  canonical-equal docs hash identically (4)
- Order independence: usedRuleIds, activeContexts, end-to-end (3)
- Same-key sharing across pages (1)
- Manual clear + `list()` semantics (2)
- Concurrency: two simultaneous misses; subsequent hit (1)
- Warning propagation across hits (1)
- Malformed cache entry → fresh compute (2)
- byteSize round-trip equality (1)
- Page iteration order preserved (1)

Coverage: **100% line / 88.63% branch** — line well above the FM04
§14.4 ≥95% line target.  Uncovered branches are defensive
null-checks in the cache-entry parser (each branch reachable only
via specific malformed JSON shapes; the "any malformed → fall
through" test exercises one).

## Spec adherence

Implements FM06 §4 (incremental rebuilds) and the FM03 reproducibility
contract.  No spec divergences.

## v0 simplifications

- **No eviction policy.**  In-memory IO grows unbounded; production
  callers' `CacheIO` implementations are where eviction lives.
- **No concurrent-write coordination.**  Two simultaneous misses
  both write — the second write idempotently overwrites with
  identical content.  No race condition, no double-execute on
  subsequent hits.
- **CSS only.**  Same pattern would work for LaTeX / terminal slicers
  but isn't shipped yet (no slicer for those targets either).
