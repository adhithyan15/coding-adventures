/**
 * @coding-adventures/forme-aot-fs-cache
 *
 * Disk-backed `CacheIO` for [`forme-aot-incremental-cache`](../forme-aot-incremental-cache).
 * FM06 §4 storage backend.
 *
 * ```ts
 * import { createIncrementalCache } from "@coding-adventures/forme-aot-incremental-cache";
 * import { createFsCacheIO } from "@coding-adventures/forme-aot-fs-cache";
 *
 * const cache = createIncrementalCache(createFsCacheIO({
 *   cacheDir: ".forme-cache",   // must exist
 * }));
 *
 * const result = await cache.sliceWithCache(doc, pages, {
 *   activeContexts: ["screen"],
 * });
 * ```
 *
 * Storage layout under `cacheDir`:
 *
 *   cacheDir/
 *     <first 2 hex>/
 *       <remaining 62 hex>.cache
 *
 * Sharded so any single sub-dir stays well below typical filesystem
 * directory entry limits. Temp-file replacement is enabled by
 * default: atomic on POSIX and process-serialized on Windows. Set
 * `atomicWrites: false` if the caller prevents concurrent writes.
 *
 * @module index
 */

export { createFsCacheIO } from "./fs-cache.js";
export type { FsCacheOptions } from "./fs-cache.js";
