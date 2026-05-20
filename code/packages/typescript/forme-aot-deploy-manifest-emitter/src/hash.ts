/**
 * hash.ts — SHA-256 of a string, base64-encoded.  Same pattern
 * as `forme-aot-page-bundle-emitter`.
 *
 * Uses Node's built-in `node:crypto`.  No external dependency,
 * no network.
 *
 * @module hash
 */

import { createHash } from "node:crypto";

/**
 * Compute the base64-encoded SHA-256 digest of `s` (UTF-8
 * encoded).
 */
export function sha256Base64(s: string): string {
  return createHash("sha256").update(s, "utf8").digest("base64");
}

/**
 * UTF-8 byte length of `s`.  Same pattern as the page-bundle
 * emitter.
 */
export function utf8ByteLength(s: string): number {
  return new TextEncoder().encode(s).length;
}
