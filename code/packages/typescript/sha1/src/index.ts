/**
 * @coding-adventures/sha1
 *
 * SHA-1 cryptographic hash function (FIPS 180-4) implemented from scratch.
 *
 * What Is SHA-1?
 * ==============
 * SHA-1 (Secure Hash Algorithm 1) takes any sequence of bytes and produces a
 * fixed-size 20-byte (160-bit) "fingerprint" called a digest. The same input
 * always produces the same digest. Change even one bit of input and the digest
 * changes completely — the "avalanche effect". You cannot reverse a digest back
 * to the original input.
 *
 * We implement SHA-1 from scratch rather than using the Web Crypto API so that
 * the algorithm is transparent: every step is visible and explained.
 *
 * The Big Picture: Merkle-Damgård Construction
 * =============================================
 * SHA-1 processes data in 512-bit (64-byte) blocks, like a production line that
 * handles exactly one box at a time:
 *
 *   message ──► [pad] ──► block₀ ──► block₁ ──► ... ──► 20-byte digest
 *                              │           │
 *                      [H₀..H₄]──►compress──►compress──►...
 *
 * The "state" is five 32-bit words (H₀..H₄). For each block, 80 rounds of bit
 * mixing fold the block into the state. The final state is the digest.
 *
 * Analogy: a blender. Start with a base liquid (the initial constants). Add
 * ingredients one chunk at a time (message blocks). Each blend mixes the new
 * ingredient with everything before it. You cannot un-blend.
 *
 * JavaScript 32-bit Integer Caveat
 * =================================
 * JavaScript's bitwise operators (|, &, ^, ~, <<, >>) always return *signed*
 * 32-bit integers. SHA-1 needs *unsigned* 32-bit integers. The fix: append
 * `>>> 0` after any bitwise operation to coerce the result to an unsigned
 * 32-bit integer. Example:
 *
 *   (-1 | 0) === -1            // signed — wrong for SHA-1
 *   (-1 | 0) >>> 0 === 4294967295  // unsigned — correct
 *
 * All additions also need `>>> 0` because `a + b` can exceed 2^32 and we want
 * the lower 32 bits treated as unsigned.
 *
 * FIPS 180-4 Test Vectors
 * =======================
 *   sha1(new TextEncoder().encode(""))    = da39a3ee5e6b4b0d3255bfef95601890afd80709
 *   sha1(new TextEncoder().encode("abc")) = a9993e364706816aba3e25717850c26c9cd0d89d
 */

export const VERSION = "0.1.0";

// ─── Initialization Constants ────────────────────────────────────────────────
//
// SHA-1 starts with these five 32-bit words as its initial state. They are
// "nothing up my sleeve" numbers — chosen to have an obvious counting pattern
// (01234567, 89ABCDEF, ... reversed in byte pairs) that proves no mathematical
// backdoor is hidden in the choice.
//
//   H₀ = 0x67452301 → bytes 67 45 23 01 → reverse: 01 23 45 67
//   H₁ = 0xEFCDAB89 → bytes EF CD AB 89 → reverse: 89 AB CD EF
//
const INIT: [number, number, number, number, number] = [
  0x67452301, // H₀
  0xefcdab89, // H₁
  0x98badcfe, // H₂
  0x10325476, // H₃
  0xc3d2e1f0, // H₄
];

// Round constants — one per 20-round stage, derived from square roots.
//   K₀ = floor(sqrt(2)  × 2^30) = 0x5A827999  (rounds 0–19)
//   K₁ = floor(sqrt(3)  × 2^30) = 0x6ED9EBA1  (rounds 20–39)
//   K₂ = floor(sqrt(5)  × 2^30) = 0x8F1BBCDC  (rounds 40–59)
//   K₃ = floor(sqrt(10) × 2^30) = 0xCA62C1D6  (rounds 60–79)
//
// Using irrational numbers (square roots) guarantees no special algebraic
// structure — they are the "most random" numbers we can choose.
const K: [number, number, number, number] = [
  0x5a827999, // rounds 0–19
  0x6ed9eba1, // rounds 20–39
  0x8f1bbcdc, // rounds 40–59
  0xca62c1d6, // rounds 60–79
];

// ─── Helper: Circular Left Shift ─────────────────────────────────────────────
//
// rotl(n, x) rotates x left by n bit positions within a 32-bit word. Bits that
// "fall off" the left end reappear on the right — unlike <<, which discards them.
//
// Example: n=2, x = 0b01101001
//   Regular:  01101001 << 2 = 10100100  (the leading 01 is gone)
//   Circular: 01101001 ROTL 2 = 10100110  (the leading 01 wraps around)
//
// Implementation: (x << n) | (x >> (32 - n))
// The left half shifts up; the right half fills in what fell off the top.
// `>>> 0` converts the signed JS result to unsigned 32-bit.
function rotl(n: number, x: number): number {
  return ((x << n) | (x >>> (32 - n))) >>> 0;
}

// ─── Padding ─────────────────────────────────────────────────────────────────
//
// The compression function needs exactly 64-byte (512-bit) blocks. Padding
// extends the message to a multiple of 64 bytes, per FIPS 180-4 §5.1.1:
//
//   1. Append 0x80 (the '1' bit followed by seven '0' bits).
//   2. Append 0x00 bytes until length ≡ 56 (mod 64).
//   3. Append the original bit length as a 64-bit big-endian integer.
//
// Example — "abc" (3 bytes = 24 bits):
//   61 62 63 80 [52 zero bytes] 00 00 00 00 00 00 00 18
//                                                   ^^ 24 in hex
//
// Why 56 mod 64? We need 8 bytes for the length field, and 56 + 8 = 64, which
// fills exactly one block. If adding 0x80 would push us past byte 56, we need
// an extra block of pure padding.
//
// JavaScript 64-bit length note:
// JS numbers are 64-bit floats, so they safely represent integers up to 2^53.
// For the length field, we split the 64-bit value into two 32-bit halves:
//   high = Math.floor(bitLen / 2^32)  — upper 32 bits
//   low  = bitLen >>> 0               — lower 32 bits
// For messages under 536 MB (2^29 bytes), high === 0 and low = byteLen * 8.
function pad(data: Uint8Array): Uint8Array {
  const byteLen = data.length;
  // bitLen as two 32-bit halves (covers messages up to 2^53 bytes)
  const bitLenHigh = Math.floor((byteLen * 8) / 0x100000000) >>> 0;
  const bitLenLow = (byteLen * 8) >>> 0;

  // How many zero bytes do we need after 0x80?
  // We need: (byteLen + 1 + zeroes) % 64 === 56
  const zeroes = ((56 - (byteLen + 1)) % 64 + 64) % 64;
  const padded = new Uint8Array(byteLen + 1 + zeroes + 8);

  padded.set(data, 0);
  padded[byteLen] = 0x80; // the mandatory '1' bit

  // Zero bytes are already 0 from Uint8Array initialization

  // Append 64-bit big-endian length (8 bytes at the end)
  const view = new DataView(padded.buffer);
  view.setUint32(byteLen + 1 + zeroes, bitLenHigh, false); // big-endian
  view.setUint32(byteLen + 1 + zeroes + 4, bitLenLow, false); // big-endian

  return padded;
}

// ─── Message Schedule ────────────────────────────────────────────────────────
//
// Each 64-byte block is parsed as 16 big-endian 32-bit words (W[0..15]), then
// expanded to 80 words using this recurrence:
//
//   W[i] = ROTL(1, W[i-3] XOR W[i-8] XOR W[i-14] XOR W[i-16])  for i ≥ 16
//
// Why expand from 16 to 80 words? More words → more mixing → better avalanche.
// A single bit flip in the input block changes W[i-3/8/14/16] at different
// offsets, so the ripple spreads through the entire schedule. By round 80,
// every output word is influenced by every input bit.
//
// DataView.getUint32(offset, false) reads a big-endian 32-bit word.
// SHA-1 is big-endian throughout (most significant byte first).
function schedule(block: Uint8Array): Uint32Array {
  const W = new Uint32Array(80);
  const view = new DataView(block.buffer, block.byteOffset, block.byteLength);
  for (let i = 0; i < 16; i++) {
    W[i] = view.getUint32(i * 4, false); // false = big-endian
  }
  for (let i = 16; i < 80; i++) {
    W[i] = rotl(1, (W[i - 3] ^ W[i - 8] ^ W[i - 14] ^ W[i - 16]) >>> 0);
  }
  return W;
}

// ─── Compression Function ────────────────────────────────────────────────────
//
// 80 rounds of mixing fold one 64-byte block into the five-word state.
//
// Four stages of 20 rounds each, using a different auxiliary function per stage:
//
//   Stage  Rounds  f(B, C, D)                    Purpose
//   ─────  ──────  ──────────────────────────    ────────────────
//     1    0–19    (B & C) | (~B & D)            Selector / mux
//     2    20–39   B ^ C ^ D                     Parity
//     3    40–59   (B&C) | (B&D) | (C&D)         Majority vote
//     4    60–79   B ^ C ^ D                     Parity again
//
// Selector (rounds 0–19): if bit of B is 1, choose C; if 0, choose D.
//   B=1 → (1 & C) | (0 & D) = C
//   B=0 → (0 & C) | (1 & D) = D
//
// Parity (rounds 20–39, 60–79): XOR of all three. Result is 1 when an odd
// number of the three input bits are 1 — the "parity" of a set of bits.
//
// Majority (rounds 40–59): result is 1 if at least 2 of the 3 input bits are 1.
//   (B&C): 1 only when B=1 AND C=1
//   (B&D): 1 only when B=1 AND D=1
//   (C&D): 1 only when C=1 AND D=1
//   OR them: any two-out-of-three combination returns 1.
//
// Each round:
//   temp = ROTL(5, a) + f(b,c,d) + e + K + W[t]   (mod 2^32)
//   shift: e=d, d=c, c=ROTL(30,b), b=a, a=temp
//
// Davies-Meyer feed-forward: after all 80 rounds, add the compressed output
// back to the original state. This makes the function non-invertible even if
// you could reverse all 80 rounds — you'd still need to subtract the input
// state that you don't have.
function compress(
  state: [number, number, number, number, number],
  block: Uint8Array,
): [number, number, number, number, number] {
  const W = schedule(block);
  const [h0, h1, h2, h3, h4] = state;
  let a = h0,
    b = h1,
    c = h2,
    d = h3,
    e = h4;

  for (let t = 0; t < 80; t++) {
    let f: number, k: number;
    if (t < 20) {
      // Selector: if b=1 output c, if b=0 output d
      f = ((b & c) | (~b & d)) >>> 0;
      k = K[0];
    } else if (t < 40) {
      // Parity: 1 if an odd number of inputs are 1
      f = (b ^ c ^ d) >>> 0;
      k = K[1];
    } else if (t < 60) {
      // Majority: 1 if at least 2 of the 3 inputs are 1
      f = ((b & c) | (b & d) | (c & d)) >>> 0;
      k = K[2];
    } else {
      // Parity again (same formula, different constant)
      f = (b ^ c ^ d) >>> 0;
      k = K[3];
    }
    const temp = (rotl(5, a) + f + e + k + W[t]) >>> 0;
    e = d;
    d = c;
    c = rotl(30, b);
    b = a;
    a = temp;
  }

  return [
    (h0 + a) >>> 0,
    (h1 + b) >>> 0,
    (h2 + c) >>> 0,
    (h3 + d) >>> 0,
    (h4 + e) >>> 0,
  ];
}

// ─── Finalization ────────────────────────────────────────────────────────────
//
// Convert the five 32-bit state words to 20 bytes in big-endian order.
// Big-endian = most significant byte first (natural human-readable order).
// DataView.setUint32(offset, value, false) writes big-endian.
function stateToBytes(
  state: [number, number, number, number, number],
): Uint8Array {
  const digest = new Uint8Array(20);
  const view = new DataView(digest.buffer);
  for (let i = 0; i < 5; i++) {
    view.setUint32(i * 4, state[i], false); // big-endian
  }
  return digest;
}

// ─── Public API ───────────────────────────────────────────────────────────────

/**
 * Compute the SHA-1 digest of data. Returns 20 bytes.
 *
 * This is the one-shot API: hash a complete message in a single call.
 *
 * @example
 * ```ts
 * const enc = new TextEncoder();
 * const digest = sha1(enc.encode("abc"));
 * console.log(toHex(digest)); // → "a9993e364706816aba3e25717850c26c9cd0d89d"
 * ```
 */
export function sha1(data: Uint8Array): Uint8Array {
  const padded = pad(data);
  let state: [number, number, number, number, number] = [...INIT];
  for (let i = 0; i < padded.length; i += 64) {
    state = compress(state, padded.subarray(i, i + 64));
  }
  return stateToBytes(state);
}

/**
 * Convert a Uint8Array to a lowercase hex string.
 *
 * Each byte becomes exactly two hex characters, zero-padded:
 *   0x0A → "0a",  0xFF → "ff"
 */
export function toHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

/**
 * Compute SHA-1 and return the 40-character lowercase hex string.
 *
 * @example
 * ```ts
 * const enc = new TextEncoder();
 * sha1Hex(enc.encode("abc")); // → "a9993e364706816aba3e25717850c26c9cd0d89d"
 * ```
 */
export function sha1Hex(data: Uint8Array): string {
  return toHex(sha1(data));
}

/**
 * Streaming SHA-1 hasher that accepts data in multiple chunks.
 *
 * Useful when the full message is not available at once — for example when
 * reading a large file in chunks or hashing a network stream.
 *
 * The interface mirrors the Web Crypto / Node.js hash APIs:
 *
 * ```ts
 * const h = new SHA1Hasher();
 * h.update(enc.encode("ab"));
 * h.update(enc.encode("c"));
 * h.hexDigest(); // → "a9993e364706816aba3e25717850c26c9cd0d89d"
 * ```
 *
 * Multiple update() calls are equivalent to a single sha1(all_data).
 *
 * Implementation note:
 *   Data accumulates in a buffer. When the buffer reaches 64 bytes we compress
 *   the first block and discard it (keeping the state). On digest(), we pad
 *   whatever remains (using the TOTAL byte count) and compress the padding.
 */
export class SHA1Hasher {
  private _state: [number, number, number, number, number];
  private _buffer: number[]; // partial block (< 64 bytes)
  private _byteCount: number; // total bytes fed in

  constructor() {
    this._state = [...INIT];
    this._buffer = [];
    this._byteCount = 0;
  }

  /** Feed more bytes into the hash. Returns `this` for method chaining. */
  update(data: Uint8Array): this {
    this._byteCount += data.length;
    for (const byte of data) {
      this._buffer.push(byte);
      if (this._buffer.length === 64) {
        this._state = compress(
          this._state,
          new Uint8Array(this._buffer),
        );
        this._buffer = [];
      }
    }
    return this;
  }

  /**
   * Return the 20-byte digest of all data fed so far.
   *
   * Non-destructive: the internal state is not modified, so you can continue
   * calling update() after calling digest().
   */
  digest(): Uint8Array {
    // Pad the remaining buffer using the TOTAL byte count (not buffer length).
    // We must re-pad from the buffer tail, not from the full message, because
    // earlier blocks have already been compressed into _state.
    const bitLenHigh = Math.floor((this._byteCount * 8) / 0x100000000) >>> 0;
    const bitLenLow = (this._byteCount * 8) >>> 0;

    const tailBytes = new Uint8Array(this._buffer);
    const tailPad = pad(tailBytes);

    // But wait — pad() uses tailBytes.length as the byte count, which is wrong.
    // We need the TOTAL byte count for the length field. Overwrite the last 8
    // bytes of tailPad with the correct total length.
    //
    // Actually, we need to construct the padding tail differently for streaming.
    // The padding rule: append 0x80, zeros, then 64-bit big-endian total bit count.
    // The partial buffer acts as the "remainder" but the length must be total bytes.
    const buf = this._buffer;
    const zeroes = ((56 - (buf.length + 1)) % 64 + 64) % 64;
    const tail = new Uint8Array(buf.length + 1 + zeroes + 8);
    tail.set(buf, 0);
    tail[buf.length] = 0x80;
    const view = new DataView(tail.buffer);
    view.setUint32(buf.length + 1 + zeroes, bitLenHigh, false);
    view.setUint32(buf.length + 1 + zeroes + 4, bitLenLow, false);

    // Compress the padding tail against a copy of the live state
    let state: [number, number, number, number, number] = [...this._state];
    for (let i = 0; i < tail.length; i += 64) {
      state = compress(state, tail.subarray(i, i + 64));
    }
    return stateToBytes(state);
  }

  /** Return the 40-character hex string of the digest. */
  hexDigest(): string {
    return toHex(this.digest());
  }

  /**
   * Return an independent copy of this hasher.
   *
   * Useful for computing multiple hashes that share a common prefix:
   *
   * ```ts
   * const h = new SHA1Hasher().update(commonPrefix);
   * const h1 = h.copy(); h1.update(suffixA);
   * const h2 = h.copy(); h2.update(suffixB);
   * ```
   */
  copy(): SHA1Hasher {
    const other = new SHA1Hasher();
    other._state = [...this._state];
    other._buffer = [...this._buffer];
    other._byteCount = this._byteCount;
    return other;
  }
}
