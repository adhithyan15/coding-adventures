// DES.swift
// Part of coding-adventures — an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// Data Encryption Standard (DES) — FIPS 46-3
// ============================================================================
//
// DES is a symmetric-key block cipher designed by IBM and adopted as a
// federal standard by NIST in 1977 (FIPS 46). It operates on 64-bit (8-byte)
// blocks with a 56-bit effective key (64 bits with 8 parity bits ignored).
//
// Though DES is now obsolete for standalone use (56-bit key space was broken
// by brute force in 1999), it remains a foundational algorithm: its Feistel
// network structure, S-box design, and key schedule influenced virtually every
// symmetric cipher that followed.
//
// Architecture: Feistel Network
// ==============================
// DES is a 16-round Feistel cipher. In a Feistel network:
//   - The 64-bit block is split into left (L) and right (R) halves.
//   - Each round: L' = R,  R' = L XOR F(R, subkey)
//   - After 16 rounds, swap and concatenate: output = R16 || L16
//
// This "swap then concatenate" means decryption uses the same structure —
// just apply the 16 subkeys in reverse order. No separate decrypt circuit needed.
//
// The Round Function F(R, K):
// ===========================
//   1. Expand R from 32 bits to 48 bits (E expansion table)
//   2. XOR with 48-bit subkey K
//   3. Split into 8 groups of 6 bits; each group indexes into one of 8 S-boxes
//      (each S-box maps 6 bits → 4 bits, producing 32 bits total)
//   4. Permute the 32 bits through the P permutation
//
// S-boxes: The cryptographic heart. Each S-box is a 4×16 nibble table:
//   - The outermost 2 bits of the 6-bit input select the row (0-3)
//   - The inner 4 bits select the column (0-15)
//   - The table entry (0-15) is the 4-bit output
//
// Key Schedule: PC-1 + PC-2
// ==========================
//   1. Apply PC-1 to the 64-bit key → 56-bit key (drops parity bits)
//   2. Split into C (28 bits) and D (28 bits)
//   3. For each round i: left-rotate C and D by SHIFTS[i] positions
//   4. Apply PC-2 to (C || D) → 48-bit subkey Ki
//
// 3DES (TDEA): Triple Encryption
// ================================
// NIST SP 800-67 defines TDEA as E(K1, D(K2, E(K3, P))):
//   Step 1: Encrypt with K3
//   Step 2: Decrypt with K2
//   Step 3: Encrypt with K1
//
// When K1=K2=K3=K: E(K, D(K, E(K, P))) = E(K, P) — backward compatible.
// The EDE (Encrypt-Decrypt-Encrypt) ordering was chosen for this property.
//
// ECB Mode + PKCS#7 Padding
// ==========================
// Electronic Codebook (ECB) mode encrypts each 64-bit block independently.
// PKCS#7 padding ensures the message is a multiple of 8 bytes:
//   - If the last block has k bytes of data, append (8-k) bytes each = (8-k)
//   - If already aligned: append a full 8-byte block of value 0x08
//
// ============================================================================

import Foundation

// ============================================================================
// MARK: - IP — Initial Permutation (FIPS 46-3 Table 6)
// ============================================================================
//
// IP rearranges the 64 input bits. Each entry is a 1-based bit position in
// the input. DES uses big-endian bit numbering (bit 1 = MSB of byte 0).
//
// IP exists for hardware efficiency (routing wires on a chip), not
// for cryptographic strength. IP and FP cancel each other out when composed.

private let IP: [Int] = [
    58, 50, 42, 34, 26, 18, 10, 2,
    60, 52, 44, 36, 28, 20, 12, 4,
    62, 54, 46, 38, 30, 22, 14, 6,
    64, 56, 48, 40, 32, 24, 16, 8,
    57, 49, 41, 33, 25, 17,  9, 1,
    59, 51, 43, 35, 27, 19, 11, 3,
    61, 53, 45, 37, 29, 21, 13, 5,
    63, 55, 47, 39, 31, 23, 15, 7
]

// ============================================================================
// MARK: - FP — Final Permutation (FIPS 46-3 Table 7)
// ============================================================================
//
// FP is the inverse of IP: applying IP then FP returns the original input.
// Also called IP^{-1} in the literature.

private let FP: [Int] = [
    40,  8, 48, 16, 56, 24, 64, 32,
    39,  7, 47, 15, 55, 23, 63, 31,
    38,  6, 46, 14, 54, 22, 62, 30,
    37,  5, 45, 13, 53, 21, 61, 29,
    36,  4, 44, 12, 52, 20, 60, 28,
    35,  3, 43, 11, 51, 19, 59, 27,
    34,  2, 42, 10, 50, 18, 58, 26,
    33,  1, 41,  9, 49, 17, 57, 25
]

// ============================================================================
// MARK: - E — Expansion Permutation (FIPS 46-3 Table 8)
// ============================================================================
//
// E expands the 32-bit right half to 48 bits by duplicating 16 of the bits.
// The duplicated bits appear at the boundaries between 6-bit groups, creating
// "overlap" that makes adjacent S-box outputs depend on the same input bits.
// This propagates changes across S-box boundaries (diffusion).

private let E: [Int] = [
    32,  1,  2,  3,  4,  5,
     4,  5,  6,  7,  8,  9,
     8,  9, 10, 11, 12, 13,
    12, 13, 14, 15, 16, 17,
    16, 17, 18, 19, 20, 21,
    20, 21, 22, 23, 24, 25,
    24, 25, 26, 27, 28, 29,
    28, 29, 30, 31, 32,  1
]

// ============================================================================
// MARK: - P — Post-S-box Permutation (FIPS 46-3 Table 9)
// ============================================================================
//
// P permutes the 32-bit output of the S-box stage. It is designed so that
// each output bit of each S-box feeds into different S-boxes in the next round,
// maximizing diffusion across rounds.

private let P_TABLE: [Int] = [
    16,  7, 20, 21, 29, 12, 28, 17,
     1, 15, 23, 26,  5, 18, 31, 10,
     2,  8, 24, 14, 32, 27,  3,  9,
    19, 13, 30,  6, 22, 11,  4, 25
]

// ============================================================================
// MARK: - PC1 — Permuted Choice 1 (FIPS 46-3 Table 3)
// ============================================================================
//
// PC-1 selects 56 bits from the 64-bit key, dropping the 8 parity bits
// (positions 8, 16, 24, 32, 40, 48, 56, 64). The 56 selected bits are
// split into two 28-bit halves C and D for the key schedule.

private let PC1: [Int] = [
    57, 49, 41, 33, 25, 17,  9,
     1, 58, 50, 42, 34, 26, 18,
    10,  2, 59, 51, 43, 35, 27,
    19, 11,  3, 60, 52, 44, 36,
    63, 55, 47, 39, 31, 23, 15,
     7, 62, 54, 46, 38, 30, 22,
    14,  6, 61, 53, 45, 37, 29,
    21, 13,  5, 28, 20, 12,  4
]

// ============================================================================
// MARK: - PC2 — Permuted Choice 2 (FIPS 46-3 Table 4)
// ============================================================================
//
// PC-2 selects 48 bits from the 56-bit (C || D) to form each round subkey.
// The 8 dropped bits change each round (because C and D rotate), so every
// key bit eventually participates in multiple subkeys.

private let PC2: [Int] = [
    14, 17, 11, 24,  1,  5,
     3, 28, 15,  6, 21, 10,
    23, 19, 12,  4, 26,  8,
    16,  7, 27, 20, 13,  2,
    41, 52, 31, 37, 47, 55,
    30, 40, 51, 45, 33, 48,
    44, 49, 39, 56, 34, 53,
    46, 42, 50, 36, 29, 32
]

// ============================================================================
// MARK: - SHIFTS — Key Schedule Rotation Amounts (FIPS 46-3)
// ============================================================================
//
// Each round rotates C and D left by this many positions. Rounds 1, 2, 9, 16
// rotate by 1; all others rotate by 2. Total rotation over 16 rounds = 28,
// which wraps the 28-bit halves back to the original position.

private let SHIFTS: [Int] = [1, 1, 2, 2, 2, 2, 2, 2, 1, 2, 2, 2, 2, 2, 2, 1]

// ============================================================================
// MARK: - SBOXES — Eight 4×16 Substitution Boxes (FIPS 46-3 Tables 10-17)
// ============================================================================
//
// Each S-box maps a 6-bit input to a 4-bit output. The mapping is non-linear,
// providing confusion (making the relationship between key and ciphertext
// complex). Non-linearity is the primary source of DES's security.
//
// S-box indexing (6-bit input `b5 b4 b3 b2 b1 b0`):
//   row = (b5 << 1) | b0        — outer bits
//   col = (b4 << 3) | (b3 << 2) | (b2 << 1) | b1  — inner bits
//
// The 8 S-boxes are distinct and were chosen by IBM/NSA to maximize avalanche
// effect and resistance to differential and linear cryptanalysis.

private let SBOXES: [[UInt8]] = [
    // S1
    [14,  4, 13,  1,  2, 15, 11,  8,  3, 10,  6, 12,  5,  9,  0,  7,
      0, 15,  7,  4, 14,  2, 13,  1, 10,  6, 12, 11,  9,  5,  3,  8,
      4,  1, 14,  8, 13,  6,  2, 11, 15, 12,  9,  7,  3, 10,  5,  0,
     15, 12,  8,  2,  4,  9,  1,  7,  5, 11,  3, 14, 10,  0,  6, 13],
    // S2
    [15,  1,  8, 14,  6, 11,  3,  4,  9,  7,  2, 13, 12,  0,  5, 10,
      3, 13,  4,  7, 15,  2,  8, 14, 12,  0,  1, 10,  6,  9, 11,  5,
      0, 14,  7, 11, 10,  4, 13,  1,  5,  8, 12,  6,  9,  3,  2, 15,
     13,  8, 10,  1,  3, 15,  4,  2, 11,  6,  7, 12,  0,  5, 14,  9],
    // S3
    [10,  0,  9, 14,  6,  3, 15,  5,  1, 13, 12,  7, 11,  4,  2,  8,
     13,  7,  0,  9,  3,  4,  6, 10,  2,  8,  5, 14, 12, 11, 15,  1,
     13,  6,  4,  9,  8, 15,  3,  0, 11,  1,  2, 12,  5, 10, 14,  7,
      1, 10, 13,  0,  6,  9,  8,  7,  4, 15, 14,  3, 11,  5,  2, 12],
    // S4
    [ 7, 13, 14,  3,  0,  6,  9, 10,  1,  2,  8,  5, 11, 12,  4, 15,
     13,  8, 11,  5,  6, 15,  0,  3,  4,  7,  2, 12,  1, 10, 14,  9,
     10,  6,  9,  0, 12, 11,  7, 13, 15,  1,  3, 14,  5,  2,  8,  4,
      3, 15,  0,  6, 10,  1, 13,  8,  9,  4,  5, 11, 12,  7,  2, 14],
    // S5
    [ 2, 12,  4,  1,  7, 10, 11,  6,  8,  5,  3, 15, 13,  0, 14,  9,
     14, 11,  2, 12,  4,  7, 13,  1,  5,  0, 15, 10,  3,  9,  8,  6,
      4,  2,  1, 11, 10, 13,  7,  8, 15,  9, 12,  5,  6,  3,  0, 14,
     11,  8, 12,  7,  1, 14,  2, 13,  6, 15,  0,  9, 10,  4,  5,  3],
    // S6
    [12,  1, 10, 15,  9,  2,  6,  8,  0, 13,  3,  4, 14,  7,  5, 11,
     10, 15,  4,  2,  7, 12,  9,  5,  6,  1, 13, 14,  0, 11,  3,  8,
      9, 14, 15,  5,  2,  8, 12,  3,  7,  0,  4, 10,  1, 13, 11,  6,
      4,  3,  2, 12,  9,  5, 15, 10, 11, 14,  1,  7,  6,  0,  8, 13],
    // S7
    [ 4, 11,  2, 14, 15,  0,  8, 13,  3, 12,  9,  7,  5, 10,  6,  1,
     13,  0, 11,  7,  4,  9,  1, 10, 14,  3,  5, 12,  2, 15,  8,  6,
      1,  4, 11, 13, 12,  3,  7, 14, 10, 15,  6,  8,  0,  5,  9,  2,
      6, 11, 13,  8,  1,  4, 10,  7,  9,  5,  0, 15, 14,  2,  3, 12],
    // S8
    [13,  2,  8,  4,  6, 15, 11,  1, 10,  9,  3, 14,  5,  0, 12,  7,
      1, 15, 13,  8, 10,  3,  7,  4, 12,  5,  6, 11,  0, 14,  9,  2,
      7, 11,  4,  1,  9, 12, 14,  2,  0,  6, 10, 13, 15,  3,  5,  8,
      2,  1, 14,  7,  4, 10,  8, 13, 15, 12,  9,  0,  3,  5,  6, 11]
]

// ============================================================================
// MARK: - Bit manipulation helpers
// ============================================================================

/// Extract a single bit from a byte array (1-based, MSB first).
///
/// DES uses 1-based bit numbering where bit 1 is the most significant bit
/// of the first byte. To extract bit n from bytes:
///   - Which byte: (n-1) / 8
///   - Which bit within that byte: 7 - ((n-1) % 8)  (MSB = bit 7)
private func getBit(_ bytes: [UInt8], _ pos: Int) -> Int {
    let byteIdx = (pos - 1) / 8
    let bitIdx  = 7 - ((pos - 1) % 8)
    return Int((bytes[byteIdx] >> bitIdx) & 1)
}

/// Apply a permutation table to a byte array, producing a new bit string.
///
/// - Parameters:
///   - bytes: Input byte array (source bits).
///   - table: 1-based source bit positions; output bit i comes from table[i-1].
///   - outBits: Number of output bits (used to determine output array size).
/// - Returns: Byte array containing the permuted bits, MSB first.
private func permute(_ bytes: [UInt8], _ table: [Int], outBits: Int) -> [UInt8] {
    let outBytes = (outBits + 7) / 8
    var result = [UInt8](repeating: 0, count: outBytes)
    for (i, srcBit) in table.enumerated() {
        let bit = getBit(bytes, srcBit)
        if bit == 1 {
            let byteIdx = i / 8
            let bitIdx  = 7 - (i % 8)
            result[byteIdx] |= (1 << bitIdx)
        }
    }
    return result
}

/// Left-rotate a 28-bit value (stored in the low 28 bits of a UInt32).
///
/// The key schedule rotates two 28-bit halves (C and D) each round.
/// We store them as UInt32 and mask to 28 bits after rotation.
private func rotL28(_ val: UInt32, _ n: Int) -> UInt32 {
    return ((val << n) | (val >> (28 - n))) & 0x0FFFFFFF
}

// ============================================================================
// MARK: - Key Schedule: expand_key
// ============================================================================
//
// Returns the 16 48-bit subkeys as an array of 6-byte arrays.
// Decryption uses the same function with reversed output.

/// Expand an 8-byte DES key into 16 round subkeys.
///
/// - Parameter key: Exactly 8 bytes. Bits 8, 16, ..., 64 are parity bits
///   and are ignored by PC-1.
/// - Returns: Array of 16 subkeys, each 6 bytes (48 bits). Subkey 0 is for
///   round 1 (used first during encryption).
public func expandKey(_ key: [UInt8]) -> [[UInt8]] {
    precondition(key.count == 8, "DES key must be exactly 8 bytes")

    // PC-1: reduce 64-bit key to 56 bits, split into C (28 bits) and D (28 bits)
    let key56 = permute(key, PC1, outBits: 56)

    // Pack the 56 bits into two 28-bit halves stored as UInt32
    // C = high 28 bits (bits 0..27 of key56)
    // D = low  28 bits (bits 28..55 of key56)
    var c: UInt32 = 0
    var d: UInt32 = 0

    for i in 0..<28 {
        let byteIdx = i / 8
        let bitIdx  = 7 - (i % 8)
        let bit = (key56[byteIdx] >> bitIdx) & 1
        c = (c << 1) | UInt32(bit)
    }
    for i in 0..<28 {
        let byteIdx = (28 + i) / 8
        let bitIdx  = 7 - ((28 + i) % 8)
        let bit = (key56[byteIdx] >> bitIdx) & 1
        d = (d << 1) | UInt32(bit)
    }

    // Generate 16 subkeys
    var subkeys: [[UInt8]] = []
    for round in 0..<16 {
        // Rotate C and D
        c = rotL28(c, SHIFTS[round])
        d = rotL28(d, SHIFTS[round])

        // Reassemble C || D into a 56-bit byte array for PC-2
        var cd = [UInt8](repeating: 0, count: 7)
        for i in 0..<28 {
            let bit = (c >> (27 - i)) & 1
            let byteIdx = i / 8
            let bitIdx  = 7 - (i % 8)
            if bit == 1 { cd[byteIdx] |= (1 << bitIdx) }
        }
        for i in 0..<28 {
            let bit = (d >> (27 - i)) & 1
            let byteIdx = (28 + i) / 8
            let bitIdx  = 7 - ((28 + i) % 8)
            if bit == 1 { cd[byteIdx] |= (1 << bitIdx) }
        }

        // PC-2: select 48 bits from the 56-bit C || D
        subkeys.append(permute(cd, PC2, outBits: 48))
    }
    return subkeys
}

// ============================================================================
// MARK: - DES Round Function F(R, K)
// ============================================================================
//
// Applies one DES round function to the 32-bit right half R using subkey K.
// Returns a 32-bit result as 4 bytes.

private func desF(_ r: [UInt8], _ k: [UInt8]) -> [UInt8] {
    // Step 1: Expand R from 32 to 48 bits via E permutation
    let expanded = permute(r, E, outBits: 48)  // 6 bytes

    // Step 2: XOR expanded R with the 48-bit subkey
    var xored = [UInt8](repeating: 0, count: 6)
    for i in 0..<6 {
        xored[i] = expanded[i] ^ k[i]
    }

    // Step 3: S-box substitution — 8 groups of 6 bits → 8 groups of 4 bits
    // Process each 6-bit group by extracting bits from the 48-bit xored value.
    var sOutput: UInt32 = 0
    for s in 0..<8 {
        // Extract 6 bits starting at bit offset s*6 (0-based, MSB first)
        let bitOffset = s * 6
        var sixBits: UInt8 = 0
        for b in 0..<6 {
            let globalBit = bitOffset + b
            let byteIdx = globalBit / 8
            let bitIdx  = 7 - (globalBit % 8)
            let bit = (xored[byteIdx] >> bitIdx) & 1
            sixBits = (sixBits << 1) | bit
        }

        // Row = outer bits (bit 5 and bit 0 of the 6-bit group)
        let row = Int(((sixBits & 0x20) >> 4) | (sixBits & 0x01))
        // Col = inner bits (bits 4..1)
        let col = Int((sixBits & 0x1E) >> 1)
        let sVal = SBOXES[s][row * 16 + col]  // 4-bit output

        sOutput = (sOutput << 4) | UInt32(sVal)
    }

    // Pack sOutput (32 bits) into 4 bytes
    var sBytes = [UInt8](repeating: 0, count: 4)
    sBytes[0] = UInt8((sOutput >> 24) & 0xFF)
    sBytes[1] = UInt8((sOutput >> 16) & 0xFF)
    sBytes[2] = UInt8((sOutput >>  8) & 0xFF)
    sBytes[3] = UInt8( sOutput        & 0xFF)

    // Step 4: P permutation
    return permute(sBytes, P_TABLE, outBits: 32)
}

// ============================================================================
// MARK: - Core Block Cipher
// ============================================================================

/// Encrypt or decrypt one 8-byte DES block with the given subkeys.
///
/// Encryption and decryption share the same Feistel structure. The only
/// difference is subkey order: encryption uses subkeys 0..15, decryption
/// uses subkeys 15..0.
///
/// - Parameters:
///   - block: Exactly 8 bytes of plaintext or ciphertext.
///   - subkeys: Array of 16 subkeys from `expandKey`. Pass in reverse order
///     for decryption.
/// - Returns: Encrypted or decrypted 8 bytes.
private func desRound(_ block: [UInt8], subkeys: [[UInt8]]) -> [UInt8] {
    // Initial Permutation
    let permuted = permute(block, IP, outBits: 64)

    // Split into left (32 bits) and right (32 bits)
    var l = Array(permuted[0..<4])
    var r = Array(permuted[4..<8])

    // 16 Feistel rounds
    for round in 0..<16 {
        let fOut = desF(r, subkeys[round])
        let newR = zip(l, fOut).map { $0 ^ $1 }
        l = r
        r = newR
    }

    // Final swap and concatenation: output = R || L (Feistel convention)
    let preOutput = r + l

    // Final Permutation (IP inverse)
    return permute(preOutput, FP, outBits: 64)
}

// ============================================================================
// MARK: - Public API
// ============================================================================

/// Encrypt one 8-byte block with DES.
///
/// Implements the DES block cipher as specified in FIPS 46-3.
/// This is the raw single-block operation (no mode, no padding).
///
/// - Parameters:
///   - block: Exactly 8 bytes of plaintext.
///   - key: Exactly 8 bytes. Bits 8, 16, 24, 32, 40, 48, 56, 64 are parity
///     bits and are ignored. The effective key length is 56 bits.
/// - Returns: Exactly 8 bytes of ciphertext.
/// - Precondition: `block.count == 8`, `key.count == 8`
public func desEncryptBlock(_ block: [UInt8], key: [UInt8]) -> [UInt8] {
    precondition(block.count == 8, "DES block must be exactly 8 bytes")
    precondition(key.count == 8,   "DES key must be exactly 8 bytes")
    let subkeys = expandKey(key)
    return desRound(block, subkeys: subkeys)
}

/// Decrypt one 8-byte block with DES.
///
/// Uses the same Feistel structure as encryption, but applies subkeys in
/// reverse order (K16, K15, ..., K1).
///
/// - Parameters:
///   - block: Exactly 8 bytes of ciphertext.
///   - key: Exactly 8 bytes (same key used for encryption).
/// - Returns: Exactly 8 bytes of plaintext.
/// - Precondition: `block.count == 8`, `key.count == 8`
public func desDecryptBlock(_ block: [UInt8], key: [UInt8]) -> [UInt8] {
    precondition(block.count == 8, "DES block must be exactly 8 bytes")
    precondition(key.count == 8,   "DES key must be exactly 8 bytes")
    let subkeys = expandKey(key).reversed()
    return desRound(block, subkeys: Array(subkeys))
}

// ============================================================================
// MARK: - ECB Mode with PKCS#7 Padding
// ============================================================================
//
// ECB (Electronic Codebook) mode encrypts each 8-byte block independently.
// Identical plaintext blocks produce identical ciphertext blocks — this is a
// known weakness, but ECB remains useful for key wrapping and educational work.
//
// PKCS#7 padding appends k bytes of value k to reach the next block boundary:
//   plaintext length 0..7: pad to 8 bytes (add 8-n bytes of value 8-n)
//   plaintext length 8: pad to 16 bytes (add 8 bytes of value 0x08)
//   plaintext length 9..15: pad to 16 bytes, etc.
//
// PKCS#7 always adds padding, even when the message is already block-aligned.
// This ensures unambiguous removal: the last byte always tells how many bytes
// to strip.

/// Encrypt arbitrary-length data using DES in ECB mode with PKCS#7 padding.
///
/// - Parameters:
///   - plaintext: Any number of bytes (including empty).
///   - key: Exactly 8 bytes DES key.
/// - Returns: Ciphertext whose length is a multiple of 8. Always at least 8 bytes.
public func desECBEncrypt(_ plaintext: [UInt8], key: [UInt8]) -> [UInt8] {
    precondition(key.count == 8, "DES key must be exactly 8 bytes")

    // PKCS#7 padding
    let padLen = 8 - (plaintext.count % 8)
    var padded = plaintext
    padded.append(contentsOf: [UInt8](repeating: UInt8(padLen), count: padLen))

    let subkeys = expandKey(key)
    var ciphertext: [UInt8] = []
    ciphertext.reserveCapacity(padded.count)

    var i = 0
    while i < padded.count {
        let block = Array(padded[i..<i+8])
        ciphertext.append(contentsOf: desRound(block, subkeys: subkeys))
        i += 8
    }
    return ciphertext
}

/// Decrypt data encrypted with `desECBEncrypt`.
///
/// Removes PKCS#7 padding after decryption.
///
/// - Parameters:
///   - ciphertext: Byte array whose length must be a non-zero multiple of 8.
///   - key: Exactly 8 bytes DES key.
/// - Returns: Original plaintext with padding removed.
/// - Precondition: `ciphertext.count > 0 && ciphertext.count % 8 == 0`
public func desECBDecrypt(_ ciphertext: [UInt8], key: [UInt8]) -> [UInt8] {
    precondition(key.count == 8, "DES key must be exactly 8 bytes")
    precondition(ciphertext.count > 0 && ciphertext.count % 8 == 0,
                 "DES ciphertext length must be a positive multiple of 8 bytes")

    let subkeys = Array(expandKey(key).reversed())
    var plaintext: [UInt8] = []
    plaintext.reserveCapacity(ciphertext.count)

    var i = 0
    while i < ciphertext.count {
        let block = Array(ciphertext[i..<i+8])
        plaintext.append(contentsOf: desRound(block, subkeys: subkeys))
        i += 8
    }

    // Strip PKCS#7 padding
    let padLen = Int(plaintext.last ?? 0)
    guard padLen >= 1, padLen <= 8, padLen <= plaintext.count else {
        return plaintext
    }
    return Array(plaintext.dropLast(padLen))
}

// ============================================================================
// MARK: - Triple DES (3DES / TDEA)
// ============================================================================
//
// NIST SP 800-67 defines TDEA as EDE (Encrypt-Decrypt-Encrypt):
//   TDEA(P) = E(K1, D(K2, E(K3, P)))
//
// The EDE ordering was chosen so that K1=K2=K3=K reduces to single DES:
//   E(K, D(K, E(K, P))) = E(K, P)
//
// Three independent keys (K1 ≠ K2 ≠ K3) give 112 effective key bits
// (due to meet-in-the-middle, not the full 168 bits).
//
// NIST deprecated 3DES in 2017 (SP 800-131A) and disallowed new use in 2024.

/// Encrypt one 8-byte block with Triple DES (EDE mode per NIST SP 800-67).
///
/// TDEA ordering: E(K1, D(K2, E(K3, P)))
///   Step 1: Encrypt with K3 (innermost)
///   Step 2: Decrypt with K2 (middle)
///   Step 3: Encrypt with K1 (outermost)
///
/// When K1=K2=K3: reduces to single DES for backward compatibility.
///
/// - Parameters:
///   - block: Exactly 8 bytes of plaintext.
///   - k1: First 8-byte DES key (outermost encrypt).
///   - k2: Second 8-byte DES key (middle decrypt).
///   - k3: Third 8-byte DES key (innermost encrypt).
/// - Returns: Exactly 8 bytes of ciphertext.
public func tdeaEncryptBlock(_ block: [UInt8], k1: [UInt8], k2: [UInt8], k3: [UInt8]) -> [UInt8] {
    let step1 = desEncryptBlock(block, key: k3)
    let step2 = desDecryptBlock(step1, key: k2)
    return desEncryptBlock(step2, key: k1)
}

/// Decrypt one 8-byte block with Triple DES (EDE mode per NIST SP 800-67).
///
/// TDEA decryption inverts EDE: D(K3, E(K2, D(K1, C)))
///   Step 1: Decrypt with K1 (reverse outermost)
///   Step 2: Encrypt with K2 (reverse middle)
///   Step 3: Decrypt with K3 (reverse innermost)
///
/// - Parameters:
///   - block: Exactly 8 bytes of ciphertext.
///   - k1: First key (must match encryption K1).
///   - k2: Second key (must match encryption K2).
///   - k3: Third key (must match encryption K3).
/// - Returns: Exactly 8 bytes of plaintext.
public func tdeaDecryptBlock(_ block: [UInt8], k1: [UInt8], k2: [UInt8], k3: [UInt8]) -> [UInt8] {
    let step1 = desDecryptBlock(block, key: k1)
    let step2 = desEncryptBlock(step1, key: k2)
    return desDecryptBlock(step2, key: k3)
}
