/**
 * hash.ts — SHA-256 of a string, base64-encoded.
 *
 * Uses Node's built-in `node:crypto`.  No external dependency,
 * no network, no allocator surprises.
 *
 * @module hash
 */

import { createHash } from "node:crypto";

/**
 * Compute the base64-encoded SHA-256 digest of `s` (UTF-8
 * encoded).  Standard base64 (with `+`, `/`, `=` padding).
 */
export function sha256Base64(s: string): string {
  return createHash("sha256").update(s, "utf8").digest("base64");
}

/**
 * Compute the UTF-8 byte length of `s`.  Uses TextEncoder
 * (built-in, no allocator) — `Buffer.byteLength` would also
 * work but we avoid Buffer for portability.
 */
export function utf8ByteLength(s: string): number {
  return new TextEncoder().encode(s).length;
}
