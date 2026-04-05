/**
 * @coding-adventures/sha512
 *
 * SHA-512 cryptographic hash function (FIPS 180-4) implemented from scratch.
 *
 * What Is SHA-512?
 * ================
 * SHA-512 is the 64-bit sibling of SHA-256 in the SHA-2 family. It takes any
 * sequence of bytes and produces a fixed-size 64-byte (512-bit) digest. The
 * same input always produces the same digest. Change even one bit and the
 * digest changes completely (the "avalanche effect").
 *
 * On 64-bit platforms, SHA-512 is often *faster* than SHA-256 because it
 * processes 128-byte blocks (vs 64-byte) using native 64-bit arithmetic.
 *
 * How It Differs from SHA-256
 * ===========================
 * SHA-512 is structurally identical to SHA-256 but with wider words:
 *
 *   Property         SHA-256       SHA-512
 *   ────────         ───────       ───────
 *   Word size        32-bit        64-bit
 *   State words      8 x u32       8 x u64
 *   Block size       64 bytes      128 bytes
 *   Rounds           64            80
 *   Digest size      32 bytes      64 bytes
 *   Length field      64-bit       128-bit
 *
 * The rotation/shift amounts also differ (tuned for 64-bit words).
 *
 * JavaScript 64-bit Integer Challenge
 * ====================================
 * JavaScript's bitwise operators work on 32-bit integers only. For SHA-512's
 * 64-bit operations, we use BigInt — an arbitrary-precision integer type built
 * into modern JavaScript engines (ES2020+).
 *
 * BigInt arithmetic:
 *   - Literals: 0n, 42n, 0xFFn
 *   - Operators: +, -, *, /, %, **, &, |, ^, ~, <<, >>
 *   - CANNOT mix BigInt with Number (1n + 1 throws TypeError)
 *   - We mask with & 0xFFFFFFFFFFFFFFFFn after every operation to stay in 64 bits
 *
 * BigInt is slower than Number for 32-bit operations, but for educational code
 * clarity is more important than speed. A production implementation would use
 * manual hi/lo 32-bit pairs or WebAssembly.
 *
 * FIPS 180-4 Test Vectors
 * =======================
 *   sha512("") = cf83e1357eefb8bd...af927da3e  (128 hex chars)
 *   sha512("abc") = ddaf35a193617aba...a54ca49f  (128 hex chars)
 */

export const VERSION = "0.1.0";

// ─── 64-bit Mask ────────────────────────────────────────────────────────────
//
// BigInt has arbitrary precision, so we must manually truncate to 64 bits
// after every arithmetic operation. This mask keeps only the lower 64 bits.
//
// 0xFFFFFFFFFFFFFFFF = 2^64 - 1 = 18446744073709551615

const MASK64 = 0xFFFFFFFFFFFFFFFFn;

// ─── Initialization Constants ───────────────────────────────────────────────
//
// SHA-512 starts with these eight 64-bit words as its initial state.
// They are the first 64 bits of the fractional parts of the square roots
// of the first 8 primes (2, 3, 5, 7, 11, 13, 17, 19).
//
//   H₀ = frac(sqrt(2))  * 2^64 = 0x6a09e667f3bcc908
//   H₁ = frac(sqrt(3))  * 2^64 = 0xbb67ae8584caa73b
//   ...and so on

const INIT: bigint[] = [
  0x6a09e667f3bcc908n,
  0xbb67ae8584caa73bn,
  0x3c6ef372fe94f82bn,
  0xa54ff53a5f1d36f1n,
  0x510e527fade682d1n,
  0x9b05688c2b3e6c1fn,
  0x1f83d9abfb41bd6bn,
  0x5be0cd19137e2179n,
];

// ─── Round Constants ────────────────────────────────────────────────────────
//
// 80 constants, one per round. Each is the first 64 bits of the fractional
// part of the cube root of the i-th prime (2, 3, 5, 7, 11, ..., 409).
//
// These "nothing up my sleeve" numbers prove no backdoor is hidden —
// anyone can verify them by computing cube roots of primes.

const K: bigint[] = [
  0x428a2f98d728ae22n, 0x7137449123ef65cdn, 0xb5c0fbcfec4d3b2fn, 0xe9b5dba58189dbbcn,
  0x3956c25bf348b538n, 0x59f111f1b605d019n, 0x923f82a4af194f9bn, 0xab1c5ed5da6d8118n,
  0xd807aa98a3030242n, 0x12835b0145706fben, 0x243185be4ee4b28cn, 0x550c7dc3d5ffb4e2n,
  0x72be5d74f27b896fn, 0x80deb1fe3b1696b1n, 0x9bdc06a725c71235n, 0xc19bf174cf692694n,
  0xe49b69c19ef14ad2n, 0xefbe4786384f25e3n, 0x0fc19dc68b8cd5b5n, 0x240ca1cc77ac9c65n,
  0x2de92c6f592b0275n, 0x4a7484aa6ea6e483n, 0x5cb0a9dcbd41fbd4n, 0x76f988da831153b5n,
  0x983e5152ee66dfabn, 0xa831c66d2db43210n, 0xb00327c898fb213fn, 0xbf597fc7beef0ee4n,
  0xc6e00bf33da88fc2n, 0xd5a79147930aa725n, 0x06ca6351e003826fn, 0x142929670a0e6e70n,
  0x27b70a8546d22ffcn, 0x2e1b21385c26c926n, 0x4d2c6dfc5ac42aedn, 0x53380d139d95b3dfn,
  0x650a73548baf63den, 0x766a0abb3c77b2a8n, 0x81c2c92e47edaee6n, 0x92722c851482353bn,
  0xa2bfe8a14cf10364n, 0xa81a664bbc423001n, 0xc24b8b70d0f89791n, 0xc76c51a30654be30n,
  0xd192e819d6ef5218n, 0xd69906245565a910n, 0xf40e35855771202an, 0x106aa07032bbd1b8n,
  0x19a4c116b8d2d0c8n, 0x1e376c085141ab53n, 0x2748774cdf8eeb99n, 0x34b0bcb5e19b48a8n,
  0x391c0cb3c5c95a63n, 0x4ed8aa4ae3418acbn, 0x5b9cca4f7763e373n, 0x682e6ff3d6b2b8a3n,
  0x748f82ee5defb2fcn, 0x78a5636f43172f60n, 0x84c87814a1f0ab72n, 0x8cc702081a6439ecn,
  0x90befffa23631e28n, 0xa4506cebde82bde9n, 0xbef9a3f7b2c67915n, 0xc67178f2e372532bn,
  0xca273eceea26619cn, 0xd186b8c721c0c207n, 0xeada7dd6cde0eb1en, 0xf57d4f7fee6ed178n,
  0x06f067aa72176fban, 0x0a637dc5a2c898a6n, 0x113f9804bef90daen, 0x1b710b35131c471bn,
  0x28db77f523047d84n, 0x32caab7b40c72493n, 0x3c9ebe0a15c9bebcn, 0x431d67c49c100d4cn,
  0x4cc5d4becb3e42b6n, 0x597f299cfc657e2an, 0x5fcb6fab3ad6faecn, 0x6c44198c4a475817n,
];

// ─── Helper: 64-bit Right Rotation ──────────────────────────────────────────
//
// rotr(n, x) rotates x right by n bit positions within a 64-bit word.
// Bits that "fall off" the right end reappear on the left.
//
// BigInt's >> is arithmetic right shift (sign-extending), but since we mask
// to 64 bits our values are always non-negative, so >> behaves like a
// logical shift. We mask the result to stay in 64-bit range.
//
// Example (8-bit for clarity): rotr(2, 0b11010011)
//   Lower bits shifted right: 0b11010011 >> 2 = 0b00110100
//   Upper bits wrapped:       0b11010011 << 6 = 0b11000000
//   Combined:                 0b00110100 | 0b11000000 = 0b11110100

function rotr(n: bigint, x: bigint): bigint {
  return ((x >> n) | (x << (64n - n))) & MASK64;
}

// ─── SHA-512 Sigma Functions ────────────────────────────────────────────────
//
// SHA-512 uses four mixing functions, each combining rotations and shifts.
// Capital sigma (Σ) operates on state words; lowercase sigma (σ) operates
// on message schedule words.
//
// The rotation amounts are specifically chosen for 64-bit words:
//
//   Σ0(x) = ROTR(28,x) XOR ROTR(34,x) XOR ROTR(39,x)
//   Σ1(x) = ROTR(14,x) XOR ROTR(18,x) XOR ROTR(41,x)
//   σ0(x) = ROTR(1,x)  XOR ROTR(8,x)  XOR (x >> 7)
//   σ1(x) = ROTR(19,x) XOR ROTR(61,x) XOR (x >> 6)
//
// Note: σ0 and σ1 use a right SHIFT (not rotation) for their third term.
// A shift discards bits; a rotation preserves them. The shift makes the
// schedule expansion non-invertible.

function bigSigma0(x: bigint): bigint {
  return rotr(28n, x) ^ rotr(34n, x) ^ rotr(39n, x);
}

function bigSigma1(x: bigint): bigint {
  return rotr(14n, x) ^ rotr(18n, x) ^ rotr(41n, x);
}

function smallSigma0(x: bigint): bigint {
  return rotr(1n, x) ^ rotr(8n, x) ^ (x >> 7n);
}

function smallSigma1(x: bigint): bigint {
  return rotr(19n, x) ^ rotr(61n, x) ^ (x >> 6n);
}

// ─── Choice and Majority ────────────────────────────────────────────────────
//
// Two boolean functions used in each round of compression:
//
// Ch(x, y, z) — "Choice": for each bit position, x chooses between y and z.
//   If x bit = 1, output y bit. If x bit = 0, output z bit.
//   Formula: (x AND y) XOR (NOT x AND z)
//
//   Truth table:
//     x=0: (0 & y) ^ (1 & z) = z  (choose z)
//     x=1: (1 & y) ^ (0 & z) = y  (choose y)
//
// Maj(x, y, z) — "Majority": output the bit value that appears in at
// least 2 of the 3 inputs.
//   Formula: (x AND y) XOR (x AND z) XOR (y AND z)
//
//   Truth table:
//     0,0,0 → 0    0,0,1 → 0    0,1,0 → 0    0,1,1 → 1
//     1,0,0 → 0    1,0,1 → 1    1,1,0 → 1    1,1,1 → 1

function ch(x: bigint, y: bigint, z: bigint): bigint {
  return ((x & y) ^ ((~x & MASK64) & z)) & MASK64;
}

function maj(x: bigint, y: bigint, z: bigint): bigint {
  return (x & y) ^ (x & z) ^ (y & z);
}

// ─── Padding ────────────────────────────────────────────────────────────────
//
// SHA-512 processes 128-byte (1024-bit) blocks. Padding extends the message:
//
//   1. Append 0x80 (the '1' bit followed by seven '0' bits).
//   2. Append 0x00 bytes until length ≡ 112 (mod 128).
//   3. Append the original bit length as a 128-bit big-endian integer.
//
// Why 112 mod 128? We need 16 bytes for the length field, and
// 112 + 16 = 128, filling exactly one block. If the message plus 0x80
// would exceed byte 112, we need an extra block.
//
// For practical purposes the bit length fits in 64 bits (messages up to
// 2^61 bytes = 2 exabytes). We write zeros for the high 64 bits and the
// actual bit count for the low 64 bits.

function pad(data: Uint8Array): Uint8Array {
  const byteLen = data.length;
  // Bit length as BigInt for the 128-bit length field
  const bitLen = BigInt(byteLen) * 8n;

  // How many zero bytes after 0x80?
  // Solve: (byteLen + 1 + zeroes) mod 128 === 112
  const zeroes = ((112 - (byteLen + 1)) % 128 + 128) % 128;
  const totalLen = byteLen + 1 + zeroes + 16;
  const padded = new Uint8Array(totalLen);

  // Copy original data
  padded.set(data, 0);
  // Append the mandatory '1' bit
  padded[byteLen] = 0x80;
  // Zero bytes are already 0 from Uint8Array initialization

  // Append 128-bit big-endian length (16 bytes at the end)
  // High 64 bits (bitLen >> 64) — zero for practical messages
  const highBits = bitLen >> 64n;
  const lowBits = bitLen & MASK64;
  const view = new DataView(padded.buffer);
  // Write high 64 bits as two 32-bit words
  const highHi = Number((highBits >> 32n) & 0xFFFFFFFFn);
  const highLo = Number(highBits & 0xFFFFFFFFn);
  const lowHi = Number((lowBits >> 32n) & 0xFFFFFFFFn);
  const lowLo = Number(lowBits & 0xFFFFFFFFn);
  const lenOffset = byteLen + 1 + zeroes;
  view.setUint32(lenOffset, highHi, false);
  view.setUint32(lenOffset + 4, highLo, false);
  view.setUint32(lenOffset + 8, lowHi, false);
  view.setUint32(lenOffset + 12, lowLo, false);

  return padded;
}

// ─── Message Schedule ───────────────────────────────────────────────────────
//
// Each 128-byte block is parsed as 16 big-endian 64-bit words (W[0..15]),
// then expanded to 80 words using:
//
//   W[i] = σ1(W[i-2]) + W[i-7] + σ0(W[i-15]) + W[i-16]   (mod 2^64)
//
// This is different from SHA-1 which uses a simple XOR-and-rotate. SHA-512
// uses the σ functions (rotation + shift combinations) for stronger diffusion.

function schedule(block: Uint8Array): bigint[] {
  const W: bigint[] = new Array(80);
  const view = new DataView(block.buffer, block.byteOffset, block.byteLength);

  // Parse 16 big-endian 64-bit words from the 128-byte block.
  // DataView doesn't have getUint64, so we read two 32-bit halves.
  for (let i = 0; i < 16; i++) {
    const hi = BigInt(view.getUint32(i * 8, false));
    const lo = BigInt(view.getUint32(i * 8 + 4, false));
    W[i] = ((hi << 32n) | lo) & MASK64;
  }

  // Expand from 16 to 80 words
  for (let i = 16; i < 80; i++) {
    W[i] = (smallSigma1(W[i - 2]) + W[i - 7] + smallSigma0(W[i - 15]) + W[i - 16]) & MASK64;
  }

  return W;
}

// ─── Compression Function ───────────────────────────────────────────────────
//
// 80 rounds of mixing fold one 128-byte block into the eight-word state.
//
// Unlike SHA-1's four distinct stages, SHA-512 uses the same pair of
// functions (Ch and Maj) in every round, with a different round constant K[t].
//
// Each round computes two temporary values:
//
//   T₁ = h + Σ1(e) + Ch(e,f,g) + K[t] + W[t]
//   T₂ = Σ0(a) + Maj(a,b,c)
//
// Then the eight working variables shift down:
//   h=g, g=f, f=e, e=d+T₁, d=c, c=b, b=a, a=T₁+T₂
//
// The shift pattern creates a cascade: each variable gets pushed through
// all eight positions over 8 rounds, ensuring thorough mixing.
//
// Davies-Meyer feed-forward: after all 80 rounds, add the compressed
// output back to the original state. This prevents invertibility.

function compress(state: bigint[], block: Uint8Array): bigint[] {
  const W = schedule(block);
  const [h0, h1, h2, h3, h4, h5, h6, h7] = state;
  let a = h0, b = h1, c = h2, d = h3,
      e = h4, f = h5, g = h6, h = h7;

  for (let t = 0; t < 80; t++) {
    const T1 = (h + bigSigma1(e) + ch(e, f, g) + K[t] + W[t]) & MASK64;
    const T2 = (bigSigma0(a) + maj(a, b, c)) & MASK64;
    h = g;
    g = f;
    f = e;
    e = (d + T1) & MASK64;
    d = c;
    c = b;
    b = a;
    a = (T1 + T2) & MASK64;
  }

  return [
    (h0 + a) & MASK64,
    (h1 + b) & MASK64,
    (h2 + c) & MASK64,
    (h3 + d) & MASK64,
    (h4 + e) & MASK64,
    (h5 + f) & MASK64,
    (h6 + g) & MASK64,
    (h7 + h) & MASK64,
  ];
}

// ─── Finalization ───────────────────────────────────────────────────────────
//
// Convert the eight 64-bit state words to 64 bytes in big-endian order.
// Each 64-bit word becomes 8 bytes, most significant byte first.

function stateToBytes(state: bigint[]): Uint8Array {
  const digest = new Uint8Array(64);
  const view = new DataView(digest.buffer);
  for (let i = 0; i < 8; i++) {
    const word = state[i];
    view.setUint32(i * 8, Number((word >> 32n) & 0xFFFFFFFFn), false);
    view.setUint32(i * 8 + 4, Number(word & 0xFFFFFFFFn), false);
  }
  return digest;
}

// ─── Public API ─────────────────────────────────────────────────────────────

/**
 * Compute the SHA-512 digest of data. Returns 64 bytes.
 *
 * This is the one-shot API: hash a complete message in a single call.
 *
 * @example
 * ```ts
 * const enc = new TextEncoder();
 * const digest = sha512(enc.encode("abc"));
 * console.log(toHex(digest)); // → "ddaf35a193617aba..."
 * ```
 */
export function sha512(data: Uint8Array): Uint8Array {
  const padded = pad(data);
  let state = [...INIT];
  for (let i = 0; i < padded.length; i += 128) {
    state = compress(state, padded.subarray(i, i + 128));
  }
  return stateToBytes(state);
}

/**
 * Convert a Uint8Array to a lowercase hex string.
 *
 * Each byte becomes exactly two hex characters, zero-padded:
 *   0x0A -> "0a",  0xFF -> "ff"
 */
export function toHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

/**
 * Compute SHA-512 and return the 128-character lowercase hex string.
 *
 * @example
 * ```ts
 * const enc = new TextEncoder();
 * sha512Hex(enc.encode("abc")); // → "ddaf35a193617aba..."
 * ```
 */
export function sha512Hex(data: Uint8Array): string {
  return toHex(sha512(data));
}

/**
 * Streaming SHA-512 hasher that accepts data in multiple chunks.
 *
 * Useful when the full message is not available at once -- for example when
 * reading a large file in chunks or hashing a network stream.
 *
 * ```ts
 * const h = new SHA512Hasher();
 * h.update(enc.encode("ab"));
 * h.update(enc.encode("c"));
 * h.hexDigest(); // → "ddaf35a193617aba..."
 * ```
 *
 * Multiple update() calls are equivalent to a single sha512(all_data).
 *
 * Implementation note:
 *   Data accumulates in a buffer. When the buffer reaches 128 bytes we
 *   compress the first block and discard it (keeping the state). On
 *   digest(), we pad whatever remains (using the TOTAL byte count) and
 *   compress the padding.
 */
export class SHA512Hasher {
  private _state: bigint[];
  private _buffer: number[];   // partial block (< 128 bytes)
  private _byteCount: number;  // total bytes fed in

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
      if (this._buffer.length === 128) {
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
   * Return the 64-byte digest of all data fed so far.
   *
   * Non-destructive: the internal state is not modified, so you can
   * continue calling update() after calling digest().
   */
  digest(): Uint8Array {
    // Build padding tail using the TOTAL byte count
    const bitLen = BigInt(this._byteCount) * 8n;
    const buf = this._buffer;
    const zeroes = ((112 - (buf.length + 1)) % 128 + 128) % 128;
    const tail = new Uint8Array(buf.length + 1 + zeroes + 16);
    tail.set(buf, 0);
    tail[buf.length] = 0x80;

    // Write 128-bit big-endian length
    const highBits = bitLen >> 64n;
    const lowBits = bitLen & MASK64;
    const view = new DataView(tail.buffer);
    const lenOffset = buf.length + 1 + zeroes;
    view.setUint32(lenOffset, Number((highBits >> 32n) & 0xFFFFFFFFn), false);
    view.setUint32(lenOffset + 4, Number(highBits & 0xFFFFFFFFn), false);
    view.setUint32(lenOffset + 8, Number((lowBits >> 32n) & 0xFFFFFFFFn), false);
    view.setUint32(lenOffset + 12, Number(lowBits & 0xFFFFFFFFn), false);

    // Compress the padding tail against a copy of the live state
    let state = [...this._state];
    for (let i = 0; i < tail.length; i += 128) {
      state = compress(state, tail.subarray(i, i + 128));
    }
    return stateToBytes(state);
  }

  /** Return the 128-character hex string of the digest. */
  hexDigest(): string {
    return toHex(this.digest());
  }

  /**
   * Return an independent copy of this hasher.
   *
   * Useful for computing multiple hashes that share a common prefix:
   *
   * ```ts
   * const h = new SHA512Hasher().update(commonPrefix);
   * const h1 = h.copy(); h1.update(suffixA);
   * const h2 = h.copy(); h2.update(suffixB);
   * ```
   */
  copy(): SHA512Hasher {
    const other = new SHA512Hasher();
    other._state = [...this._state];
    other._buffer = [...this._buffer];
    other._byteCount = this._byteCount;
    return other;
  }
}
