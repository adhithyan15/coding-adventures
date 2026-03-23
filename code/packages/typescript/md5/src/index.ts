/**
 * @coding-adventures/md5
 *
 * MD5 message digest algorithm (RFC 1321) implemented from scratch in TypeScript.
 *
 * This package is part of the coding-adventures monorepo, a ground-up implementation
 * of the computing stack from transistors to operating systems.
 *
 * What Is MD5?
 * ============
 * MD5 (Message Digest 5) takes any sequence of bytes and produces a fixed-size
 * 16-byte (128-bit) "fingerprint" called a digest. The same input always produces
 * the same digest. Change even one bit of input and the digest changes completely.
 *
 * Created by Ron Rivest in 1991 as an improvement over MD4. Standardized in
 * RFC 1321. MD5 is cryptographically broken (collision attacks since 2004) and
 * should NOT be used for security purposes (digital signatures, password hashing,
 * TLS certificates). It remains valid for: non-security checksums, UUID v3, and
 * legacy systems that already use it.
 *
 * The #1 Gotcha: Little-Endian Throughout
 * ========================================
 * MD5 is LITTLE-ENDIAN: least significant byte first. This differs from SHA-1
 * (big-endian) and is the source of most MD5 implementation bugs.
 *
 *   Big-endian (SHA-1):    0x0A0B0C0D → bytes [0A, 0B, 0C, 0D]
 *   Little-endian (MD5):   0x0A0B0C0D → bytes [0D, 0C, 0B, 0A]
 *
 * In JavaScript/TypeScript we use DataView with the `littleEndian` flag:
 *   DataView.getUint32(offset, true)   ← true = little-endian (MD5)
 *   DataView.setUint32(offset, v, true) ← true = little-endian (MD5)
 *
 * JavaScript 32-bit Arithmetic Caveat
 * =====================================
 * JavaScript's bitwise operators (~, |, &, ^, <<, >>) work on signed 32-bit
 * integers internally. After any bitwise op the result might be a negative number
 * if the top bit is set. We force an unsigned interpretation with `>>> 0`:
 *
 *   ~0 in JS gives -1 (signed), but (~0) >>> 0 gives 4294967295 (unsigned)
 *   0xFFFFFFFF | 0 gives -1 (signed), but (0xFFFFFFFF | 0) >>> 0 gives 4294967295
 *
 * Always use `>>> 0` after bitwise NOT (~) and after additions to stay unsigned.
 */

export const VERSION = "0.1.0";

// ─── T-Table: 64 Constants Derived From Sine ───────────────────────────────
//
// T[i] = floor(abs(sin(i+1)) × 2^32)  for i = 0..63
//   (RFC 1321 uses 1-based indexing; we use 0-based internally)
//
// These are "nothing up my sleeve" numbers: anyone can verify them from the
// standard sine function. No hidden backdoor is possible because the derivation
// is fully public. Example:
//
//   sin(1) ≈ 0.84147...
//   |sin(1)| × 2^32 = 3614090360.02...
//   floor(...) = 3614090360 = 0xD76AA478 = T[0]
//
// We use `Math.floor(Math.abs(Math.sin(i + 1)) * 0x100000000) >>> 0` to:
//   1. Multiply by 2^32 (0x100000000 = 4294967296)
//   2. Floor to integer
//   3. Force unsigned 32-bit with >>> 0

const T: Uint32Array = new Uint32Array(64);
for (let i = 0; i < 64; i++) {
  T[i] = (Math.floor(Math.abs(Math.sin(i + 1)) * 0x100000000)) >>> 0;
}

// ─── Round Shift Amounts ────────────────────────────────────────────────────
//
// Each of the 64 rounds rotates left by a specific number of bits. The pattern
// is fixed by the RFC — four groups of 16, each repeating a 4-element cycle.
// These values were chosen to maximize diffusion (avalanche effect).
//
//   Rounds  0–15:  [7, 12, 17, 22] × 4   (Stage 1 — F function)
//   Rounds 16–31:  [5,  9, 14, 20] × 4   (Stage 2 — G function)
//   Rounds 32–47:  [4, 11, 16, 23] × 4   (Stage 3 — H function)
//   Rounds 48–63:  [6, 10, 15, 21] × 4   (Stage 4 — I function)

const S: readonly number[] = [
  7, 12, 17, 22,  7, 12, 17, 22,  7, 12, 17, 22,  7, 12, 17, 22, // rounds  0–15
  5,  9, 14, 20,  5,  9, 14, 20,  5,  9, 14, 20,  5,  9, 14, 20, // rounds 16–31
  4, 11, 16, 23,  4, 11, 16, 23,  4, 11, 16, 23,  4, 11, 16, 23, // rounds 32–47
  6, 10, 15, 21,  6, 10, 15, 21,  6, 10, 15, 21,  6, 10, 15, 21, // rounds 48–63
];

// ─── Initialization Constants ────────────────────────────────────────────────
//
// The four-word state starts with these fixed values. They look like the hex
// sequence 0123456789ABCDEF split into bytes and reversed pairwise:
//
//   A = 0x67452301 → byte sequence: 01 23 45 67 (reversed → 67 45 23 01)
//   B = 0xEFCDAB89 → byte sequence: 89 AB CD EF (reversed → EF CD AB 89)
//   C = 0x98BADCFE → byte sequence: FE DC BA 98 (reversed → 98 BA DC FE)
//   D = 0x10325476 → byte sequence: 76 54 32 10 (reversed → 10 32 54 76)

const INIT_A = 0x67452301;
const INIT_B = 0xEFCDAB89;
const INIT_C = 0x98BADCFE;
const INIT_D = 0x10325476;

// ─── Helper: Circular Left Rotation ─────────────────────────────────────────
//
// Rotate x left by n bits within a 32-bit word. Bits that fall off the left
// reappear on the right. The >>> 0 ensures the result stays unsigned 32-bit.
//
//   rotl(0x80000000, 1) = 0x00000001  (top bit wraps around to bottom)

function rotl(x: number, n: number): number {
  return ((x << n) | (x >>> (32 - n))) >>> 0;
}

// ─── Padding ─────────────────────────────────────────────────────────────────
//
// MD5 operates on 512-bit (64-byte) blocks. Messages that aren't a multiple of
// 64 bytes need padding according to RFC 1321 §3.1:
//
//   1. Append the byte 0x80 (a single 1-bit followed by zeros).
//   2. Append zero bytes until the total length ≡ 56 (mod 64).
//      This leaves 8 bytes at the end of each 64-byte block for the length.
//   3. Append the original bit-length as a 64-bit LITTLE-ENDIAN integer.
//      (This differs from SHA-1, which uses big-endian here!)
//
// Example — "abc" (3 bytes = 24 bits):
//   61 62 63 80 [52 zero bytes] 18 00 00 00 00 00 00 00
//                               ^^ LE encoding of 24 (0x18)
//
// A message that is already 56 bytes gets a full extra block (56 → 64+56=120):
//   0x80 + 7 zeros fills it to 64, then 56 zeros + length → 64 more bytes.

function pad(data: Uint8Array): Uint8Array {
  const bitLen = data.length * 8;

  // How many zero bytes do we need after the 0x80 byte?
  // After appending 0x80, the total is (data.length + 1) bytes.
  // We want (data.length + 1 + zeroCount) % 64 === 56.
  // So: zeroCount = (56 - (data.length + 1) % 64 + 64) % 64
  // The extra +64 and %64 handle the case where (data.length+1)%64 > 56.
  const afterBit = (data.length + 1) % 64;
  const zeroCount = afterBit <= 56 ? 56 - afterBit : 64 + 56 - afterBit;

  // Total = original + 1 (0x80) + zeroCount + 8 (length)
  const result = new Uint8Array(data.length + 1 + zeroCount + 8);
  result.set(data);
  result[data.length] = 0x80;
  // Zero bytes fill automatically (Uint8Array is zero-initialized)

  // Append 64-bit LITTLE-endian bit length.
  // JavaScript numbers are 64-bit floats, safe integers up to 2^53.
  // For very large messages we split into low and high 32-bit halves.
  const view = new DataView(result.buffer);
  const lo = bitLen >>> 0;                           // low 32 bits
  const hi = Math.floor(bitLen / 0x100000000) >>> 0; // high 32 bits
  view.setUint32(result.length - 8, lo, true);  // ← true = little-endian
  view.setUint32(result.length - 4, hi, true);  // ← true = little-endian

  return result;
}

// ─── Compression Function ────────────────────────────────────────────────────
//
// The heart of MD5: mix one 64-byte block into the four-word state using 64
// rounds of bit manipulation. Each round uses one of four auxiliary functions:
//
//   Stage 1 (i < 16):  F(B,C,D) = (B & C) | (~B & D)  — "if B then C else D"
//   Stage 2 (i < 32):  G(B,C,D) = (D & B) | (~D & C)  — same but D selects
//   Stage 3 (i < 48):  H(B,C,D) = B ^ C ^ D            — parity
//   Stage 4 (i < 64):  I(B,C,D) = C ^ (B | ~D)         — unusual mix
//
// The I function (stage 4) is the most interesting:
//   When D=0: ~D=all-ones, B|~D=all-ones, result = C^1 (flips C)
//   When D=1: ~D=0, B|~D=B, result = C^B (XOR of C and B)
// This creates asymmetric mixing that differs per round.
//
// JAVASCRIPT UNSIGNED CAVEAT: ~B gives a SIGNED 32-bit result in JS.
//   ~0xFFFFFFFF === 0  (correct unsigned result)
//   ~0x80000000 === 0x7FFFFFFF (correct)
//   ~0x00000001 === -2 (signed! should be 0xFFFFFFFE unsigned)
//
// Solution: always apply >>> 0 after ~ to force unsigned:
//   (~B) >>> 0   — now it's unsigned 32-bit
//
// Message word selection g per stage:
//   Stage 1: g = i            (sequential: 0, 1, 2, ..., 15)
//   Stage 2: g = (5i + 1)%16  (stride 5: 1, 6, 11, 0, 5, ...)
//   Stage 3: g = (3i + 5)%16  (stride 3: 5, 8, 11, 14, 1, ...)
//   Stage 4: g = (7i) % 16    (stride 7: 0, 7, 14, 5, 12, ...)
//
// Each round:
//   temp = B + ROTL(S[i], A + f + M[g] + T[i])  (mod 2^32)
//   (A, B, C, D) ← (D, temp, B, C)
//
// Davies-Meyer feed-forward: after all 64 rounds, add the pre-round state
// (mod 2^32) to prevent the compression from being invertible.

function compress(
  stateA: number,
  stateB: number,
  stateC: number,
  stateD: number,
  block: Uint8Array,
  blockOffset: number
): [number, number, number, number] {
  // Parse 16 little-endian 32-bit words from the block.
  // DataView.getUint32(offset, true) reads little-endian — the true matters!
  const view = new DataView(block.buffer, block.byteOffset + blockOffset, 64);
  const M = new Uint32Array(16);
  for (let j = 0; j < 16; j++) {
    M[j] = view.getUint32(j * 4, true); // true = little-endian
  }

  // Save initial state for Davies-Meyer addition at the end.
  const a0 = stateA, b0 = stateB, c0 = stateC, d0 = stateD;
  let a = a0, b = b0, c = c0, d = d0;

  for (let i = 0; i < 64; i++) {
    let f: number;
    let g: number;

    if (i < 16) {
      // Stage 1 — F function: if B then C else D
      // (~B) >>> 0 forces unsigned before the AND, avoiding JS signed trap
      f = ((b & c) | ((~b) >>> 0 & d)) >>> 0;
      g = i;
    } else if (i < 32) {
      // Stage 2 — G function: if D then B else C (D and B roles swapped from F)
      f = ((d & b) | ((~d) >>> 0 & c)) >>> 0;
      g = (5 * i + 1) % 16;
    } else if (i < 48) {
      // Stage 3 — H function: bitwise parity (XOR of all three)
      f = (b ^ c ^ d) >>> 0;
      g = (3 * i + 5) % 16;
    } else {
      // Stage 4 — I function: C XOR (B OR NOT D)
      // (~d) >>> 0 first, then OR with b, then XOR with c
      f = (c ^ (b | ((~d) >>> 0))) >>> 0;
      g = (7 * i) % 16;
    }

    // Core round computation:
    //   inner = (A + f + M[g] + T[i]) mod 2^32
    //   temp  = B + ROTL(inner, S[i])   mod 2^32
    const inner = (a + f + M[g] + T[i]) >>> 0;
    const temp  = (b + rotl(inner, S[i])) >>> 0;

    // Shift the four words: D→A, C→D, B→C, temp→B
    a = d;
    d = c;
    c = b;
    b = temp;
  }

  // Davies-Meyer: add compressed output to initial block state (mod 2^32).
  // This makes the compression function non-invertible even if the round
  // function were somehow reversed.
  return [
    (a0 + a) >>> 0,
    (b0 + b) >>> 0,
    (c0 + c) >>> 0,
    (d0 + d) >>> 0,
  ];
}

// ─── Output Serialization ────────────────────────────────────────────────────
//
// The final 16-byte digest is the four 32-bit state words written in
// LITTLE-ENDIAN byte order. DataView.setUint32(offset, value, true) does this.
//
// Example: if the final state word A = 0xD76AA478
//   Big-endian output:    D7 6A A4 78
//   Little-endian output: 78 A4 6A D7  ← what MD5 actually produces

function stateToBytes(a: number, b: number, c: number, d: number): Uint8Array {
  const out = new Uint8Array(16);
  const view = new DataView(out.buffer);
  view.setUint32(0,  a, true); // true = little-endian
  view.setUint32(4,  b, true);
  view.setUint32(8,  c, true);
  view.setUint32(12, d, true);
  return out;
}

// ─── Public API: Utilities ───────────────────────────────────────────────────

/**
 * Convert a Uint8Array to a lowercase hexadecimal string.
 *
 * Each byte becomes two hex characters. A 16-byte digest becomes 32 hex chars.
 * Example: toHex(new Uint8Array([0xd4, 0x1d])) === "d41d"
 */
export function toHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map(b => b.toString(16).padStart(2, "0"))
    .join("");
}

// ─── Public API: One-Shot md5 ────────────────────────────────────────────────

/**
 * Compute the MD5 digest of a Uint8Array. Returns 16 bytes.
 *
 * This is the one-shot API: hash a complete message in a single call.
 *
 * NOTE: MD5 is cryptographically broken. Do NOT use for passwords, digital
 * signatures, or security-sensitive checksums. Use for UUID v3 or legacy
 * compatibility only.
 *
 * RFC 1321 test vectors:
 *   md5(new TextEncoder().encode(""))    → d41d8cd98f00b204e9800998ecf8427e
 *   md5(new TextEncoder().encode("abc")) → 900150983cd24fb0d6963f7d28e17f72
 */
export function md5(data: Uint8Array): Uint8Array {
  const padded = pad(data);
  let a = INIT_A, b = INIT_B, c = INIT_C, d = INIT_D;

  // Process each 64-byte block sequentially.
  for (let offset = 0; offset < padded.length; offset += 64) {
    [a, b, c, d] = compress(a, b, c, d, padded, offset);
  }

  return stateToBytes(a, b, c, d);
}

// ─── Public API: Hex Variant ─────────────────────────────────────────────────

/**
 * Compute the MD5 digest of a Uint8Array and return it as a 32-character
 * lowercase hexadecimal string.
 *
 * Equivalent to toHex(md5(data)).
 *
 * Example:
 *   md5Hex(new TextEncoder().encode("abc")) === "900150983cd24fb0d6963f7d28e17f72"
 */
export function md5Hex(data: Uint8Array): string {
  return toHex(md5(data));
}

// ─── Public API: Streaming MD5Hasher ─────────────────────────────────────────
//
// When the full message is not available at once — e.g., reading a large file
// in chunks — the streaming API allows incremental updates.
//
// Internally, we keep:
//   - _state: the four-word running hash (updated after each complete block)
//   - _buffer: bytes accumulated but not yet forming a complete 64-byte block
//   - _byteCount: total bytes fed so far (needed for the padding length field)
//
// The update() method feeds complete 64-byte blocks to compress() immediately
// and buffers the remainder. The digest() method handles final padding without
// mutating the object state, so it can be called multiple times.

export class MD5Hasher {
  private _a: number;
  private _b: number;
  private _c: number;
  private _d: number;
  private _buffer: Uint8Array;
  private _bufLen: number;    // bytes currently in _buffer (0..63)
  private _byteCount: number; // total bytes fed so far

  constructor() {
    this._a = INIT_A;
    this._b = INIT_B;
    this._c = INIT_C;
    this._d = INIT_D;
    this._buffer = new Uint8Array(64);
    this._bufLen = 0;
    this._byteCount = 0;
  }

  /**
   * Feed more bytes into the hasher. Returns `this` for method chaining:
   *   hasher.update(chunk1).update(chunk2).digest()
   */
  update(data: Uint8Array): this {
    this._byteCount += data.length;
    let dataOffset = 0;

    // If there's a partial block buffered, try to fill it first.
    if (this._bufLen > 0) {
      const need = 64 - this._bufLen;
      const take = Math.min(need, data.length);
      this._buffer.set(data.subarray(0, take), this._bufLen);
      this._bufLen += take;
      dataOffset += take;

      if (this._bufLen === 64) {
        // Buffer is now a complete block; compress it.
        [this._a, this._b, this._c, this._d] =
          compress(this._a, this._b, this._c, this._d, this._buffer, 0);
        this._bufLen = 0;
      }
    }

    // Process remaining complete 64-byte blocks from data directly.
    while (dataOffset + 64 <= data.length) {
      [this._a, this._b, this._c, this._d] =
        compress(this._a, this._b, this._c, this._d, data, dataOffset);
      dataOffset += 64;
    }

    // Buffer any remaining bytes (< 64) for the next update() or digest().
    const remaining = data.length - dataOffset;
    if (remaining > 0) {
      this._buffer.set(data.subarray(dataOffset), this._bufLen);
      this._bufLen += remaining;
    }

    return this;
  }

  /**
   * Return the 16-byte MD5 digest of all data fed so far.
   *
   * Non-destructive: calling digest() multiple times returns the same bytes.
   * Subsequent update() calls can still be made after digest().
   */
  digest(): Uint8Array {
    // We compute the final padding on a copy of the current state, so the
    // original hasher object remains unchanged (supports multiple digest() calls).
    const tail = new Uint8Array(this._bufLen + 1 + 7 + 8); // generous upper bound
    tail.set(this._buffer.subarray(0, this._bufLen));
    tail[this._bufLen] = 0x80;

    // How many zero bytes to reach length ≡ 56 (mod 64)?
    const tailLen1 = this._bufLen + 1; // after 0x80
    const zeroCount = tailLen1 % 64 <= 56
      ? 56 - (tailLen1 % 64)
      : 64 + 56 - (tailLen1 % 64);

    const paddedLen = tailLen1 + zeroCount + 8;
    const padded = new Uint8Array(paddedLen);
    padded.set(tail.subarray(0, tailLen1));
    // Zero bytes are automatic (Uint8Array is zero-initialized)

    // Append 64-bit little-endian bit count.
    const bitLen = this._byteCount * 8;
    const lo = bitLen >>> 0;
    const hi = Math.floor(bitLen / 0x100000000) >>> 0;
    const view = new DataView(padded.buffer);
    view.setUint32(paddedLen - 8, lo, true);
    view.setUint32(paddedLen - 4, hi, true);

    // Compress the tail block(s) using a copy of the running state.
    let a = this._a, b = this._b, c = this._c, d = this._d;
    for (let offset = 0; offset < paddedLen; offset += 64) {
      [a, b, c, d] = compress(a, b, c, d, padded, offset);
    }

    return stateToBytes(a, b, c, d);
  }

  /**
   * Return the 32-character lowercase hex string of the digest.
   *
   * Equivalent to toHex(this.digest()).
   */
  hexDigest(): string {
    return toHex(this.digest());
  }

  /**
   * Return an independent copy of the current hasher.
   *
   * Useful for computing multiple digests from a common prefix:
   *   const h = new MD5Hasher().update(prefix);
   *   const hash1 = h.copy().update(suffix1).digest();
   *   const hash2 = h.copy().update(suffix2).digest();
   */
  copy(): MD5Hasher {
    const other = new MD5Hasher();
    other._a = this._a;
    other._b = this._b;
    other._c = this._c;
    other._d = this._d;
    other._buffer = new Uint8Array(this._buffer); // deep copy
    other._bufLen = this._bufLen;
    other._byteCount = this._byteCount;
    return other;
  }
}
