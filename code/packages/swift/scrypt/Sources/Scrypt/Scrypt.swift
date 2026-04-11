// ============================================================================
// Scrypt.swift — scrypt Password-Based Key Derivation Function
// RFC 7914 — "The scrypt Password-Based Key Derivation Function"
// ============================================================================
//
// What Is scrypt?
// ===============
// scrypt was designed by Colin Percival in 2009 to resist hardware brute-force
// attacks. The key insight: as hardware gets cheaper and faster, attackers
// build GPU/ASIC farms to try billions of password guesses per second.
//
// PBKDF2 is CPU-hard: making it slower raises cost equally for defender and
// attacker. But an ASIC can run PBKDF2 with almost no memory — parallelizing
// cheaply. scrypt adds a MEMORY cost that cannot be parallelized away.
//
// Real-world use:
//   - Litecoin, Dogecoin: Proof-of-Work hashing
//   - macOS 10.14+ FileVault: disk encryption key derivation
//   - 1Password, Tarsnap, OpenBSD bioctl
//   - libsodium, NaCl, many TLS key derivation schemes
//
// Why Is Memory Hardness Valuable?
// =================================
// A modern GPU has 10,000+ cores but shared limited memory bandwidth. A
// memory-hard function forces every core to fetch large unpredictable data
// from memory. Memory is physical — you cannot parallelize "read 64 MB of
// RAM" without paying for 64 MB of RAM per parallel instance.
//
// scrypt forces the attacker to choose: few parallel instances (limited by
// memory budget) or many fast-but-memory-starved instances (wrong, since
// scrypt's ROMix step requires ALL N blocks to produce the correct output).
//
// Algorithm Overview (RFC 7914 §2)
// ==================================
//
//   scrypt(Password, Salt, N, r, p, dkLen):
//     1. B = PBKDF2-HMAC-SHA256(Password, Salt, 1, p * 128 * r)
//     2. For i in 0..p-1:
//          B[i] = ROMix(r, B[i], N)
//     3. DK = PBKDF2-HMAC-SHA256(Password, B, 1, dkLen)
//
// Parameters:
//   N   — CPU/memory cost: number of blocks in the large random-access table.
//         MUST be a power of 2. N=16384 (2^14) typical for interactive logins.
//   r   — Block size factor. Each block is 128*r bytes. r=8 typical.
//   p   — Parallelization factor. p=1 typical.
//   dkLen — Desired output key length in bytes.
//
// Memory Usage: O(N * r)
//   N=16384, r=8 → 16384 * 8 * 128 = 16 MB per parallel instance.
//
// The Three Core Primitives
// =========================
//
//   ┌──────────────────────────────────────────────────────────────┐
//   │  Salsa20/8 core  —  64-byte mixing function (8 rounds)      │
//   │  BlockMix(r)     —  mix 2r 64-byte blocks via Salsa20/8     │
//   │  ROMix(r, N)     —  build N-entry lookup table, smash memory │
//   └──────────────────────────────────────────────────────────────┘
//
//   PBKDF2 wraps the whole construction: password stretching in, out.
//
// ============================================================================

import Foundation
import PBKDF2

// ============================================================================
// MARK: - Error Types
// ============================================================================
//
// Validation mirrors the RFC 7914 constraints. Errors are thrown rather than
// crashing so callers can handle bad parameters gracefully.

/// Errors thrown by the `scrypt` key derivation function.
public enum ScryptError: Error, Equatable {
    /// N must be ≥ 2 and a power of 2. N=1 is not allowed (degenerate).
    case invalidN
    /// N > 2^20 is rejected to prevent accidental multi-gigabyte allocations.
    case nTooLarge
    /// r must be ≥ 1. r=0 produces 0-byte blocks.
    case invalidR
    /// p must be ≥ 1.
    case invalidP
    /// dkLen must be ≥ 1 and ≤ 2^20.
    case invalidKeyLength
    /// p * r must be ≤ 2^30 (overflow guard).
    case prTooLarge
    /// Propagated from the internal HMAC computation (should never occur in
    /// practice since scrypt passes controlled inputs to PBKDF2).
    case hmacError
}

// ============================================================================
// MARK: - Salsa20/8 Core
// ============================================================================
//
// Salsa20/8 is a stream cipher core designed by Daniel Bernstein (DJB) used
// here as a mixing function. It operates on a 512-bit (64-byte) block of
// 16 UInt32 words and applies 8 rounds of the "quarter round" operation.
//
// The "8" in Salsa20/8 means 8 rounds (4 column + 4 row) rather than the
// full 20 rounds of Salsa20 used for encryption. Fewer rounds is safe here
// because scrypt's ROMix table already provides cryptographic strength —
// Salsa20/8 just needs to be a good mixing function, not a full cipher.
//
// The Quarter Round (QR)
// ======================
// Salsa20's quarter round mixes 4 words (a, b, c, d) using rotations and
// XOR. The rotation amounts (7, 9, 13, 18) were chosen to maximize diffusion:
//
//   b ^= (a + d) <<< 7
//   c ^= (b + a) <<< 9
//   d ^= (c + b) <<< 13
//   a ^= (d + c) <<< 18
//
// "<<<" denotes left rotation (circular shift).
//
// Column Rounds vs Row Rounds
// ============================
// Salsa20 uses a 4x4 grid of 32-bit words. Column rounds mix each column;
// row rounds mix each (transposed) row. Together they ensure every input bit
// influences every output bit — the "avalanche effect."
//
//   Column indices: (0,4,8,12), (5,9,13,1), (10,14,2,6), (15,3,7,11)
//   Row indices:    (0,1,2,3),  (5,6,7,4),  (10,11,8,9), (15,12,13,14)
//
// Note that the row round indices appear transposed because Salsa20 stores
// its 4x4 matrix in column-major order in the flat word array.
//
// All arithmetic uses UInt32 wrapping operators (&+, <<, >>) so we never
// need to worry about overflow — it wraps silently by design.
//
// Finally, each output word is added back to the initial input (z[i]),
// making Salsa20/8 a bijection and preventing the function from collapsing
// to a fixed point.

/// Apply the Salsa20/8 mixing function to a 64-byte block.
///
/// - Parameter input: Exactly 64 bytes, interpreted as 16 little-endian UInt32 words.
/// - Returns: 64 bytes — the mixed output.
///
/// Example (from RFC 7914 §3):
/// ```
/// Input:  7e 87 9a 21 4f 3e c9 86 7c a9 40 e6 41 71 2e 37 ...
/// Output: a4 1f 85 9c 66 08 cc 99 3b 81 ca cb 02 0c ef 05 ...
/// ```
private func salsa20_8(_ input: [UInt8]) -> [UInt8] {
    precondition(input.count == 64, "Salsa20/8 requires exactly 64 bytes")

    // ── Decode 16 little-endian UInt32 words ──────────────────────────────
    // Little-endian means the least significant byte is first in memory.
    // For word i, bytes at offsets [4i, 4i+1, 4i+2, 4i+3] map to bits
    // [0..7, 8..15, 16..23, 24..31] of the 32-bit word.
    var x = [UInt32](repeating: 0, count: 16)
    for i in 0..<16 {
        let o = i * 4
        x[i] = UInt32(input[o])
             | (UInt32(input[o + 1]) << 8)
             | (UInt32(input[o + 2]) << 16)
             | (UInt32(input[o + 3]) << 24)
    }

    // Save the initial state — we add it back at the end (ARX design).
    let z = x

    // ── Rotation helper ───────────────────────────────────────────────────
    // Left rotation by n bits. Swift's UInt32 shift is non-wrapping, so we
    // use both a left shift and a right shift to simulate rotation.
    func rotl(_ v: UInt32, _ n: UInt32) -> UInt32 { (v << n) | (v >> (32 - n)) }

    // ── 8 rounds = 4 iterations of (column round + row round) ────────────
    // Each iteration applies one column round (mixing the 4 columns of the
    // 4x4 word matrix) then one row round (mixing the 4 rows).
    //
    // The indices below are the flat-array positions for each column/row:
    //
    //   Matrix layout (column-major indexing):
    //    0  4  8 12
    //    1  5  9 13
    //    2  6 10 14
    //    3  7 11 15
    //
    //   Column 0: indices 0, 4,  8, 12
    //   Column 1: indices 5, 9, 13,  1
    //   Column 2: indices 10,14,  2,  6
    //   Column 3: indices 15, 3,  7, 11
    //
    //   Row 0: indices 0,  1,  2,  3
    //   Row 1: indices 5,  6,  7,  4
    //   Row 2: indices 10, 11,  8,  9
    //   Row 3: indices 15, 12, 13, 14
    for _ in 0..<4 {
        // Column rounds
        x[4]  ^= rotl(x[0]  &+ x[12], 7);  x[8]  ^= rotl(x[4]  &+ x[0],  9)
        x[12] ^= rotl(x[8]  &+ x[4],  13); x[0]  ^= rotl(x[12] &+ x[8],  18)
        x[9]  ^= rotl(x[5]  &+ x[1],  7);  x[13] ^= rotl(x[9]  &+ x[5],  9)
        x[1]  ^= rotl(x[13] &+ x[9],  13); x[5]  ^= rotl(x[1]  &+ x[13], 18)
        x[14] ^= rotl(x[10] &+ x[6],  7);  x[2]  ^= rotl(x[14] &+ x[10], 9)
        x[6]  ^= rotl(x[2]  &+ x[14], 13); x[10] ^= rotl(x[6]  &+ x[2],  18)
        x[3]  ^= rotl(x[15] &+ x[11], 7);  x[7]  ^= rotl(x[3]  &+ x[15], 9)
        x[11] ^= rotl(x[7]  &+ x[3],  13); x[15] ^= rotl(x[11] &+ x[7],  18)
        // Row rounds
        x[1]  ^= rotl(x[0]  &+ x[3],  7);  x[2]  ^= rotl(x[1]  &+ x[0],  9)
        x[3]  ^= rotl(x[2]  &+ x[1],  13); x[0]  ^= rotl(x[3]  &+ x[2],  18)
        x[6]  ^= rotl(x[5]  &+ x[4],  7);  x[7]  ^= rotl(x[6]  &+ x[5],  9)
        x[4]  ^= rotl(x[7]  &+ x[6],  13); x[5]  ^= rotl(x[4]  &+ x[7],  18)
        x[11] ^= rotl(x[10] &+ x[9],  7);  x[8]  ^= rotl(x[11] &+ x[10], 9)
        x[9]  ^= rotl(x[8]  &+ x[11], 13); x[10] ^= rotl(x[9]  &+ x[8],  18)
        x[12] ^= rotl(x[15] &+ x[14], 7);  x[13] ^= rotl(x[12] &+ x[15], 9)
        x[14] ^= rotl(x[13] &+ x[12], 13); x[15] ^= rotl(x[14] &+ x[13], 18)
    }

    // ── Encode output: add initial state, store little-endian ─────────────
    // The addition z[i] is the "add-back" step that makes Salsa20/8 invertible
    // and prevents the function from zeroing all words if input is all-zeros.
    var out = [UInt8](repeating: 0, count: 64)
    for i in 0..<16 {
        let word = x[i] &+ z[i]     // wrapping add — never overflows UInt32
        out[i * 4]     = UInt8(word & 0xFF)
        out[i * 4 + 1] = UInt8((word >> 8)  & 0xFF)
        out[i * 4 + 2] = UInt8((word >> 16) & 0xFF)
        out[i * 4 + 3] = UInt8((word >> 24) & 0xFF)
    }
    return out
}

// ============================================================================
// MARK: - BlockMix
// ============================================================================
//
// BlockMix(r) mixes a sequence of 2r 64-byte blocks by repeatedly applying
// Salsa20/8. The "r" parameter doubles the block size to increase the minimum
// memory footprint per ROMix table entry.
//
// Algorithm (RFC 7914 §4):
//   Input:  B = [B_0, B_1, ..., B_{2r-1}]  (each B_i is 64 bytes)
//   x = B_{2r-1}                            (start with the last block)
//   For i = 0 to 2r-1:
//     x = Salsa20/8(x XOR B_i)
//     y[i] = x
//   Output: [y_0, y_2, y_4, ..., y_{2r-2}, y_1, y_3, ..., y_{2r-1}]
//           (even-indexed blocks first, then odd-indexed)
//
// The interleaved output ordering is NOT a mistake. It means that each call
// to BlockMix touches all blocks in an unpredictable order when used inside
// ROMix, maximizing memory access randomness.
//
// Memory: O(r) — only x and y[i] are stored; the input B is read once.

/// Mix a sequence of 2r 64-byte blocks using Salsa20/8.
///
/// - Parameters:
///   - blocks: Array of 2*r slices, each exactly 64 bytes.
///   - r: Block size factor. Output has the same 2*r blocks, interleaved.
/// - Returns: Mixed blocks in interleaved order (even-indexed then odd-indexed).
private func blockMix(_ blocks: [[UInt8]], r: Int) -> [[UInt8]] {
    let twoR = 2 * r
    // Start x as the last block
    var x = blocks[twoR - 1]
    var y = [[UInt8]](repeating: [UInt8](repeating: 0, count: 64), count: twoR)

    for i in 0..<twoR {
        // XOR x with the i-th input block, then apply Salsa20/8
        x = salsa20_8(xor64(x, blocks[i]))
        y[i] = x
    }

    // Interleave: even-indexed outputs first, then odd-indexed
    var out = [[UInt8]]()
    out.reserveCapacity(twoR)
    for i in 0..<r { out.append(y[2 * i]) }       // y_0, y_2, y_4, ...
    for i in 0..<r { out.append(y[2 * i + 1]) }   // y_1, y_3, y_5, ...
    return out
}

// ============================================================================
// MARK: - ROMix
// ============================================================================
//
// ROMix is the memory-hard step. It builds a table V of N blocks (each a
// BlockMix input of size 128*r bytes), then uses pseudo-random lookups into
// that table to mix the input block X.
//
// Why is it memory-hard?
//   Building V: N sequential BlockMix calls. Each depends on the previous, so
//   V cannot be computed lazily — all N entries must be in memory to answer
//   any single lookup in the second phase.
//
//   Smashing phase: N iterations, each looking up a pseudo-random V[j] where
//   j = Integerify(X) mod N. An adversary cannot predict j without computing
//   ALL of V. Skipping or discarding entries causes wrong output.
//
// Algorithm (RFC 7914 §5):
//   X = B
//   For i = 0 to N-1:
//     V[i] = X
//     X = BlockMix(X)
//   For i = 0 to N-1:
//     j = Integerify(X) mod N
//     X = BlockMix(X XOR V[j])
//   Return X
//
// Integerify reads the last 64-byte block's first 8 bytes as a little-endian
// UInt64. Since N is a power of 2, "mod N" is a fast bitwise AND.
//
// Memory: O(N * r) — the V table is N entries of 2r * 64 bytes each.

/// XOR two 64-byte blocks element-wise.
private func xor64(_ a: [UInt8], _ b: [UInt8]) -> [UInt8] {
    var out = [UInt8](repeating: 0, count: 64)
    for i in 0..<64 { out[i] = a[i] ^ b[i] }
    return out
}

/// XOR two arrays of 2r blocks element-wise.
private func xorBlocks(_ a: [[UInt8]], _ b: [[UInt8]]) -> [[UInt8]] {
    return zip(a, b).map { xor64($0, $1) }
}

/// Extract a little-endian UInt64 from the first 8 bytes of the last block.
///
/// This is "Integerify" from RFC 7914 §4. The result mod N gives the table
/// index for the random-access lookup in ROMix.
private func integerify(_ x: [[UInt8]]) -> UInt64 {
    let last = x[x.count - 1]
    var val: UInt64 = 0
    for i in 0..<8 { val |= UInt64(last[i]) << (i * 8) }
    return val
}

/// Apply the ROMix memory-hard function to a single scrypt block.
///
/// - Parameters:
///   - bBytes: Flat byte array of size 128*r (a single "p-block").
///   - n: Number of entries in the random-access table (power of 2).
///   - r: Block size factor.
/// - Returns: Mixed flat byte array of size 128*r.
private func roMix(_ bBytes: [UInt8], n: Int, r: Int) -> [UInt8] {
    let twoR = 2 * r

    // Decode flat bytes into 2r blocks of 64 bytes each
    var x = (0..<twoR).map { i in Array(bBytes[i * 64 ..< (i + 1) * 64]) }

    // ── Phase 1: Build the lookup table V ────────────────────────────────
    // V[i] = X before the i-th BlockMix. We must store ALL N entries.
    // reserveCapacity prevents repeated reallocations.
    var v = [[[UInt8]]]()
    v.reserveCapacity(n)
    for _ in 0..<n {
        v.append(x)
        x = blockMix(x, r: r)
    }

    // ── Phase 2: Pseudo-random lookups ────────────────────────────────────
    // Each iteration reads a pseudo-random V[j] and XORs it with X before
    // applying BlockMix. An adversary who dropped entries from V will produce
    // the wrong answer — forcing them to keep all N entries resident in memory.
    for _ in 0..<n {
        let j = Int(integerify(x) & UInt64(n - 1))   // mod N, N is power of 2
        x = blockMix(xorBlocks(x, v[j]), r: r)
    }

    return x.flatMap { $0 }
}

// ============================================================================
// MARK: - Public API
// ============================================================================
//
// The public entry points are `scrypt` (returns [UInt8]) and `scryptHex`
// (returns a lowercase hex string). All parameter validation happens in
// `scrypt`; `scryptHex` is a thin wrapper.
//
// Parameter Guidance (OWASP / Colin Percival's recommendations):
//   Interactive login  — N=16384 (2^14), r=8, p=1  (~16 MB, ~0.1s)
//   File encryption    — N=1048576 (2^20), r=8, p=1 (~1 GB, ~5s)
//   Embedded devices   — N=1024, r=8, p=1 (~1 MB, ~0.01s)
//
// The dkLen upper bound of 2^20 is a practical safety cap. RFC 7914 permits
// larger values, but dkLen > 1 MB indicates a likely programmer error.

/// Derive a key using the scrypt algorithm (RFC 7914).
///
/// - Parameters:
///   - password: The password or passphrase to hash. May be empty.
///   - salt: A unique random value per credential (≥16 bytes recommended).
///   - n: CPU/memory cost factor. Must be ≥ 2 and a power of 2. Must be ≤ 2^20.
///   - r: Block size factor. Must be ≥ 1. Each block is 128*r bytes.
///   - p: Parallelization factor. Must be ≥ 1. p*r must be ≤ 2^30.
///   - dkLen: Desired output length in bytes. Must be ≥ 1 and ≤ 2^20.
/// - Returns: Derived key as a byte array of length `dkLen`.
/// - Throws: `ScryptError` if any parameter is invalid.
///
/// Example — RFC 7914 test vector 2:
/// ```swift
/// let dk = try scrypt(
///     password: Array("password".utf8),
///     salt: Array("NaCl".utf8),
///     n: 1024, r: 8, p: 16, dkLen: 64
/// )
/// ```
public func scrypt(
    password: [UInt8], salt: [UInt8], n: Int, r: Int, p: Int, dkLen: Int
) throws -> [UInt8] {
    // ── Parameter validation (RFC 7914 §2) ───────────────────────────────
    // N must be a power of 2 and at least 2. The power-of-2 check uses the
    // bit trick: a power of 2 has exactly one bit set, so (N & (N-1)) == 0.
    guard n >= 2 && (n & (n - 1)) == 0 else { throw ScryptError.invalidN }
    // Cap N to prevent accidental gigabyte allocations. 2^20 = 1,048,576.
    guard n <= (1 << 20)                 else { throw ScryptError.nTooLarge }
    guard r >= 1                         else { throw ScryptError.invalidR }
    guard p >= 1                         else { throw ScryptError.invalidP }
    guard dkLen >= 1 && dkLen <= (1 << 20) else { throw ScryptError.invalidKeyLength }
    // Overflow guard: p*r is used in memory size calculations. Capping at 2^30
    // ensures the intermediate byte count (p * 128 * r) fits in a 64-bit Int.
    guard p * r <= (1 << 30)             else { throw ScryptError.prTooLarge }

    // ── Step 1: Expand password + salt into p parallel blocks ─────────────
    // B is p blocks of 128*r bytes each. scrypt's PBKDF2 always uses 1
    // iteration — the memory-hardness comes from ROMix, not PBKDF2 iteration.
    //
    // allowEmptyPassword: true because RFC 7914 test vector 1 uses password=""
    // and scrypt is a protocol-level construct that allows it by specification.
    // The PBKDF2 package's allowEmptyPassword flag exists precisely for this.
    let bLen = p * 128 * r
    var b = Array(try pbkdf2HmacSHA256(
        password: Data(password), salt: Data(salt), iterations: 1, keyLength: bLen,
        allowEmptyPassword: true
    ))

    // ── Step 2: Apply ROMix to each parallel block ────────────────────────
    // Each block is processed independently and can be parallelized (hence
    // the 'p' parameter). Here we process them sequentially for simplicity.
    for i in 0..<p {
        let start = i * 128 * r
        let end   = start + 128 * r
        let chunk = Array(b[start..<end])
        let mixed = roMix(chunk, n: n, r: r)
        b.replaceSubrange(start..<end, with: mixed)
    }

    // ── Step 3: Compress mixed blocks into the final derived key ─────────
    // The mixed B is now used as the salt for a second PBKDF2 call, which
    // compresses it down to dkLen bytes. This means the attacker cannot
    // bypass ROMix — any error in ROMix produces a completely wrong B, and
    // thus a completely wrong final key.
    return Array(try pbkdf2HmacSHA256(
        password: Data(password), salt: Data(b), iterations: 1, keyLength: dkLen,
        allowEmptyPassword: true
    ))
}

/// Like `scrypt` but returns the derived key as a lowercase hex string.
///
/// Useful for storing or comparing password hashes in text-based formats.
///
/// Example — RFC 7914 test vector 1:
/// ```swift
/// let hex = try scryptHex(password: [], salt: [], n: 16, r: 1, p: 1, dkLen: 64)
/// // → "77d6576238657b203b19ca42c18a0497..."
/// ```
public func scryptHex(
    password: [UInt8], salt: [UInt8], n: Int, r: Int, p: Int, dkLen: Int
) throws -> String {
    let dk = try scrypt(password: password, salt: salt, n: n, r: r, p: p, dkLen: dkLen)
    return dk.map { String(format: "%02x", $0) }.joined()
}
