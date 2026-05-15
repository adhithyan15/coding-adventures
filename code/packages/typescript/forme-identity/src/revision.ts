/**
 * Revision identity — `RevisionId` generation via BLAKE2b over the
 * canonical-JSON serialisation of a JsonValue.
 *
 * See FM01 §7.3.  The format is `<algo>:<hex>`; v0 uses `blake2b` because
 * the monorepo has a from-scratch BLAKE2b but no BLAKE3.  The prefix
 * makes the format forward-compatible — a future migration to BLAKE3
 * just changes the prefix.
 *
 * Why hash the canonical JSON instead of the raw value?  Because two
 * logically equal values can have different in-memory representations
 * (key order, number formatting), and we need byte-equality to imply
 * RevisionId-equality.  Canonical JSON is the bridge.
 *
 * Why a 32-byte digest?  256 bits is well over the birthday-collision
 * resistance threshold for any plausible Forme corpus (a billion
 * documents has collision probability < 2^-160).  Shorter digests
 * (16 bytes) would still be cryptographically safe but make manual
 * inspection harder; longer digests (64 bytes) waste bytes in cache
 * keys.  32 is the goldilocks size matching SHA-256 / BLAKE3.
 */

import { blake2bHex } from "@coding-adventures/blake2b";
import type { JsonValue, RevisionId } from "@coding-adventures/forme-types";
import { canonicalJson } from "./canonical-json.js";

/** Algorithm prefix used in v0.  Stored in the RevisionId for forward compatibility. */
export const REVISION_ALGORITHM = "blake2b" as const;

/** Length of the digest in bytes.  256-bit BLAKE2b. */
export const REVISION_DIGEST_BYTES = 32;

/**
 * Compute a deterministic content-addressed `RevisionId` for any
 * JsonValue.  Two logically equal values produce the same id; any
 * change in the value (a single bit anywhere in the canonical form)
 * produces a different id with overwhelming probability.
 */
export function computeRevisionId(payload: JsonValue): RevisionId {
  const canonical = canonicalJson(payload);
  const bytes = new TextEncoder().encode(canonical);
  const hex = blake2bHex(bytes, { digestSize: REVISION_DIGEST_BYTES });
  return `${REVISION_ALGORITHM}:${hex}` as RevisionId;
}

/**
 * Predicate: does the given string match the RevisionId format?  Useful
 * for input validation at the source-code boundary.
 */
export function isRevisionIdShape(value: string): boolean {
  // <algo>:<hex>; algo is lower-case alphanumerics, hex is 0-9 a-f.
  // Length check rejects truncated copies and other-algorithm digests.
  const expectedHexChars = REVISION_DIGEST_BYTES * 2;
  // We accept any algorithm prefix here (forward compatibility); only
  // require the suffix to look like the expected length when the
  // algorithm matches our default.  Stricter callers can compose
  // additional checks themselves.
  const colon = value.indexOf(":");
  if (colon <= 0 || colon === value.length - 1) return false;
  const algo = value.slice(0, colon);
  const hex  = value.slice(colon + 1);
  if (!/^[a-z0-9]+$/.test(algo)) return false;
  if (!/^[0-9a-f]+$/.test(hex))  return false;
  if (algo === REVISION_ALGORITHM && hex.length !== expectedHexChars) {
    return false;
  }
  return true;
}
