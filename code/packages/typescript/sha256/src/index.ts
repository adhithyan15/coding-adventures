/**
 * @coding-adventures/sha256
 *
 * SHA-256 cryptographic hash function (FIPS 180-4) implemented from scratch.
 *
 * What Is SHA-256?
 * ================
 * SHA-256 is a member of the SHA-2 family designed by the NSA and published by
 * NIST in 2001. It takes any sequence of bytes and produces a fixed-size 32-byte
 * (256-bit) "fingerprint" called a digest. The same input always produces the
 * same digest. Change even one bit and the digest changes completely — the
 * "avalanche effect". You cannot reverse a digest back to the original input.
 *
 * SHA-256 is the workhorse of modern cryptography: TLS certificates, Bitcoin
 * mining, git commit hashes, code signing, and password hashing all rely on it.
 * Unlike MD5 (broken 2004) and SHA-1 (broken 2017), SHA-256 remains secure with
 * no known practical attacks. The birthday bound is 2^128 operations.
 *
 * How SHA-256 Differs from SHA-1
 * ==============================
 * Both use the Merkle-Damgård construction (pad, split into blocks, compress),
 * but SHA-256 is stronger in every dimension:
 *
 *   Property      SHA-1          SHA-256
 *   ─────────     ─────          ───────
 *   State words   5 × 32-bit     8 × 32-bit
 *   Rounds        80             64
 *   Block size    512 bits       512 bits (same)
 *   Digest size   160 bits       256 bits
 *   Schedule      linear XOR     σ0, σ1 (rotation-heavy)
 *   Round funcs   Ch, Parity,    Ch, Maj (no parity)
 *                 Maj, Parity
 *   Constants     4 (sqrt)       64 (cube roots)
 *
 * The wider state and non-linear message schedule make collision attacks far
 * harder. SHA-1 was broken with 2^63 operations; SHA-256's birthday bound
 * requires 2^128 — a gap of 2^65 (37 billion billion times harder).
 *
 * The Merkle-Damgård Construction
 * ===============================
 * SHA-256 processes data in 512-bit (64-byte) blocks:
 *
 *   message ──► [pad] ──► block₀ ──► block₁ ──► ... ──► 32-byte digest
 *                              │           │
 *                      [H₀..H₇]──►compress──►compress──►...
 *
 * The "state" is eight 32-bit words (H₀..H₇). For each block, 64 rounds of
 * bit mixing fold the block into the state. The final state is the digest.
 *
 * JavaScript 32-bit Integer Caveat
 * =================================
 * JavaScript's bitwise operators (|, &, ^, ~, <<, >>) always return *signed*
 * 32-bit integers. SHA-256 needs *unsigned* 32-bit integers. The fix: append
 * `>>> 0` after any bitwise operation to coerce the result to unsigned 32-bit.
 *
 * FIPS 180-4 Test Vectors
 * =======================
 *   sha256("")    → e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
 *   sha256("abc") → ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
 */

export const VERSION = "0.1.0";

// ─── Initial Hash Values ────────────────────────────────────────────────────
//
// SHA-256 starts with eight 32-bit words as its initial state. These are the
// first 32 bits of the fractional parts of the square roots of the first 8
// primes (2, 3, 5, 7, 11, 13, 17, 19).
//
// Why square roots of primes? These are "nothing up my sleeve" numbers — their
// mathematical origin is transparent and verifiable, proving no backdoor is
// hidden in the choice. The fractional parts of irrational numbers (like √2)
// look random and have no special algebraic structure.
//
// Derivation example for H₀:
//   √2 = 1.41421356...
//   fractional part = 0.41421356...
//   × 2^32 = 1779033703.9520... → floor → 0x6A09E667
//
const INIT: [number, number, number, number, number, number, number, number] = [
  0x6a09e667, // √2
  0xbb67ae85, // √3
  0x3c6ef372, // √5
  0xa54ff53a, // √7
  0x510e527f, // √11
  0x9b05688c, // √13
  0x1f83d9ab, // √17
  0x5be0cd19, // √19
];

// ─── Round Constants ────────────────────────────────────────────────────────
//
// 64 constants, one per round. Each is the first 32 bits of the fractional
// parts of the cube roots of the first 64 primes (2, 3, 5, ..., 311).
//
// Same "nothing up my sleeve" principle as the initial hash values — cube
// roots of primes are irrational and their fractional parts look random.
//
// Derivation example for K₀:
//   ∛2 = 1.25992104...
//   fractional part = 0.25992104...
//   × 2^32 = 1116352408.8... → floor → 0x428A2F98
//
// Having 64 unique constants (vs SHA-1's 4) means each round has its own
// "flavor" of mixing, making the compression function harder to attack.
//
const K: readonly number[] = [
  0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
  0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
  0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
  0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
  0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
  0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
  0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
  0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
  0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
  0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
  0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
  0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
  0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5,
  0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
  0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
  0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

// ─── Helper: Right Rotation ─────────────────────────────────────────────────
//
// rotr(n, x) rotates x right by n bit positions within a 32-bit word. Bits
// that "fall off" the right end reappear on the left — unlike >>>, which
// fills with zeros.
//
// Example: n=2, x = 0b01101001 (8-bit for clarity)
//   Shift right: 01101001 >>> 2 = 00011010  (01 on the right is lost)
//   Rotate:      01101001 ROTR 2 = 01011010  (01 wraps to the left)
//
// Implementation: (x >>> n) | (x << (32 - n))
// The right half shifts down; the left half fills in what fell off the bottom.
// `>>> 0` converts the signed JS result to unsigned 32-bit.
//
function rotr(n: number, x: number): number {
  return ((x >>> n) | (x << (32 - n))) >>> 0;
}

// ─── Auxiliary Functions ────────────────────────────────────────────────────
//
// SHA-256 uses six auxiliary functions built from bitwise operations. They
// provide the non-linearity that makes the hash resistant to attacks.
//
// Ch(x, y, z) — "Choose"
// If bit of x is 1, choose the corresponding bit of y.
// If bit of x is 0, choose the corresponding bit of z.
// Truth table:
//   x=0, y=0, z=0 → 0    x=1, y=0, z=0 → 0
//   x=0, y=0, z=1 → 1    x=1, y=0, z=1 → 0
//   x=0, y=1, z=0 → 0    x=1, y=1, z=0 → 1
//   x=0, y=1, z=1 → 1    x=1, y=1, z=1 → 1
// Formula: (x AND y) XOR (NOT x AND z)
//
function ch(x: number, y: number, z: number): number {
  return ((x & y) ^ (~x & z)) >>> 0;
}

// Maj(x, y, z) — "Majority"
// Output is 1 if at least 2 of the 3 input bits are 1.
// Truth table:
//   0,0,0→0  0,0,1→0  0,1,0→0  0,1,1→1
//   1,0,0→0  1,0,1→1  1,1,0→1  1,1,1→1
// Formula: (x AND y) XOR (x AND z) XOR (y AND z)
//
function maj(x: number, y: number, z: number): number {
  return ((x & y) ^ (x & z) ^ (y & z)) >>> 0;
}

// Σ0(x) — "Big Sigma 0" — Used in the compression round on variable `a`.
// Combines three different rotations of x to mix bits from widely separated
// positions. The rotation amounts (2, 13, 22) are chosen so that every bit
// of x influences the output at multiple positions.
//   Σ0(x) = ROTR(2, x) XOR ROTR(13, x) XOR ROTR(22, x)
//
function bigSigma0(x: number): number {
  return (rotr(2, x) ^ rotr(13, x) ^ rotr(22, x)) >>> 0;
}

// Σ1(x) — "Big Sigma 1" — Used in the compression round on variable `e`.
// Same concept as Σ0 but with different rotation amounts (6, 11, 25).
//   Σ1(x) = ROTR(6, x) XOR ROTR(11, x) XOR ROTR(25, x)
//
function bigSigma1(x: number): number {
  return (rotr(6, x) ^ rotr(11, x) ^ rotr(25, x)) >>> 0;
}

// σ0(x) — "Small sigma 0" — Used in the message schedule expansion.
// Mixes a word with two rotations and one shift. The shift (SHR) makes this
// function non-invertible — you lose bits, preventing schedule reversal.
//   σ0(x) = ROTR(7, x) XOR ROTR(18, x) XOR SHR(3, x)
//
function smallSigma0(x: number): number {
  return (rotr(7, x) ^ rotr(18, x) ^ (x >>> 3)) >>> 0;
}

// σ1(x) — "Small sigma 1" — Also used in the message schedule expansion.
//   σ1(x) = ROTR(17, x) XOR ROTR(19, x) XOR SHR(10, x)
//
function smallSigma1(x: number): number {
  return (rotr(17, x) ^ rotr(19, x) ^ (x >>> 10)) >>> 0;
}

// ─── Padding ────────────────────────────────────────────────────────────────
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
// Why 56 mod 64? We need 8 bytes for the length field, and 56 + 8 = 64,
// which fills exactly one block. If adding 0x80 would push us past byte 56,
// we need an extra block of pure padding.
//
function pad(data: Uint8Array): Uint8Array {
  const byteLen = data.length;
  const bitLenHigh = Math.floor((byteLen * 8) / 0x100000000) >>> 0;
  const bitLenLow = (byteLen * 8) >>> 0;

  // How many zero bytes after 0x80?
  // Solve: (byteLen + 1 + zeroes) % 64 === 56
  const zeroes = ((56 - (byteLen + 1)) % 64 + 64) % 64;
  const padded = new Uint8Array(byteLen + 1 + zeroes + 8);

  padded.set(data, 0);
  padded[byteLen] = 0x80;

  // Zero bytes are already 0 from Uint8Array initialization.

  // Append 64-bit big-endian length (8 bytes at the end).
  const view = new DataView(padded.buffer);
  view.setUint32(byteLen + 1 + zeroes, bitLenHigh, false);
  view.setUint32(byteLen + 1 + zeroes + 4, bitLenLow, false);

  return padded;
}

// ─── Message Schedule ───────────────────────────────────────────────────────
//
// Each 64-byte block is parsed as 16 big-endian 32-bit words (W[0..15]),
// then expanded to 64 words using the σ0 and σ1 functions:
//
//   W[t] = σ1(W[t-2]) + W[t-7] + σ0(W[t-15]) + W[t-16]   for 16 ≤ t < 64
//
// Why is this better than SHA-1's schedule?
// SHA-1 just XORs and rotates — a linear operation. SHA-256 uses σ0 and σ1,
// which combine rotations AND a right shift (SHR). The shift destroys
// information, making the schedule non-linear and non-invertible. This means
// an attacker cannot easily craft message blocks that produce a desired
// schedule, which is key to collision resistance.
//
// The four-term recurrence (t-2, t-7, t-15, t-16) ensures each new word
// depends on words spread across the entire previous block, maximizing
// diffusion. Compare SHA-1 which uses t-3, t-8, t-14, t-16 — similar
// spread but with the linear XOR weakness.
//
function schedule(block: Uint8Array): Uint32Array {
  const W = new Uint32Array(64);
  const view = new DataView(block.buffer, block.byteOffset, block.byteLength);

  // Parse 16 big-endian 32-bit words from the block.
  for (let i = 0; i < 16; i++) {
    W[i] = view.getUint32(i * 4, false); // false = big-endian
  }

  // Expand to 64 words using the σ functions.
  for (let t = 16; t < 64; t++) {
    W[t] = (smallSigma1(W[t - 2]) + W[t - 7] + smallSigma0(W[t - 15]) + W[t - 16]) >>> 0;
  }

  return W;
}

// ─── Compression Function ───────────────────────────────────────────────────
//
// 64 rounds of mixing fold one 64-byte block into the eight-word state.
//
// Unlike SHA-1's four 20-round stages with different functions, SHA-256 uses
// the same two functions (Ch and Maj) for all 64 rounds, with a unique round
// constant K[t] providing per-round variation.
//
// Each round computes two temporary values:
//   T1 = h + Σ1(e) + Ch(e, f, g) + K[t] + W[t]
//   T2 = Σ0(a) + Maj(a, b, c)
//
// Then the working variables shift down, with T1+T2 entering at the top (a)
// and T1 being added to the middle (e):
//
//   h = g
//   g = f
//   f = e
//   e = d + T1        ← T1 enters the "choice" side
//   d = c
//   c = b
//   b = a
//   a = T1 + T2       ← Both temporaries combine at the top
//
// This creates two injection points (a and e) that are 4 positions apart,
// ensuring rapid diffusion across all 8 state words.
//
// Davies-Meyer feed-forward: after all 64 rounds, add the compressed output
// back to the input state. This makes the function non-invertible — even if
// you could reverse all 64 rounds, you'd still need to subtract the unknown
// input state.
//
type State8 = [number, number, number, number, number, number, number, number];

function compress(state: State8, block: Uint8Array): State8 {
  const W = schedule(block);
  const [h0, h1, h2, h3, h4, h5, h6, h7] = state;
  let a = h0, b = h1, c = h2, d = h3, e = h4, f = h5, g = h6, h = h7;

  for (let t = 0; t < 64; t++) {
    const t1 = (h + bigSigma1(e) + ch(e, f, g) + K[t] + W[t]) >>> 0;
    const t2 = (bigSigma0(a) + maj(a, b, c)) >>> 0;

    h = g;
    g = f;
    f = e;
    e = (d + t1) >>> 0;
    d = c;
    c = b;
    b = a;
    a = (t1 + t2) >>> 0;
  }

  return [
    (h0 + a) >>> 0,
    (h1 + b) >>> 0,
    (h2 + c) >>> 0,
    (h3 + d) >>> 0,
    (h4 + e) >>> 0,
    (h5 + f) >>> 0,
    (h6 + g) >>> 0,
    (h7 + h) >>> 0,
  ];
}

// ─── Finalization ───────────────────────────────────────────────────────────
//
// Convert the eight 32-bit state words to 32 bytes in big-endian order.
// Big-endian = most significant byte first (natural human-readable order).
//
function stateToBytes(state: State8): Uint8Array {
  const digest = new Uint8Array(32);
  const view = new DataView(digest.buffer);
  for (let i = 0; i < 8; i++) {
    view.setUint32(i * 4, state[i], false); // big-endian
  }
  return digest;
}

// ─── Public API ─────────────────────────────────────────────────────────────

/**
 * Compute the SHA-256 digest of data. Returns 32 bytes.
 *
 * This is the one-shot API: hash a complete message in a single call.
 *
 * @example
 * ```ts
 * const enc = new TextEncoder();
 * const digest = sha256(enc.encode("abc"));
 * console.log(toHex(digest));
 * // → "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
 * ```
 */
export function sha256(data: Uint8Array): Uint8Array {
  const padded = pad(data);
  let state: State8 = [...INIT];
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
 * Compute SHA-256 and return the 64-character lowercase hex string.
 *
 * @example
 * ```ts
 * const enc = new TextEncoder();
 * sha256Hex(enc.encode("abc"));
 * // → "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
 * ```
 */
export function sha256Hex(data: Uint8Array): string {
  return toHex(sha256(data));
}

/**
 * Streaming SHA-256 hasher that accepts data in multiple chunks.
 *
 * Useful when the full message is not available at once — for example when
 * reading a large file in chunks or hashing a network stream.
 *
 * The interface mirrors the Web Crypto / Node.js hash APIs:
 *
 * ```ts
 * const h = new SHA256Hasher();
 * h.update(enc.encode("ab"));
 * h.update(enc.encode("c"));
 * h.hexDigest();
 * // → "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
 * ```
 *
 * Multiple update() calls are equivalent to a single sha256(all_data).
 *
 * Implementation note:
 *   Data accumulates in a buffer. When the buffer reaches 64 bytes we compress
 *   the first block and discard it (keeping the state). On digest(), we pad
 *   whatever remains (using the TOTAL byte count) and compress the padding.
 */
export class SHA256Hasher {
  private _state: State8;
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
        this._state = compress(this._state, new Uint8Array(this._buffer));
        this._buffer = [];
      }
    }
    return this;
  }

  /**
   * Return the 32-byte digest of all data fed so far.
   *
   * Non-destructive: the internal state is not modified, so you can continue
   * calling update() after calling digest().
   */
  digest(): Uint8Array {
    // Build the padding tail using the TOTAL byte count (not buffer length).
    // Earlier blocks have already been compressed into _state; only the
    // partial buffer remains.
    const bitLenHigh = Math.floor((this._byteCount * 8) / 0x100000000) >>> 0;
    const bitLenLow = (this._byteCount * 8) >>> 0;

    const buf = this._buffer;
    const zeroes = ((56 - (buf.length + 1)) % 64 + 64) % 64;
    const tail = new Uint8Array(buf.length + 1 + zeroes + 8);
    tail.set(buf, 0);
    tail[buf.length] = 0x80;
    const view = new DataView(tail.buffer);
    view.setUint32(buf.length + 1 + zeroes, bitLenHigh, false);
    view.setUint32(buf.length + 1 + zeroes + 4, bitLenLow, false);

    // Compress the padding tail against a copy of the live state.
    let state: State8 = [...this._state];
    for (let i = 0; i < tail.length; i += 64) {
      state = compress(state, tail.subarray(i, i + 64));
    }
    return stateToBytes(state);
  }

  /** Return the 64-character hex string of the digest. */
  hexDigest(): string {
    return toHex(this.digest());
  }

  /**
   * Return an independent copy of this hasher.
   *
   * Useful for computing multiple hashes that share a common prefix:
   *
   * ```ts
   * const h = new SHA256Hasher().update(commonPrefix);
   * const h1 = h.copy(); h1.update(suffixA);
   * const h2 = h.copy(); h2.update(suffixB);
   * ```
   */
  copy(): SHA256Hasher {
    const other = new SHA256Hasher();
    other._state = [...this._state];
    other._buffer = [...this._buffer];
    other._byteCount = this._byteCount;
    return other;
  }
}
