/**
 * LogicalId — UUIDv7 generation and validation.
 *
 * UUIDv7 (RFC 9562) carries 48 bits of millisecond Unix timestamp in
 * its high bits, then a 4-bit version field (0x7), 12 bits of random
 * "rand_a", 2 bits of variant (0b10), and 62 bits of random "rand_b".
 * Total 128 bits / 16 bytes / 36 chars including hyphens:
 *
 *     XXXXXXXX-XXXX-7XXX-yXXX-XXXXXXXXXXXX
 *      time(48)  ver(4)+ra(12)  v(2)+rb(62)
 *
 * Why UUIDv7 and not UUIDv4?  Two things:
 *
 *   1. **Lexicographic sort order matches creation order.**  When we
 *      eventually persist Forme content with a per-document id,
 *      time-prefixed ids let listings be naturally chronological.
 *      v4 has no such property.
 *
 *   2. **Collision resistance is identical.**  The random suffix is
 *      74 bits — at 1000 ids/second that's ~580 years before a 50%
 *      birthday-collision chance.  More than enough for any single
 *      Forme installation.
 *
 * === Source of randomness ===
 *
 * We use `globalThis.crypto.getRandomValues`, which is the standardised
 * cryptographically-secure RNG available in Node 19+, browsers, Deno,
 * Bun, and Cloudflare Workers.  Older runtimes are not supported; the
 * function throws a clear error if `crypto` is missing.
 *
 * === Source of time ===
 *
 * We use `Date.now()` directly.  Stages running under a deterministic
 * `ctx.time` should *not* call this function — they should let the
 * orchestrator pre-mint logical ids during a deterministic build, or
 * read identities from `id.json` adjacent to the source file (FM01 §7.2).
 * `generateLogicalId` is intentionally not a "stage runtime" API; it's
 * a utility for sources, the editor, and tooling.
 */

import type { LogicalId } from "@coding-adventures/forme-types";

/**
 * Generate a fresh UUIDv7 as a `LogicalId`.
 *
 * @throws Error if `globalThis.crypto.getRandomValues` is not available.
 */
export function generateLogicalId(): LogicalId {
  return formatV7(Date.now(), randomBytes(10));
}

/**
 * Generate a UUIDv7 with an externally-supplied timestamp.  Useful for
 * deterministic / reproducible builds where the timestamp comes from a
 * fixed clock and the random bits come from a seeded PRNG.
 *
 * @param unixMillis  Milliseconds since the Unix epoch, in [0, 2^48).
 * @param randomTail  10 bytes of randomness.  The version and variant
 *                    nibbles will be overwritten with their fixed
 *                    values; the rest of the bytes are used verbatim.
 */
export function buildLogicalIdFrom(
  unixMillis: number,
  randomTail: Uint8Array,
): LogicalId {
  if (!Number.isInteger(unixMillis) || unixMillis < 0 || unixMillis > 0xFFFFFFFFFFFF) {
    throw new RangeError(
      `buildLogicalIdFrom: unixMillis must be an integer in [0, 2^48), got ${unixMillis}`,
    );
  }
  if (randomTail.length !== 10) {
    throw new RangeError(
      `buildLogicalIdFrom: randomTail must be exactly 10 bytes, got ${randomTail.length}`,
    );
  }
  return formatV7(unixMillis, randomTail);
}

/**
 * Predicate: does this string look like a UUIDv7?  Loose check — verifies
 * the canonical 8-4-4-4-12 grouping, lower-case hex, version `7` nibble
 * at position 14, and variant `8|9|a|b` nibble at position 19.  Useful
 * for parser-level input validation.
 */
export function isLogicalIdShape(value: string): boolean {
  return UUID_V7_REGEX.test(value);
}

const UUID_V7_REGEX =
  /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;

// ─── Internals ────────────────────────────────────────────────────────────

/**
 * Build a UUIDv7 string from a 48-bit timestamp and 10 random bytes.
 * The version and variant bits are stamped into the proper positions
 * regardless of the input — callers don't need to mask them.
 */
function formatV7(unixMillis: number, random: Uint8Array): LogicalId {
  // Layout (16 bytes):
  //   bytes 0–5  : 48-bit big-endian millisecond timestamp
  //   bytes 6–7  : version(4) + rand_a(12)
  //   bytes 8–15 : variant(2) + rand_b(62)
  const bytes = new Uint8Array(16);

  // Timestamp.  JavaScript number is safe up to 2^53; we only need 48 bits.
  // Use Math.floor on the high half so a fractional ms can't sneak in.
  const tsHi = Math.floor(unixMillis / 0x100000000);
  const tsLo = unixMillis >>> 0;
  bytes[0] = (tsHi >>> 8) & 0xff;
  bytes[1] = tsHi & 0xff;
  bytes[2] = (tsLo >>> 24) & 0xff;
  bytes[3] = (tsLo >>> 16) & 0xff;
  bytes[4] = (tsLo >>> 8) & 0xff;
  bytes[5] = tsLo & 0xff;

  // Random body.
  for (let i = 0; i < 10; i++) bytes[6 + i] = random[i]!;

  // Stamp version (high nibble of byte 6 = 0x7).
  bytes[6] = (bytes[6]! & 0x0f) | 0x70;
  // Stamp variant (top two bits of byte 8 = 0b10).
  bytes[8] = (bytes[8]! & 0x3f) | 0x80;

  return formatHyphens(bytes) as LogicalId;
}

/** Format a 16-byte UUID as the canonical 8-4-4-4-12 lower-case-hex string. */
function formatHyphens(b: Uint8Array): string {
  const h = (n: number) => n.toString(16).padStart(2, "0");
  return (
    h(b[0]!) + h(b[1]!) + h(b[2]!) + h(b[3]!) + "-" +
    h(b[4]!) + h(b[5]!) + "-" +
    h(b[6]!) + h(b[7]!) + "-" +
    h(b[8]!) + h(b[9]!) + "-" +
    h(b[10]!) + h(b[11]!) + h(b[12]!) + h(b[13]!) + h(b[14]!) + h(b[15]!)
  );
}

/** Pull `n` cryptographically-strong random bytes from the platform RNG. */
function randomBytes(n: number): Uint8Array {
  const c = globalThis.crypto;
  if (!c || typeof c.getRandomValues !== "function") {
    throw new Error(
      "forme-identity: globalThis.crypto.getRandomValues is not available. " +
      "Forme requires Node 19+, a modern browser, Deno, Bun, or a Worker runtime.",
    );
  }
  const buf = new Uint8Array(n);
  c.getRandomValues(buf);
  return buf;
}
