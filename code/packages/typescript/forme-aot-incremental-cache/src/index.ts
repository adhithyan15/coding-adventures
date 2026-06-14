/**
 * @coding-adventures/forme-aot-incremental-cache
 *
 * Content-addressed incremental rebuild cache wrapping
 * [`forme-aot-css-slicer`](../forme-aot-css-slicer).  FM06 §4.
 *
 * ```ts
 * import { createIncrementalCache, createMemoryCacheIO } from "@coding-adventures/forme-aot-incremental-cache";
 *
 * const cache = createIncrementalCache(createMemoryCacheIO());
 *
 * // First call — all cache misses, slicer runs for every page.
 * const r1 = await cache.sliceWithCache(doc, pages, { activeContexts: ["screen"] });
 * for (const [pageId, art] of r1.artefacts) {
 *   console.log(pageId, art.cacheHit, art.cacheKey.slice(0, 8));
 * }
 *
 * // Second call with the SAME inputs — every page is a cache hit.
 * const r2 = await cache.sliceWithCache(doc, pages, { activeContexts: ["screen"] });
 * for (const art of r2.artefacts.values()) {
 *   assert(art.cacheHit === true);
 * }
 * ```
 *
 * Cache key shape:
 *
 *   sha256(canonicalStyleDocument(doc) + "\n"
 *        + JSON(sort(usedRuleIds))     + "\n"
 *        + JSON(sort(activeContexts)))
 *
 * — order-stable, so callers don't have to canonicalise their
 * inputs.  Pages with identical `usedRuleIds` collapse to one
 * cache entry but still get per-page-scoped deliverables on serve.
 *
 * Storage IO is **injected** via `CacheIO` — this package has no
 * direct fs / net dependency.  Production callers wire `CacheIO`
 * to disk in their own package (which declares `"fs"` capability
 * there, not here).
 *
 * @module index
 */

export {
  createIncrementalCache, createMemoryCacheIO,
} from "./cache.js";
export type {
  CacheIO, CachePutMeta, CacheArtifact, IncrementalCache,
  SliceWithCacheResult,
} from "./cache.js";
