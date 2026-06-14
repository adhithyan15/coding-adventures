/**
 * @coding-adventures/forme-cache
 *
 * Persistent cache layer for the Forme orchestrator (FM03 §5).
 *
 * Three concerns:
 *
 *   - **Storage adapters.**  `CacheBackend` interface plus two
 *     built-ins: `memoryCache()` (in-process, lost on dispose) and
 *     `filesystemCache(root)` (sharded under a directory tree).  A
 *     future `RemoteCache` slots into the same interface.
 *
 *   - **Key derivation.**  `cacheKey(input)` produces a deterministic
 *     BLAKE2b-256 hex digest from `(stage_name, stage_version, config,
 *     input_revision, capabilities)`.  `capabilitySetHash(...)` is the
 *     order-insensitive sub-hash for capability sets.
 *
 *   - **Integrity.**  Every read recomputes the payload's BLAKE2b
 *     digest and compares with the stored `contentHash`.  Mismatch
 *     ⇒ treat as miss + invalidate.  `verifyEntry`, `computeContentHash`,
 *     `makeEntry` are exposed for tests and orchestrator wiring.
 */

export type { CacheBackend, CacheEntry } from "./types.js";

export {
  CACHE_KEY_VERSION,
  CACHE_KEY_DIGEST_BYTES,
  cacheKey,
  capabilitySetHash,
} from "./keys.js";
export type { CacheKeyInput } from "./keys.js";

export {
  computeContentHash,
  verifyEntry,
  makeEntry,
} from "./integrity.js";

export { memoryCache } from "./memory.js";
export { filesystemCache } from "./filesystem.js";
