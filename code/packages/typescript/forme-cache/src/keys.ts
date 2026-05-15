/**
 * Cache-key derivation per FM03 §5.2.
 *
 * The key is a BLAKE2b-256 hex digest over a fixed-format byte
 * sequence:
 *
 *     "forme-cache-v1\0"
 *  || stage_name
 *  || "\0"
 *  || stage_version
 *  || "\0"
 *  || canonical_json(stage_config)
 *  || "\0"
 *  || input_revision
 *  || "\0"
 *  || capability_set_hash
 *
 * The leading `"forme-cache-v1\0"` magic is a kernel-version barrier:
 * if we ever change the key derivation contract (e.g. switch to
 * BLAKE3, add a new field), bumping the magic string invalidates the
 * entire cache without a manual flush.
 *
 * Each field is separated by a NUL byte so adjacent fields can't
 * collide via concatenation: `("ab", "c")` and `("a", "bc")` would
 * hash to the same value without a separator.  NUL is forbidden in
 * the kernel's identifier alphabets (capability strings, stage names,
 * versions) so it can't appear inside a field.
 *
 * The capability_set_hash is computed by sorting the capability
 * strings, joining with NUL, and hashing once with BLAKE2b — keeps
 * the cache key bounded regardless of how many capabilities a stage
 * declares.
 */

import { blake2bHex } from "@coding-adventures/blake2b";
import { canonicalJson } from "@coding-adventures/forme-identity";
import type { JsonValue, RevisionId } from "@coding-adventures/forme-types";

/** Magic-string prefix.  Bump on any breaking change to the contract. */
export const CACHE_KEY_VERSION = "forme-cache-v1" as const;

/** Digest length in bytes (256 bits).  Matches forme-identity's revision hash. */
export const CACHE_KEY_DIGEST_BYTES = 32;

const SEP_BYTE = 0x00; // NUL separator between fields

export interface CacheKeyInput {
  /** Stage's package name (e.g. `"@forme/parse-markdown"`). */
  readonly stageName: string;
  /** Stage's semver string. */
  readonly stageVersion: string;
  /**
   * Stage's `config` value — exactly what the orchestrator passes to
   * `Stage.run`.  Hashed via canonical JSON so key-order changes don't
   * break cache reuse.
   */
  readonly stageConfig: JsonValue;
  /** Revision of the input.  For sources, this is the "current state" digest. */
  readonly inputRevision: RevisionId;
  /** Capabilities the stage instance was granted.  Order-insensitive. */
  readonly capabilities: readonly string[];
}

/**
 * Derive a deterministic cache key for one stage invocation.
 *
 * The same inputs always produce the same key; any change in any
 * field produces a different key with overwhelming probability.
 */
export function cacheKey(input: CacheKeyInput): string {
  const canonical = canonicalJson(input.stageConfig);
  const capHash = capabilitySetHash(input.capabilities);

  const parts: Uint8Array[] = [
    encode(CACHE_KEY_VERSION),
    new Uint8Array([SEP_BYTE]),
    encode(input.stageName),
    new Uint8Array([SEP_BYTE]),
    encode(input.stageVersion),
    new Uint8Array([SEP_BYTE]),
    encode(canonical),
    new Uint8Array([SEP_BYTE]),
    encode(input.inputRevision),
    new Uint8Array([SEP_BYTE]),
    encode(capHash),
  ];

  return blake2bHex(concat(parts), { digestSize: CACHE_KEY_DIGEST_BYTES });
}

/**
 * Hash a capability set order-insensitively.  Sort the strings, join
 * with NUL, hash once with BLAKE2b.  The output is a 64-char hex
 * string; the input order doesn't matter.
 *
 * Exposed for callers that want to derive their own cache keys with
 * the same convention (e.g. test fixtures).
 */
export function capabilitySetHash(capabilities: readonly string[]): string {
  // Sort to make the function commutative on input order.  Defensive
  // copy because the caller's array might be a readonly proxy or
  // shared reference.
  const sorted = [...capabilities].sort();
  const joined = sorted.join("\0");
  return blake2bHex(encode(joined), { digestSize: CACHE_KEY_DIGEST_BYTES });
}

// ─── Internals ────────────────────────────────────────────────────────────

const ENCODER = new TextEncoder();
function encode(s: string): Uint8Array { return ENCODER.encode(s); }

function concat(parts: readonly Uint8Array[]): Uint8Array {
  let total = 0;
  for (const p of parts) total += p.byteLength;
  const out = new Uint8Array(total);
  let offset = 0;
  for (const p of parts) {
    out.set(p, offset);
    offset += p.byteLength;
  }
  return out;
}
