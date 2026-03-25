/**
 * @coding-adventures/uuid
 *
 * UUID v1/v3/v4/v5/v7 generation and parsing (RFC 4122 + RFC 9562)
 *
 * What Is a UUID?
 * ===============
 * A UUID (Universally Unique Identifier) is a 128-bit label used to identify
 * information without central coordination. Two different computers can each
 * generate a UUID and be overwhelmingly confident they generated different ones.
 *
 * The 128 bits are conventionally written in this hyphenated hex format:
 *
 *   xxxxxxxx-xxxx-Mxxx-Nxxx-xxxxxxxxxxxx
 *   └──────┘ └──┘ └──┘ └──┘ └──────────┘
 *    32 bits  16b  16b  16b   48 bits
 *      (8)   (4)  (4)  (4)    (12)   hex chars in each group
 *
 * The 'M' nibble encodes the version (1–7). The 'N' nibble's top bits encode
 * the variant (RFC 4122 uses 10xx, meaning the top two bits are 1 and 0).
 *
 * UUID Versions
 * =============
 * - v1: Time + MAC address. Encodes the current time as 100-nanosecond
 *       intervals since 15 October 1582 (the Gregorian epoch), plus the
 *       host's MAC address. The time makes it unique across moments; the
 *       MAC makes it unique across machines.
 *
 * - v3: Name-based, MD5. Hash a namespace UUID + a name string with MD5.
 *       Deterministic: same namespace + name always yields the same UUID.
 *       Good for reproducible IDs (e.g., stable identifier for a URL).
 *
 * - v4: Random. 122 bits of cryptographic randomness. The most commonly
 *       used version — simple, no coordination required.
 *
 * - v5: Name-based, SHA-1. Like v3 but uses SHA-1 (stronger hash).
 *       Preferred over v3 for new systems.
 *
 * - v7: Unix timestamp + random. Like v1 but uses millisecond-precision
 *       Unix time (not Gregorian), stored in a sortable big-endian layout.
 *       Useful for database primary keys that sort by creation time.
 *
 * Variant Field
 * =============
 * Byte 8 (the first byte of the fourth group) encodes the variant:
 *
 *   0xxx  (top bit 0)  → NCS backward compatibility (obsolete)
 *   10xx  (top 2 bits) → RFC 4122 / RFC 9562 (modern standard)
 *   110x  (top 3 bits) → Microsoft GUID (legacy COM)
 *   111x  (top 3 bits) → Reserved
 *
 * We always produce RFC 4122 variant (10xx): set the top 2 bits to 10 by:
 *   byte[8] = (byte[8] & 0x3F) | 0x80
 *   0x3F = 00111111 (clear top 2 bits)
 *   0x80 = 10000000 (set top bit to 1, second to 0)
 *
 * Internal Representation
 * =======================
 * We store all UUIDs as 16-byte Uint8Arrays (big-endian, most significant
 * byte first). This matches the standard UUID byte ordering defined by RFC 4122.
 */

import { sha1 } from "@coding-adventures/sha1";
import { md5 } from "@coding-adventures/md5";

export const VERSION = "0.1.0";

// ─── Error Class ─────────────────────────────────────────────────────────────

/**
 * Thrown when a string cannot be parsed as a UUID, or when an invalid value
 * is passed to the UUID constructor.
 *
 * Extends the built-in Error so callers can distinguish UUID errors from other
 * exceptions with `err instanceof UUIDError`.
 */
export class UUIDError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "UUIDError";
  }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/**
 * Convert a Uint8Array to a lowercase hex string, two characters per byte.
 *
 * Example: bytesToHex(new Uint8Array([0xAB, 0x0C])) → "ab0c"
 */
function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

/**
 * Parse a 32-character hex string (no hyphens) into 16 bytes.
 *
 * The input must be exactly 32 hex characters. Each pair of characters
 * becomes one byte: "6f" → 0x6F.
 */
function hexToBytes(hex: string): Uint8Array {
  if (hex.length !== 32) {
    throw new UUIDError(`Expected 32 hex characters, got ${hex.length}`);
  }
  const bytes = new Uint8Array(16);
  for (let i = 0; i < 16; i++) {
    const hi = parseInt(hex[i * 2], 16);
    const lo = parseInt(hex[i * 2 + 1], 16);
    if (isNaN(hi) || isNaN(lo)) {
      throw new UUIDError(`Invalid hex character at position ${i * 2}`);
    }
    bytes[i] = (hi << 4) | lo;
  }
  return bytes;
}

/**
 * Set the version nibble and variant bits in a 16-byte UUID raw buffer.
 *
 * Version encoding (byte 6, high nibble):
 *   byte[6] = (byte[6] & 0x0F) | (version << 4)
 *   0x0F = 00001111 — clears the high nibble, preserving the low 4 bits
 *   version << 4    — shifts the version number into the high nibble position
 *   Example: version=4 → (byte[6] & 0x0F) | 0x40
 *
 * Variant encoding (byte 8, top 2 bits = "10"):
 *   byte[8] = (byte[8] & 0x3F) | 0x80
 *   0x3F = 00111111 — clears the top 2 bits
 *   0x80 = 10000000 — sets the top bit to 1, second bit to 0
 */
function setVersionVariant(raw: Uint8Array, version: number): void {
  raw[6] = (raw[6] & 0x0F) | (version << 4);
  raw[8] = (raw[8] & 0x3F) | 0x80;
}

// ─── UUID Class ───────────────────────────────────────────────────────────────

/**
 * Immutable 128-bit UUID value.
 *
 * Internally stored as 16 bytes in RFC 4122 byte order (big-endian).
 * Constructors accept raw bytes, a hex/formatted string, or a BigInt.
 *
 * Usage:
 *   const id = v4();
 *   console.log(id.toString());   // "550e8400-e29b-41d4-a716-446655440000"
 *   console.log(id.version);      // 4
 *   console.log(id.variant);      // "rfc4122"
 */
export class UUID {
  private readonly _bytes: Uint8Array;

  /**
   * Construct a UUID from:
   *   - Uint8Array of exactly 16 bytes (copied, not referenced)
   *   - A string in any supported format (standard, compact, braced, URN)
   *   - A BigInt representing the 128-bit integer value
   *
   * Throws UUIDError for invalid inputs.
   */
  constructor(value: Uint8Array | string | bigint) {
    if (value instanceof Uint8Array) {
      // Accept exactly 16 bytes. Copy so the UUID is immutable from the outside.
      if (value.length !== 16) {
        throw new UUIDError(`UUID bytes must be exactly 16, got ${value.length}`);
      }
      this._bytes = new Uint8Array(value);
    } else if (typeof value === "string") {
      // Delegate to the parse() function for all string forms.
      this._bytes = parse(value)._bytes;
    } else if (typeof value === "bigint") {
      // A 128-bit BigInt, big-endian: high bits go into byte[0].
      if (value < BigInt(0) || value > BigInt("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF")) {
        throw new UUIDError("BigInt value out of 128-bit range");
      }
      const raw = new Uint8Array(16);
      let n = value;
      for (let i = 15; i >= 0; i--) {
        raw[i] = Number(n & BigInt(0xFF));
        n >>= BigInt(8);
      }
      this._bytes = raw;
    } else {
      throw new UUIDError("UUID constructor expects Uint8Array, string, or bigint");
    }
  }

  /**
   * The 16 raw bytes of this UUID (a copy; mutating the result is safe).
   *
   * Byte order follows RFC 4122 §4.1.2: big-endian, most significant byte first.
   */
  get bytes(): Uint8Array {
    return this._bytes.slice();
  }

  /**
   * The 128-bit integer value of this UUID as a BigInt.
   *
   * Big-endian: byte[0] is the most significant byte.
   * Useful for arithmetic comparisons and database storage.
   *
   * Example: NIL UUID → 0n
   *          MAX UUID → 2^128 - 1
   */
  get int(): bigint {
    let n = BigInt(0);
    for (let i = 0; i < 16; i++) {
      n = (n << BigInt(8)) | BigInt(this._bytes[i]);
    }
    return n;
  }

  /**
   * The version number (1–8, or 0 for NIL/unversioned UUIDs).
   *
   * Encoded in the high nibble of byte 6 (the 'M' nibble):
   *   version = byte[6] >> 4
   *
   * Version 0 means unversioned (NIL UUID, all zeros).
   * Version 0xF (15) appears in the MAX UUID (all ones).
   */
  get version(): number {
    return (this._bytes[6] >> 4) & 0x0F;
  }

  /**
   * The variant string describing which UUID specification applies.
   *
   * Encoded in the top bits of byte 8:
   *   bit pattern  → variant name
   *   0xxx         → "ncs" (NCS backward compatibility, top bit 0)
   *   10xx         → "rfc4122" (RFC 4122 / RFC 9562, top 2 bits = 10)
   *   110x         → "microsoft" (Microsoft GUID, top 3 bits = 110)
   *   111x         → "reserved" (future, top 3 bits = 111)
   *
   * All UUIDs generated by this library use "rfc4122".
   */
  get variant(): string {
    const b = this._bytes[8];
    if ((b & 0x80) === 0x00) return "ncs";        // 0xxx
    if ((b & 0xC0) === 0x80) return "rfc4122";    // 10xx
    if ((b & 0xE0) === 0xC0) return "microsoft";  // 110x
    return "reserved";                             // 111x
  }

  /**
   * True if this is the NIL UUID (128 bits, all zero).
   *
   * The NIL UUID is used as a sentinel value: "no UUID assigned yet."
   * RFC 4122 §4.1.7 defines it as 00000000-0000-0000-0000-000000000000.
   */
  get isNil(): boolean {
    return this._bytes.every((b) => b === 0);
  }

  /**
   * True if this is the MAX UUID (128 bits, all one).
   *
   * The MAX UUID is defined in RFC 9562 §5.10 as the complement of NIL:
   * ffffffff-ffff-ffff-ffff-ffffffffffff.
   * It can be used as a "maximum" sentinel, e.g., as an exclusive upper bound
   * in a range query.
   */
  get isMax(): boolean {
    return this._bytes.every((b) => b === 0xFF);
  }

  /**
   * Return the UUID as a standard 8-4-4-4-12 hyphenated lowercase hex string.
   *
   * Format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
   *
   * This is the canonical string form (RFC 4122 §3). Lowercase is preferred
   * because it's easier to read and compare (avoids case sensitivity issues).
   */
  toString(): string {
    const h = bytesToHex(this._bytes);
    // Insert hyphens at positions 8, 12, 16, 20 of the 32-char hex string.
    return `${h.slice(0, 8)}-${h.slice(8, 12)}-${h.slice(12, 16)}-${h.slice(16, 20)}-${h.slice(20)}`;
  }

  /**
   * True if this UUID has the same 128-bit value as `other`.
   *
   * Byte-by-byte comparison of the internal Uint8Arrays.
   */
  equals(other: UUID): boolean {
    for (let i = 0; i < 16; i++) {
      if (this._bytes[i] !== other._bytes[i]) return false;
    }
    return true;
  }

  /**
   * Compare this UUID to `other` lexicographically by byte value.
   *
   * Returns:
   *   -1 if this < other
   *    0 if this === other
   *    1 if this > other
   *
   * Comparison is byte-by-byte from the most significant byte (index 0).
   * This ordering is consistent with the integer ordering of the 128-bit value.
   *
   * For v7 UUIDs, this ordering also approximates chronological ordering
   * because the 48-bit millisecond timestamp occupies the first 6 bytes.
   */
  compareTo(other: UUID): number {
    for (let i = 0; i < 16; i++) {
      if (this._bytes[i] < other._bytes[i]) return -1;
      if (this._bytes[i] > other._bytes[i]) return 1;
    }
    return 0;
  }
}

// ─── Parsing ─────────────────────────────────────────────────────────────────

/**
 * Parse a UUID string in any of these four formats, returning a UUID object.
 *
 * Supported formats:
 *   Standard:  550e8400-e29b-41d4-a716-446655440000
 *   Uppercase: 550E8400-E29B-41D4-A716-446655440000  (case-insensitive)
 *   Compact:   550e8400e29b41d4a716446655440000       (32 hex chars, no hyphens)
 *   Braced:    {550e8400-e29b-41d4-a716-446655440000}
 *   URN:       urn:uuid:550e8400-e29b-41d4-a716-446655440000
 *
 * Throws UUIDError for any string that doesn't match a known format.
 *
 * Algorithm:
 *   1. Strip known prefixes (urn:uuid:, { }) and then normalize to 32 hex chars.
 *   2. Validate exactly 32 hex characters remain.
 *   3. Convert hex pairs to 16 bytes.
 */
export function parse(s: string): UUID {
  let hex: string;
  const trimmed = s.trim();

  // URN form: urn:uuid:xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
  if (trimmed.toLowerCase().startsWith("urn:uuid:")) {
    hex = trimmed.slice(9).toLowerCase().replace(/-/g, "");
  }
  // Braced form: {xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx}
  else if (trimmed.startsWith("{") && trimmed.endsWith("}")) {
    hex = trimmed.slice(1, -1).toLowerCase().replace(/-/g, "");
  }
  // Standard or compact form
  else {
    hex = trimmed.toLowerCase().replace(/-/g, "");
  }

  // After stripping decoration, we must have exactly 32 hex characters.
  if (!/^[0-9a-f]{32}$/.test(hex)) {
    throw new UUIDError(`Invalid UUID string: "${s}"`);
  }

  return new UUID(hexToBytes(hex));
}

/**
 * Return true if `s` is a valid UUID string in any supported format.
 *
 * This is the non-throwing counterpart to parse(). Useful for validation
 * without try/catch boilerplate:
 *
 *   if (isValid(userInput)) { ... }
 */
export function isValid(s: string): boolean {
  try {
    parse(s);
    return true;
  } catch {
    return false;
  }
}

// ─── Well-Known Namespace UUIDs ────────────────────────────────────────────────
//
// RFC 4122 §C defines four "well-known" namespaces for name-based UUIDs (v3/v5).
// These are fixed UUIDs that serve as seeds when hashing a name — using
// different namespaces for the same name produces different UUIDs, preventing
// collisions between, say, DNS names and URLs that happen to have the same text.
//
// NAMESPACE_DNS:  for domain names                (6ba7b810-...)
// NAMESPACE_URL:  for URLs                        (6ba7b811-...)
// NAMESPACE_OID:  for ISO OIDs                    (6ba7b812-...)
// NAMESPACE_X500: for X.500 Distinguished Names   (6ba7b814-...)
//
// Note: the last digit of the first group increments: 10, 11, 12, 14.

/** Namespace for DNS domain names. Use with v5() or v3(). */
export const NAMESPACE_DNS: UUID = parse("6ba7b810-9dad-11d1-80b4-00c04fd430c8");

/** Namespace for URLs. Use with v5() or v3(). */
export const NAMESPACE_URL: UUID = parse("6ba7b811-9dad-11d1-80b4-00c04fd430c8");

/** Namespace for ISO Object Identifiers (OIDs). Use with v5() or v3(). */
export const NAMESPACE_OID: UUID = parse("6ba7b812-9dad-11d1-80b4-00c04fd430c8");

/** Namespace for X.500 Distinguished Names. Use with v5() or v3(). */
export const NAMESPACE_X500: UUID = parse("6ba7b814-9dad-11d1-80b4-00c04fd430c8");

// ─── NIL and MAX UUIDs ────────────────────────────────────────────────────────

/**
 * The NIL UUID: 00000000-0000-0000-0000-000000000000
 *
 * All 128 bits are zero. Used as a sentinel for "no UUID" or "unset UUID."
 * Analogous to null/None for UUID fields.
 */
export const NIL: UUID = new UUID(new Uint8Array(16));

/**
 * The MAX UUID: ffffffff-ffff-ffff-ffff-ffffffffffff
 *
 * All 128 bits are one. Defined in RFC 9562. Can be used as a sentinel for
 * "maximum UUID" or as an exclusive upper bound in range scans.
 */
export const MAX: UUID = new UUID(new Uint8Array(16).fill(0xFF));

// ─── UUID Generation Functions ────────────────────────────────────────────────

/**
 * Generate a Version 4 (random) UUID.
 *
 * Construction:
 *   1. Fill 16 bytes with cryptographic randomness.
 *   2. Set the version nibble (byte 6, high nibble) to 4.
 *   3. Set the variant bits (byte 8, top 2 bits) to 10.
 *
 * With 122 bits of randomness (128 - 4 version bits - 2 variant bits), the
 * probability of a collision among 1 trillion UUIDs is roughly 1 in 10^18.
 * For most practical purposes, treat v4 UUIDs as unique.
 *
 * Uses: crypto.getRandomValues() — the Web Crypto API available in browsers
 * and Node.js 15+. This is a CSPRNG (cryptographically secure pseudo-random
 * number generator), unlike Math.random() which is not secure.
 */
export function v4(): UUID {
  const raw = crypto.getRandomValues(new Uint8Array(16));
  setVersionVariant(raw, 4);
  return new UUID(raw);
}

/**
 * Generate a Version 5 (SHA-1, name-based) UUID.
 *
 * Algorithm (RFC 4122 §4.3):
 *   1. Concatenate the namespace UUID bytes (16) + the UTF-8 encoded name.
 *   2. Hash with SHA-1 → 20 bytes.
 *   3. Take the first 16 bytes as the UUID raw bytes.
 *   4. Set version = 5 and RFC 4122 variant bits.
 *
 * Deterministic: the same namespace + name always yields the same UUID.
 * This is intentional — it lets you generate stable, reproducible IDs from
 * well-known inputs without needing a central registry.
 *
 * v5 is preferred over v3 because SHA-1 is stronger than MD5 (though neither
 * is recommended for security purposes — this is about reproducibility, not
 * cryptographic strength).
 *
 * RFC 4122 test vector:
 *   v5(NAMESPACE_DNS, "python.org") = "886313e1-3b8a-5372-9b90-0c9aee199e5d"
 *
 * @param namespace  A namespace UUID (use NAMESPACE_DNS, NAMESPACE_URL, etc.)
 * @param name       The name string (UTF-8 encoded before hashing)
 */
export function v5(namespace: UUID, name: string): UUID {
  const enc = new TextEncoder();
  // Concatenate namespace bytes + name bytes into a single buffer.
  // The spread operator flattens Uint8Arrays into a plain array of numbers,
  // then we create a new Uint8Array from that flat array.
  const data = new Uint8Array([...namespace.bytes, ...enc.encode(name)]);
  const digest = sha1(data); // 20 bytes
  // We only need the first 16 bytes; the remaining 4 are discarded.
  const raw = digest.slice(0, 16);
  setVersionVariant(raw, 5);
  return new UUID(raw);
}

/**
 * Generate a Version 3 (MD5, name-based) UUID.
 *
 * Algorithm (RFC 4122 §4.3):
 *   1. Concatenate the namespace UUID bytes (16) + the UTF-8 encoded name.
 *   2. Hash with MD5 → 16 bytes (exactly the UUID size).
 *   3. Set version = 3 and RFC 4122 variant bits.
 *
 * Identical to v5 except MD5 is used instead of SHA-1. Prefer v5 for new
 * systems; v3 exists for backward compatibility with existing deployments.
 *
 * RFC 4122 test vector:
 *   v3(NAMESPACE_DNS, "python.org") = "6fa459ea-ee8a-3ca4-894e-db77e160355e"
 *
 * @param namespace  A namespace UUID (use NAMESPACE_DNS, NAMESPACE_URL, etc.)
 * @param name       The name string (UTF-8 encoded before hashing)
 */
export function v3(namespace: UUID, name: string): UUID {
  const enc = new TextEncoder();
  const data = new Uint8Array([...namespace.bytes, ...enc.encode(name)]);
  const raw = md5(data); // 16 bytes — MD5 output exactly fills a UUID
  setVersionVariant(raw, 3);
  return new UUID(raw);
}

/**
 * Generate a Version 1 (time + MAC address) UUID.
 *
 * RFC 4122 §4.2 specifies v1 as:
 *   - 60-bit timestamp: 100-nanosecond intervals since 15 October 1582
 *   - 14-bit clock sequence: random, to handle rapid calls or clock resets
 *   - 48-bit node ID: ideally the MAC address; we use random bytes here
 *
 * Gregorian calendar offset:
 *   The UUID epoch (15 Oct 1582) is 12219292800 seconds before Unix epoch
 *   (1 Jan 1970). In 100-ns intervals: 122192928000000000 intervals.
 *
 * Timestamp layout (60 bits split across three fields):
 *   time_low  (32 bits): bits 0–31 of the timestamp (least significant)
 *   time_mid  (16 bits): bits 32–47 of the timestamp
 *   time_hi   (12 bits): bits 48–59 of the timestamp (most significant)
 *
 * Byte layout of the 16-byte UUID:
 *   [0..3]   time_low (32 bits, big-endian)
 *   [4..5]   time_mid (16 bits, big-endian)
 *   [6..7]   time_hi_version (16 bits, big-endian; high nibble = version=1)
 *   [8]      clock_seq_hi_variant (8 bits; top 2 bits = variant)
 *   [9]      clock_seq_low (8 bits)
 *   [10..15] node (48 bits = 6 bytes; we use random bytes)
 *
 * Note: We use random bytes for the node ID rather than the MAC address.
 * The RFC permits this (§4.5) and avoids exposing system hardware identifiers.
 * The multicast bit (LSB of first node byte) is set to 1 to signal that
 * this is not a real MAC address (RFC 4122 §4.5).
 */
export function v1(): UUID {
  // Convert current time to 100-nanosecond intervals since Gregorian epoch.
  // BigInt arithmetic is needed because the timestamp exceeds 2^53 (JavaScript's
  // safe integer limit for floating-point). We represent Date.now() in ms, then:
  //   ms * 1_000_000 ns/ms / 100 = ms * 10_000 units of 100ns
  // Plus the 122192928000000000 offset from 1582 to 1970.
  const gregorianOffset = BigInt("122192928000000000");
  const t100ns = BigInt(Date.now()) * BigInt(10_000) + gregorianOffset;

  // Mask out the individual timestamp fields.
  // The 60-bit timestamp is distributed across three UUID fields:
  //   time_low:  bits [31..0]   — lower 32 bits of t100ns
  //   time_mid:  bits [47..32]  — next 16 bits
  //   time_hi:   bits [59..48]  — upper 12 bits (the rest of the 60-bit value)
  const timeLow = Number(t100ns & BigInt(0xFFFFFFFF));        // bits 0–31
  const timeMid = Number((t100ns >> BigInt(32)) & BigInt(0xFFFF)); // bits 32–47
  const timeHi = Number((t100ns >> BigInt(48)) & BigInt(0x0FFF));  // bits 48–59

  // 14-bit clock sequence: random to avoid collisions on rapid generation.
  const clockSeq = crypto.getRandomValues(new Uint8Array(2));

  // 48-bit node: 6 random bytes with the multicast bit set (LSB of byte[0]).
  // Setting the multicast bit signals "not a real hardware MAC address."
  const node = crypto.getRandomValues(new Uint8Array(6));
  node[0] |= 0x01; // set multicast bit

  // Assemble the 16-byte UUID.
  const raw = new Uint8Array(16);
  const view = new DataView(raw.buffer);

  // Bytes 0–3: time_low (32-bit big-endian)
  view.setUint32(0, timeLow, false);
  // Bytes 4–5: time_mid (16-bit big-endian)
  view.setUint16(4, timeMid, false);
  // Bytes 6–7: time_hi_and_version (16-bit big-endian, version in high nibble)
  view.setUint16(6, timeHi, false);
  // Bytes 8–9: clock_seq (version/variant bits set by setVersionVariant)
  raw[8] = clockSeq[0];
  raw[9] = clockSeq[1];
  // Bytes 10–15: node
  raw.set(node, 10);

  setVersionVariant(raw, 1);
  return new UUID(raw);
}

/**
 * Generate a Version 7 (Unix timestamp + random) UUID.
 *
 * RFC 9562 §5.7 defines v7 as a monotonically increasing time-ordered UUID.
 * Unlike v1, the timestamp is the standard Unix epoch in milliseconds — the
 * same unit as Date.now() — making it easy to extract without special offsets.
 *
 * Byte layout:
 *   [0..5]   48-bit Unix timestamp in milliseconds (big-endian)
 *              → enables lexicographic sort = chronological sort
 *   [6]      version nibble (0x7x) + 4 bits of random
 *   [7]      8 bits of random
 *   [8]      variant bits (10xx) + 6 bits of random
 *   [9..15]  55 bits of random
 *
 * The 48-bit timestamp safely represents milliseconds until the year 10889
 * (2^48 ms = ~8925 years from epoch). JavaScript's Date.now() returns a
 * 64-bit float that is exact up to 2^53 — well within the 48-bit range,
 * so no BigInt arithmetic is required here.
 *
 * Because the timestamp occupies the most significant bytes in big-endian
 * order, lexicographic comparison (byte[0] first) is equivalent to
 * chronological comparison. This makes v7 UUIDs ideal for database primary
 * keys: they sort naturally without a separate `created_at` column.
 *
 * Construction:
 *   1. Get tMs = Date.now() (milliseconds).
 *   2. Write the 48-bit timestamp as two big-endian chunks:
 *        bytes 0–3: upper 32 bits of tMs  (tMs >> 16)
 *        bytes 4–5: lower 16 bits of tMs  (tMs & 0xFFFF)
 *   3. Fill remaining 10 bytes with random data.
 *   4. Set version=7 and variant bits.
 */
export function v7(): UUID {
  const tMs = Date.now(); // Unix milliseconds — exact as a JS number for < 2^53

  // Generate 10 random bytes for the non-timestamp fields.
  const rand = crypto.getRandomValues(new Uint8Array(10));

  const raw = new Uint8Array(16);
  const view = new DataView(raw.buffer);

  // Write 48-bit timestamp big-endian across bytes 0–5.
  // Split into: upper 32 bits (bytes 0–3) and lower 16 bits (bytes 4–5).
  //
  // tMs fits in 48 bits (max ~281 trillion), so we can use regular numbers.
  // Math.floor(tMs / 65536) = tMs >> 16 for the upper part (shift by 16 bits).
  // The >>> 0 forces the result to an unsigned 32-bit integer for setUint32.
  const tsHi32 = Math.floor(tMs / 0x10000) >>> 0; // bits 47..16 of tMs
  const tsLo16 = tMs & 0xFFFF;                     // bits 15..0  of tMs

  view.setUint32(0, tsHi32, false); // big-endian
  view.setUint16(4, tsLo16, false); // big-endian

  // Bytes 6–15: version nibble + 76 bits of random
  // We overwrite bytes 6–15 with random data, then setVersionVariant will
  // correctly stamp the version nibble into byte 6 and variant into byte 8.
  raw[6] = rand[0];
  raw[7] = rand[1];
  raw[8] = rand[2];
  raw.set(rand.slice(3, 10), 9);

  setVersionVariant(raw, 7);
  return new UUID(raw);
}
