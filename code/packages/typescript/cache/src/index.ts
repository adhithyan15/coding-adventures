/**
 * Cache — configurable CPU cache hierarchy simulator.
 *
 * This package simulates a multi-level cache hierarchy like those found in
 * modern CPUs. The same Cache class serves as L1, L2, or L3 by configuring
 * size, associativity, and latency differently.
 *
 * Modules:
 *     cache-line  - CacheLine: the smallest unit of cached data
 *     cache-set   - CacheSet + CacheConfig: set-associative lookup with LRU
 *     cache       - Cache: a single configurable cache level
 *     hierarchy   - CacheHierarchy: L1I/L1D/L2/L3 composition
 *     stats       - CacheStats: hit rate, miss rate, eviction tracking
 *
 * Quick start:
 * ```ts
 * import { Cache, CacheConfig, CacheHierarchy } from "@coding-adventures/cache";
 * const l1d = new Cache(new CacheConfig("L1D", 1024, 64, 4, 1));
 * const l2  = new Cache(new CacheConfig("L2", 4096, 64, 8, 10));
 * const hierarchy = new CacheHierarchy({ l1d, l2 });
 * const result = hierarchy.read(0x1000);
 * result.servedBy;  // "memory"
 * ```
 */

export { Cache } from "./cache.js";
export type { CacheAccess } from "./cache.js";
export { CacheLine } from "./cache-line.js";
export { CacheConfig, CacheSet } from "./cache-set.js";
export type { WritePolicy } from "./cache-set.js";
export { CacheHierarchy } from "./hierarchy.js";
export type { HierarchyAccess } from "./hierarchy.js";
export { CacheStats } from "./stats.js";
