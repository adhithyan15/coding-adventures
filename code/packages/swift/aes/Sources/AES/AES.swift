// AES.swift
// Part of coding-adventures — an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// Advanced Encryption Standard (AES) — FIPS 197
// ============================================================================
//
// AES is a symmetric block cipher designed by Joan Daemen and Vincent Rijmen
// (originally called Rijndael) and adopted as a U.S. federal standard in 2001.
// It supports 128-bit, 192-bit, and 256-bit keys, operating on 128-bit (16-byte)
// blocks in 10, 12, or 14 rounds respectively.
//
// AES is a Substitution-Permutation Network (SPN), not a Feistel cipher. Each
// round applies four transformations to the entire 128-bit state simultaneously.
// Unlike Feistel ciphers, encryption and decryption use different circuits.
//
// ============================================================================
// State Layout (Column-Major)
// ============================================================================
//
// AES arranges the 16-byte block as a 4×4 byte matrix, filled column-by-column:
//
//   Byte 0  Byte 4  Byte 8  Byte 12     col 0  col 1  col 2  col 3
//   Byte 1  Byte 5  Byte 9  Byte 13  =  row 0  row 0  row 0  row 0
//   Byte 2  Byte 6  Byte 10 Byte 14     row 1  ...
//   Byte 3  Byte 7  Byte 11 Byte 15
//
//   state[row][col] = block[row + 4 * col]   (0-based indexing)
//
// ============================================================================
// The Four Round Transformations
// ============================================================================
//
// SubBytes: Non-linear substitution via the AES S-box. Each byte b is replaced
//   by SBOX[b], where SBOX is built by:
//   1. Finding the multiplicative inverse of b in GF(2^8) with polynomial 0x11B
//      (define inverse(0) = 0 by convention)
//   2. Applying an affine transformation:
//      s = inv ^ rot(inv,1) ^ rot(inv,2) ^ rot(inv,3) ^ rot(inv,4) ^ 0x63
//      where rot(b,n) means left-rotate the 8-bit byte b by n positions.
//
// ShiftRows: Cyclically shifts each row left by its row index:
//   Row 0: no shift   (identity)
//   Row 1: shift left by 1
//   Row 2: shift left by 2
//   Row 3: shift left by 3
//   This provides inter-column diffusion across rounds.
//
// MixColumns: Each column is treated as a degree-3 polynomial over GF(2^8)
//   and multiplied by the fixed polynomial:
//     a(x) = {03}x^3 + {01}x^2 + {01}x + {02}
//   modulo x^4 + 1. In matrix form, each column [b0,b1,b2,b3]^T is multiplied by:
//     [2 3 1 1]
//     [1 2 3 1]
//     [1 1 2 3]
//     [3 1 1 2]
//   All multiplication is in GF(2^8) with polynomial 0x11B.
//   This is the primary diffusion step.
//
// AddRoundKey: XOR the state with the round key. The round key is 4 columns
//   (16 bytes) extracted from the expanded key schedule.
//
// ============================================================================
// Key Schedule
// ============================================================================
//
// The key is expanded into Nb*(Nr+1) 32-bit words (W), where:
//   Nb = 4 (block size in 32-bit words — always 4 for AES)
//   Nk = key length in 32-bit words (4, 6, or 8)
//   Nr = number of rounds (10, 12, or 14)
//
// Expansion recurrence:
//   W[i] = W[i-1] XOR W[i-Nk]                     if i mod Nk ≠ 0
//   W[i] = SubWord(RotWord(W[i-1])) XOR Rcon[i/Nk] if i mod Nk = 0
//   W[i] = SubWord(W[i-1]) XOR W[i-Nk]             if Nk=8 and i mod 8 = 4
//
//   RotWord: rotate word [b0,b1,b2,b3] to [b1,b2,b3,b0]
//   SubWord: apply SBOX to each of the 4 bytes
//   Rcon[i]: [x^(i-1), 0, 0, 0] in GF(2^8); x = 0x02

import GF256

// ============================================================================
// MARK: - GF(2^8) AES Field
// ============================================================================
//
// AES uses the polynomial x^8+x^4+x^3+x+1 = 0x11B, distinct from the
// Reed-Solomon polynomial 0x11D used by the base GF256 enum.

// nonisolated(unsafe): GF256Field is a pure value type (all stored properties
// are immutable `let`) so sharing across concurrency domains is safe. The
// upstream package does not yet declare Sendable conformance, so we assert it.
nonisolated(unsafe) private let aesField = GF256Field(polynomial: 0x11B)

// ============================================================================
// MARK: - S-box Construction
// ============================================================================
//
// The S-box is built at module initialization time from GF(2^8) inverses
// and the affine transform. This educational approach demonstrates the
// mathematical structure; production code would use a hardcoded table.
//
// Affine transform formula (FIPS 197 Section 5.1.1):
//   For each bit i of the output byte s_i:
//     s_i = b_i XOR b_{(i+4)%8} XOR b_{(i+5)%8} XOR b_{(i+6)%8} XOR b_{(i+7)%8} XOR c_i
//   where c = 0x63 = 0b01100011 and b is the inverse byte.
//
// Equivalent compact form:
//   s = b ^ rotl8(b,1) ^ rotl8(b,2) ^ rotl8(b,3) ^ rotl8(b,4) ^ 0x63
//
// The S-box is a bijection (all 256 outputs are distinct) and has no fixed
// points (SBOX[b] ≠ b for all b). These properties resist linear and
// differential cryptanalysis.

/// Left-rotate an 8-bit byte by n positions.
private func rotl8(_ b: UInt8, _ n: Int) -> UInt8 {
    return (b << n) | (b >> (8 - n))
}

/// Apply the AES affine transform to a byte (after GF inverse).
private func affineTransform(_ b: UInt8) -> UInt8 {
    return b ^ rotl8(b, 1) ^ rotl8(b, 2) ^ rotl8(b, 3) ^ rotl8(b, 4) ^ 0x63
}

/// Build the AES S-box (256 entries).
private func buildSBox() -> [UInt8] {
    var sbox = [UInt8](repeating: 0, count: 256)
    for i in 0..<256 {
        let b = UInt8(i)
        let inv = (b == 0) ? 0 : aesField.inverse(b)
        sbox[i] = affineTransform(inv)
    }
    return sbox
}

/// Build the inverse S-box from the S-box.
private func buildInvSBox(_ sbox: [UInt8]) -> [UInt8] {
    var inv = [UInt8](repeating: 0, count: 256)
    for i in 0..<256 {
        inv[Int(sbox[i])] = UInt8(i)
    }
    return inv
}

// Module-level constants: built once, shared by all calls.
private let SBOX:     [UInt8] = buildSBox()
private let INV_SBOX: [UInt8] = buildInvSBox(SBOX)

// ============================================================================
// MARK: - Round Constants (Rcon)
// ============================================================================
//
// Rcon[i] = [x^i, 0, 0, 0] in GF(2^8) (x = 0x02).
// AES-128 needs Rcon[1..10], AES-192 needs [1..8], AES-256 needs [1..7].
// We precompute 11 entries (indices 1..10).
//
// x^0 = 0x01, x^1 = 0x02, x^2 = 0x04, ... x^k = xtime(x^{k-1})
// where xtime(a) = (a << 1) ^ (0x1B if a >= 0x80 else 0)

private let RCON: [UInt8] = {
    var rcon = [UInt8](repeating: 0, count: 11)
    rcon[1] = 0x01
    for i in 2...10 {
        let prev = rcon[i-1]
        rcon[i] = (prev < 0x80) ? (prev << 1) : ((prev << 1) ^ 0x1B)
    }
    return rcon
}()

// ============================================================================
// MARK: - State Type
// ============================================================================
//
// The AES state is a 4×4 byte matrix indexed as state[row][col].
// Column-major layout: state[row][col] = block[row + 4*col].

private typealias State = [[UInt8]]  // 4 rows × 4 cols

/// Convert a 16-byte block into a 4×4 AES state (column-major).
private func blockToState(_ block: [UInt8]) -> State {
    var state = [[UInt8]](repeating: [UInt8](repeating: 0, count: 4), count: 4)
    for col in 0..<4 {
        for row in 0..<4 {
            state[row][col] = block[row + 4 * col]
        }
    }
    return state
}

/// Convert a 4×4 AES state back into a 16-byte block (column-major).
private func stateToBlock(_ state: State) -> [UInt8] {
    var block = [UInt8](repeating: 0, count: 16)
    for col in 0..<4 {
        for row in 0..<4 {
            block[row + 4 * col] = state[row][col]
        }
    }
    return block
}

// ============================================================================
// MARK: - Key Expansion
// ============================================================================

/// Expand an AES key into round keys.
///
/// Returns an array of (Nr+1) round keys, each 16 bytes (4 words).
/// Round keys are used in order: round 0 is the pre-round AddRoundKey,
/// rounds 1..Nr-1 are the main rounds, round Nr is the final round.
///
/// - Parameter key: 16, 24, or 32 bytes (AES-128, AES-192, AES-256).
/// - Returns: Array of round key byte arrays (each 16 bytes).
public func expandKey(_ key: [UInt8]) -> [[UInt8]] {
    let nk = key.count / 4  // key length in 32-bit words: 4, 6, or 8
    let nr: Int              // number of rounds
    switch nk {
    case 4: nr = 10
    case 6: nr = 12
    case 8: nr = 14
    default: fatalError("AES key must be 16, 24, or 32 bytes; got \(key.count)")
    }

    // Total words needed: Nb*(Nr+1) = 4*(nr+1)
    let totalWords = 4 * (nr + 1)
    var w = [[UInt8]](repeating: [UInt8](repeating: 0, count: 4), count: totalWords)

    // First Nk words come directly from the key
    for i in 0..<nk {
        w[i] = [key[4*i], key[4*i+1], key[4*i+2], key[4*i+3]]
    }

    // Expand remaining words
    for i in nk..<totalWords {
        var temp = w[i-1]

        if i % nk == 0 {
            // RotWord: [b0,b1,b2,b3] → [b1,b2,b3,b0]
            temp = [temp[1], temp[2], temp[3], temp[0]]
            // SubWord: apply S-box to each byte
            temp = temp.map { SBOX[Int($0)] }
            // XOR with Rcon
            temp[0] ^= RCON[i / nk]
        } else if nk == 8 && i % nk == 4 {
            // AES-256 extra SubWord step
            temp = temp.map { SBOX[Int($0)] }
        }

        // W[i] = W[i-Nk] XOR temp
        w[i] = zip(w[i-nk], temp).map { $0 ^ $1 }
    }

    // Group into round keys (4 words = 16 bytes each)
    var roundKeys: [[UInt8]] = []
    for r in 0...nr {
        var rk = [UInt8](repeating: 0, count: 16)
        for col in 0..<4 {
            let word = w[r * 4 + col]
            rk[4*col]   = word[0]
            rk[4*col+1] = word[1]
            rk[4*col+2] = word[2]
            rk[4*col+3] = word[3]
        }
        roundKeys.append(rk)
    }
    return roundKeys
}

// ============================================================================
// MARK: - Round Transformations (Encryption)
// ============================================================================

/// AddRoundKey: XOR each state byte with the corresponding round key byte.
///
/// The round key is interpreted in column-major order to match the state.
private func addRoundKey(_ state: inout State, _ roundKey: [UInt8]) {
    for col in 0..<4 {
        for row in 0..<4 {
            state[row][col] ^= roundKey[row + 4 * col]
        }
    }
}

/// SubBytes: replace each state byte with SBOX[byte].
private func subBytes(_ state: inout State) {
    for row in 0..<4 {
        for col in 0..<4 {
            state[row][col] = SBOX[Int(state[row][col])]
        }
    }
}

/// ShiftRows: cyclically shift each row left by its row index.
///
///   Row 0: [s00, s01, s02, s03] → [s00, s01, s02, s03]  (no change)
///   Row 1: [s10, s11, s12, s13] → [s11, s12, s13, s10]
///   Row 2: [s20, s21, s22, s23] → [s22, s23, s20, s21]
///   Row 3: [s30, s31, s32, s33] → [s33, s30, s31, s32]
private func shiftRows(_ state: inout State) {
    for row in 1..<4 {
        let r = state[row]
        state[row] = [r[row%4], r[(row+1)%4], r[(row+2)%4], r[(row+3)%4]]
    }
}

/// MixColumns: multiply each column by the AES MixColumns matrix in GF(2^8).
///
/// Matrix (FIPS 197 Section 5.1.3):
///   [2 3 1 1]   [b0]   [2b0 + 3b1 + b2  + b3 ]
///   [1 2 3 1] × [b1] = [b0  + 2b1 + 3b2 + b3 ]
///   [1 1 2 3]   [b2]   [b0  + b1  + 2b2 + 3b3]
///   [3 1 1 2]   [b3]   [3b0 + b1  + b2  + 2b3]
///
/// All arithmetic in GF(2^8) with polynomial 0x11B.
private func mixColumns(_ state: inout State) {
    for col in 0..<4 {
        let b = (0..<4).map { state[$0][col] }
        state[0][col] = aesField.multiply(2, b[0]) ^ aesField.multiply(3, b[1]) ^ b[2] ^ b[3]
        state[1][col] = b[0] ^ aesField.multiply(2, b[1]) ^ aesField.multiply(3, b[2]) ^ b[3]
        state[2][col] = b[0] ^ b[1] ^ aesField.multiply(2, b[2]) ^ aesField.multiply(3, b[3])
        state[3][col] = aesField.multiply(3, b[0]) ^ b[1] ^ b[2] ^ aesField.multiply(2, b[3])
    }
}

// ============================================================================
// MARK: - Round Transformations (Decryption)
// ============================================================================

/// InvSubBytes: replace each byte with INV_SBOX[byte].
private func invSubBytes(_ state: inout State) {
    for row in 0..<4 {
        for col in 0..<4 {
            state[row][col] = INV_SBOX[Int(state[row][col])]
        }
    }
}

/// InvShiftRows: cyclically shift each row RIGHT by its row index.
///
///   Row 0: no change
///   Row 1: [s10, s11, s12, s13] → [s13, s10, s11, s12]  (right by 1 = left by 3)
///   Row 2: [s20, s21, s22, s23] → [s22, s23, s20, s21]  (right by 2 = left by 2)
///   Row 3: [s30, s31, s32, s33] → [s31, s32, s33, s30]  (right by 3 = left by 1)
private func invShiftRows(_ state: inout State) {
    for row in 1..<4 {
        let r = state[row]
        let shift = 4 - row  // right by `row` = left by `4 - row`
        state[row] = [r[shift%4], r[(shift+1)%4], r[(shift+2)%4], r[(shift+3)%4]]
    }
}

/// InvMixColumns: multiply each column by the inverse MixColumns matrix.
///
/// Inverse matrix (FIPS 197 Section 5.3.3):
///   [14  11  13   9]
///   [ 9  14  11  13]
///   [13   9  14  11]
///   [11  13   9  14]
///
/// 0x0e = 14, 0x0b = 11, 0x0d = 13, 0x09 = 9
private func invMixColumns(_ state: inout State) {
    for col in 0..<4 {
        let b = (0..<4).map { state[$0][col] }
        state[0][col] = aesField.multiply(0x0e, b[0]) ^ aesField.multiply(0x0b, b[1])
                      ^ aesField.multiply(0x0d, b[2]) ^ aesField.multiply(0x09, b[3])
        state[1][col] = aesField.multiply(0x09, b[0]) ^ aesField.multiply(0x0e, b[1])
                      ^ aesField.multiply(0x0b, b[2]) ^ aesField.multiply(0x0d, b[3])
        state[2][col] = aesField.multiply(0x0d, b[0]) ^ aesField.multiply(0x09, b[1])
                      ^ aesField.multiply(0x0e, b[2]) ^ aesField.multiply(0x0b, b[3])
        state[3][col] = aesField.multiply(0x0b, b[0]) ^ aesField.multiply(0x0d, b[1])
                      ^ aesField.multiply(0x09, b[2]) ^ aesField.multiply(0x0e, b[3])
    }
}

// ============================================================================
// MARK: - Public API: Block Encrypt / Decrypt
// ============================================================================

/// Encrypt one 16-byte block with AES.
///
/// Implements FIPS 197 Section 5.1 (Cipher). Supports AES-128, AES-192,
/// and AES-256 depending on the key length.
///
/// Encryption round structure:
///   AddRoundKey(state, roundKeys[0])        — initial round key addition
///   For round = 1 to Nr-1:
///     SubBytes → ShiftRows → MixColumns → AddRoundKey
///   Final round (no MixColumns):
///     SubBytes → ShiftRows → AddRoundKey(state, roundKeys[Nr])
///
/// - Parameters:
///   - block: Exactly 16 bytes of plaintext.
///   - key: 16, 24, or 32 bytes (AES-128, AES-192, AES-256).
/// - Returns: Exactly 16 bytes of ciphertext.
/// - Precondition: `block.count == 16`, `key.count ∈ {16, 24, 32}`
public func aesEncryptBlock(_ block: [UInt8], key: [UInt8]) -> [UInt8] {
    precondition(block.count == 16, "AES block must be 16 bytes; got \(block.count)")
    precondition(key.count == 16 || key.count == 24 || key.count == 32,
                 "AES key must be 16, 24, or 32 bytes; got \(key.count)")

    let roundKeys = expandKey(key)
    let nr = roundKeys.count - 1
    var state = blockToState(block)

    // Round 0: initial AddRoundKey
    addRoundKey(&state, roundKeys[0])

    // Rounds 1 to Nr-1: SubBytes → ShiftRows → MixColumns → AddRoundKey
    for round in 1..<nr {
        subBytes(&state)
        shiftRows(&state)
        mixColumns(&state)
        addRoundKey(&state, roundKeys[round])
    }

    // Final round (no MixColumns)
    subBytes(&state)
    shiftRows(&state)
    addRoundKey(&state, roundKeys[nr])

    return stateToBlock(state)
}

/// Decrypt one 16-byte block with AES.
///
/// Implements FIPS 197 Section 5.3 (Inverse Cipher). Uses InvShiftRows,
/// InvSubBytes, AddRoundKey, InvMixColumns in the inverse order.
///
/// Decryption round structure:
///   AddRoundKey(state, roundKeys[Nr])       — undo final round key
///   For round = Nr-1 downto 1:
///     InvShiftRows → InvSubBytes → AddRoundKey → InvMixColumns
///   Final round:
///     InvShiftRows → InvSubBytes → AddRoundKey(state, roundKeys[0])
///
/// - Parameters:
///   - block: Exactly 16 bytes of ciphertext.
///   - key: 16, 24, or 32 bytes (same key used for encryption).
/// - Returns: Exactly 16 bytes of plaintext.
/// - Precondition: `block.count == 16`, `key.count ∈ {16, 24, 32}`
public func aesDecryptBlock(_ block: [UInt8], key: [UInt8]) -> [UInt8] {
    precondition(block.count == 16, "AES block must be 16 bytes; got \(block.count)")
    precondition(key.count == 16 || key.count == 24 || key.count == 32,
                 "AES key must be 16, 24, or 32 bytes; got \(key.count)")

    let roundKeys = expandKey(key)
    let nr = roundKeys.count - 1
    var state = blockToState(block)

    // Undo final round
    addRoundKey(&state, roundKeys[nr])

    // Rounds Nr-1 downto 1: InvShiftRows → InvSubBytes → AddRoundKey → InvMixColumns
    for round in stride(from: nr - 1, through: 1, by: -1) {
        invShiftRows(&state)
        invSubBytes(&state)
        addRoundKey(&state, roundKeys[round])
        invMixColumns(&state)
    }

    // Undo round 0
    invShiftRows(&state)
    invSubBytes(&state)
    addRoundKey(&state, roundKeys[0])

    return stateToBlock(state)
}

// ============================================================================
// MARK: - Public Accessors for S-box constants
// ============================================================================

/// The AES S-box: 256-element lookup table mapping byte → S-box output.
///
/// Built from GF(2^8) inverse (polynomial 0x11B) + affine transform.
/// Used in SubBytes and key schedule SubWord.
public var sbox: [UInt8] { SBOX }

/// The inverse AES S-box: undoes SubBytes.
///
/// INV_SBOX[SBOX[b]] = b for all b in 0..255.
public var invSbox: [UInt8] { INV_SBOX }
