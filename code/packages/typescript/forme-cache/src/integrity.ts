/**
 * Integrity verification — defends against silent cache corruption.
 *
 * Per FM03 §5.4, every cache read recomputes the BLAKE2b digest of
 * the payload and compares with the stored `contentHash`.  A mismatch
 * is treated the same as a cache miss: the orchestrator re-executes
 * the stage, and the corrupt entry should be invalidated to clean up
 * the tampered/damaged storage.
 *
 * Why bother?  Three failure modes this catches:
 *
 *   1. **Disk corruption.**  Bit rot on long-running build servers.
 *   2. **Concurrent writes.**  A racing process writing under the
 *      same key (unlikely with content-addressed keys but possible
 *      if hash derivation has a bug).
 *   3. **Tampering.**  A malicious or accidental byte-level edit of
 *      the cache directory.
 *
 * The check is cheap — one BLAKE2b pass over the payload, ~250 MB/s
 * — and the alternative (returning corrupted data to the orchestrator)
 * is silent wrong-answers.  Always-on integrity check.
 */

import { blake2bHex } from "@coding-adventures/blake2b";
import { CACHE_KEY_DIGEST_BYTES } from "./keys.js";
import type { CacheEntry } from "./types.js";

/**
 * Compute the canonical content hash for a payload.  Used by both
 * `put` (to populate `contentHash`) and `get` (to verify it).
 */
export function computeContentHash(payload: Uint8Array): string {
  return blake2bHex(payload, { digestSize: CACHE_KEY_DIGEST_BYTES });
}

/**
 * Verify a `CacheEntry` against its stored content hash.  Returns
 * `true` if the hash matches and the entry is internally consistent,
 * `false` if it looks corrupt.
 *
 * Also catches a few cheap structural issues — `sizeBytes` mismatch
 * with `payload.byteLength` is an early signal that storage layer
 * is lying to us.
 */
export function verifyEntry(entry: CacheEntry): boolean {
  if (entry.sizeBytes !== entry.payload.byteLength) return false;
  return computeContentHash(entry.payload) === entry.contentHash;
}

/**
 * Build a fresh `CacheEntry` from raw payload bytes, populating the
 * derived fields (`writtenMs`, `sizeBytes`, `contentHash`) consistently.
 */
export function makeEntry(payload: Uint8Array, now: () => number = Date.now): CacheEntry {
  return {
    writtenMs:   now(),
    sizeBytes:   payload.byteLength,
    payload,
    contentHash: computeContentHash(payload),
  };
}
