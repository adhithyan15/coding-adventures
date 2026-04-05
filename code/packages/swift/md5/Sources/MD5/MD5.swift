// MD5.swift
// Part of coding-adventures -- an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// MD5 Message Digest Algorithm (RFC 1321)
// ============================================================================
//
// MD5 (Message Digest 5) takes any sequence of bytes and produces a fixed-size
// 16-byte (128-bit) "fingerprint" called a digest. The same input always
// produces the same digest. Change even one bit of input and the digest
// changes completely -- the "avalanche effect".
//
// Created by Ron Rivest in 1991 as an improvement over MD4. Standardized in
// RFC 1321. MD5 is cryptographically broken (collision attacks since 2004)
// and should NOT be used for security purposes (digital signatures, password
// hashing, TLS certificates). It remains valid for: non-security checksums,
// UUID v3, and legacy systems that already use it.
//
// The #1 Gotcha: Little-Endian Throughout
// =======================================
// MD5 is LITTLE-ENDIAN: least significant byte first. This differs from
// SHA-1 (big-endian) and is the source of most MD5 implementation bugs.
//
//   Big-endian (SHA-1):    0x0A0B0C0D -> bytes [0A, 0B, 0C, 0D]
//   Little-endian (MD5):   0x0A0B0C0D -> bytes [0D, 0C, 0B, 0A]
//
// Swift UInt32 Arithmetic
// =======================
// Unlike JavaScript, Swift's UInt32 type naturally wraps on overflow when
// we use the &+ operator (wrapping addition) and &<< / &>> (wrapping shifts).
// This means we do NOT need the `>>> 0` trick that JavaScript requires.
//
//   let x: UInt32 = 0xFFFFFFFF
//   x &+ 1  // = 0  (wraps around, no crash)
//
// We use `&+` for all additions and bitwise operators (&, |, ^, ~) work
// naturally on unsigned 32-bit integers in Swift.

import Foundation

// ============================================================================
// T-Table: 64 Constants Derived From Sine
// ============================================================================
//
// T[i] = floor(abs(sin(i+1)) * 2^32)  for i = 0..63
//
// These are "nothing up my sleeve" numbers: anyone can verify them from the
// standard sine function. No hidden backdoor is possible because the
// derivation is fully public. Example:
//
//   sin(1) ~ 0.84147...
//   |sin(1)| * 2^32 = 3614090360.02...
//   floor(...) = 3614090360 = 0xD76AA478 = T[0]
//
// We precompute all 64 values as a static array for performance.

private let T: [UInt32] = {
    var table = [UInt32](repeating: 0, count: 64)
    for i in 0..<64 {
        let sinVal = sin(Double(i + 1))
        table[i] = UInt32(abs(sinVal) * 4294967296.0)  // 2^32
    }
    return table
}()

// ============================================================================
// Round Shift Amounts
// ============================================================================
//
// Each of the 64 rounds rotates left by a specific number of bits. The
// pattern is fixed by the RFC -- four groups of 16, each repeating a
// 4-element cycle. These values were chosen to maximize diffusion
// (avalanche effect).
//
//   Rounds  0-15:  [7, 12, 17, 22] x 4   (Stage 1 -- F function)
//   Rounds 16-31:  [5,  9, 14, 20] x 4   (Stage 2 -- G function)
//   Rounds 32-47:  [4, 11, 16, 23] x 4   (Stage 3 -- H function)
//   Rounds 48-63:  [6, 10, 15, 21] x 4   (Stage 4 -- I function)

private let S: [UInt32] = [
    7, 12, 17, 22,  7, 12, 17, 22,  7, 12, 17, 22,  7, 12, 17, 22,  // rounds  0-15
    5,  9, 14, 20,  5,  9, 14, 20,  5,  9, 14, 20,  5,  9, 14, 20,  // rounds 16-31
    4, 11, 16, 23,  4, 11, 16, 23,  4, 11, 16, 23,  4, 11, 16, 23,  // rounds 32-47
    6, 10, 15, 21,  6, 10, 15, 21,  6, 10, 15, 21,  6, 10, 15, 21,  // rounds 48-63
]

// ============================================================================
// Initialization Constants
// ============================================================================
//
// The four-word state starts with these fixed values. They look like the
// hex sequence 0123456789ABCDEF split into bytes and reversed pairwise:
//
//   A = 0x67452301 -> byte sequence: 01 23 45 67 (reversed -> 67 45 23 01)
//   B = 0xEFCDAB89 -> byte sequence: 89 AB CD EF (reversed -> EF CD AB 89)
//   C = 0x98BADCFE -> byte sequence: FE DC BA 98 (reversed -> 98 BA DC FE)
//   D = 0x10325476 -> byte sequence: 76 54 32 10 (reversed -> 10 32 54 76)

private let INIT_A: UInt32 = 0x67452301
private let INIT_B: UInt32 = 0xEFCDAB89
private let INIT_C: UInt32 = 0x98BADCFE
private let INIT_D: UInt32 = 0x10325476

// ============================================================================
// Helper: Circular Left Rotation
// ============================================================================
//
// Rotate x left by n bits within a 32-bit word. Bits that fall off the left
// reappear on the right.
//
//   rotl(0x80000000, 1) = 0x00000001  (top bit wraps around to bottom)
//
// In Swift, we use &<< and &>> for wrapping shifts on UInt32.

@inline(__always)
private func rotl(_ x: UInt32, _ n: UInt32) -> UInt32 {
    return (x &<< n) | (x &>> (32 &- n))
}

// ============================================================================
// Helper: Read a little-endian UInt32 from a byte array
// ============================================================================
//
// MD5 treats the message as an array of little-endian 32-bit words.
// This function reads 4 bytes starting at `offset` and assembles them
// into a UInt32 with the first byte as the least significant.
//
// Example: bytes [0x78, 0xA4, 0x6A, 0xD7] -> 0xD76AA478

@inline(__always)
private func readLE32(_ bytes: [UInt8], _ offset: Int) -> UInt32 {
    return UInt32(bytes[offset])
        | (UInt32(bytes[offset + 1]) << 8)
        | (UInt32(bytes[offset + 2]) << 16)
        | (UInt32(bytes[offset + 3]) << 24)
}

// ============================================================================
// Helper: Write a UInt32 as little-endian bytes
// ============================================================================
//
// The final 16-byte digest is the four 32-bit state words written in
// LITTLE-ENDIAN byte order.
//
// Example: 0xD76AA478 -> bytes [0x78, 0xA4, 0x6A, 0xD7]

@inline(__always)
private func writeLE32(_ value: UInt32) -> [UInt8] {
    return [
        UInt8(value & 0xFF),
        UInt8((value >> 8) & 0xFF),
        UInt8((value >> 16) & 0xFF),
        UInt8((value >> 24) & 0xFF),
    ]
}

// ============================================================================
// Padding
// ============================================================================
//
// MD5 operates on 512-bit (64-byte) blocks. Messages that aren't a
// multiple of 64 bytes need padding according to RFC 1321 section 3.1:
//
//   1. Append the byte 0x80 (a single 1-bit followed by zeros).
//   2. Append zero bytes until the total length = 56 (mod 64).
//      This leaves 8 bytes at the end of each 64-byte block for length.
//   3. Append the original bit-length as a 64-bit LITTLE-ENDIAN integer.
//      (This differs from SHA-1, which uses big-endian here!)
//
// Example -- "abc" (3 bytes = 24 bits):
//   61 62 63 80 [52 zero bytes] 18 00 00 00 00 00 00 00
//                               ^^ LE encoding of 24 (0x18)

private func pad(_ data: [UInt8]) -> [UInt8] {
    let bitLen = UInt64(data.count) * 8

    // After appending 0x80, total is (data.count + 1) bytes.
    // We want (data.count + 1 + zeroCount) % 64 == 56.
    let afterBit = (data.count + 1) % 64
    let zeroCount = afterBit <= 56 ? 56 - afterBit : 64 + 56 - afterBit

    // Total = original + 1 (0x80) + zeroCount + 8 (length)
    var result = [UInt8](repeating: 0, count: data.count + 1 + zeroCount + 8)
    // Copy original data
    for i in 0..<data.count {
        result[i] = data[i]
    }
    result[data.count] = 0x80
    // Zero bytes are already zero from initialization

    // Append 64-bit LITTLE-endian bit length.
    let lengthOffset = result.count - 8
    result[lengthOffset]     = UInt8(bitLen & 0xFF)
    result[lengthOffset + 1] = UInt8((bitLen >> 8) & 0xFF)
    result[lengthOffset + 2] = UInt8((bitLen >> 16) & 0xFF)
    result[lengthOffset + 3] = UInt8((bitLen >> 24) & 0xFF)
    result[lengthOffset + 4] = UInt8((bitLen >> 32) & 0xFF)
    result[lengthOffset + 5] = UInt8((bitLen >> 40) & 0xFF)
    result[lengthOffset + 6] = UInt8((bitLen >> 48) & 0xFF)
    result[lengthOffset + 7] = UInt8((bitLen >> 56) & 0xFF)

    return result
}

// ============================================================================
// Compression Function
// ============================================================================
//
// The heart of MD5: mix one 64-byte block into the four-word state using
// 64 rounds of bit manipulation. Each round uses one of four auxiliary
// functions:
//
//   Stage 1 (i < 16):  F(B,C,D) = (B & C) | (~B & D)  -- "if B then C else D"
//   Stage 2 (i < 32):  G(B,C,D) = (D & B) | (~D & C)  -- same but D selects
//   Stage 3 (i < 48):  H(B,C,D) = B ^ C ^ D            -- parity
//   Stage 4 (i < 64):  I(B,C,D) = C ^ (B | ~D)         -- unusual mix
//
// Message word selection g per stage:
//   Stage 1: g = i            (sequential: 0, 1, 2, ..., 15)
//   Stage 2: g = (5i + 1)%16  (stride 5: 1, 6, 11, 0, 5, ...)
//   Stage 3: g = (3i + 5)%16  (stride 3: 5, 8, 11, 14, 1, ...)
//   Stage 4: g = (7i) % 16    (stride 7: 0, 7, 14, 5, 12, ...)
//
// Each round:
//   temp = B + ROTL(S[i], A + f + M[g] + T[i])  (mod 2^32)
//   (A, B, C, D) <- (D, temp, B, C)
//
// Davies-Meyer feed-forward: after all 64 rounds, add the pre-round
// state (mod 2^32) to prevent the compression from being invertible.

private func compress(
    _ stateA: UInt32,
    _ stateB: UInt32,
    _ stateC: UInt32,
    _ stateD: UInt32,
    block: [UInt8],
    blockOffset: Int
) -> (UInt32, UInt32, UInt32, UInt32) {
    // Parse 16 little-endian 32-bit words from the block.
    var M = [UInt32](repeating: 0, count: 16)
    for j in 0..<16 {
        M[j] = readLE32(block, blockOffset + j * 4)
    }

    // Save initial state for Davies-Meyer addition at the end.
    let a0 = stateA, b0 = stateB, c0 = stateC, d0 = stateD
    var a = a0, b = b0, c = c0, d = d0

    for i in 0..<64 {
        let f: UInt32
        let g: Int

        if i < 16 {
            // Stage 1 -- F function: if B then C else D
            f = (b & c) | (~b & d)
            g = i
        } else if i < 32 {
            // Stage 2 -- G function: if D then B else C
            f = (d & b) | (~d & c)
            g = (5 * i + 1) % 16
        } else if i < 48 {
            // Stage 3 -- H function: bitwise parity (XOR of all three)
            f = b ^ c ^ d
            g = (3 * i + 5) % 16
        } else {
            // Stage 4 -- I function: C XOR (B OR NOT D)
            f = c ^ (b | ~d)
            g = (7 * i) % 16
        }

        // Core round computation:
        //   inner = (A + f + M[g] + T[i]) mod 2^32
        //   temp  = B + ROTL(inner, S[i])   mod 2^32
        let inner = a &+ f &+ M[g] &+ T[i]
        let temp = b &+ rotl(inner, S[i])

        // Shift the four words: D->A, C->D, B->C, temp->B
        a = d
        d = c
        c = b
        b = temp
    }

    // Davies-Meyer: add compressed output to initial block state (mod 2^32).
    return (a0 &+ a, b0 &+ b, c0 &+ c, d0 &+ d)
}

// ============================================================================
// Output Serialization
// ============================================================================
//
// The final 16-byte digest is the four 32-bit state words written in
// LITTLE-ENDIAN byte order.

private func stateToData(_ a: UInt32, _ b: UInt32, _ c: UInt32, _ d: UInt32) -> Data {
    var bytes = [UInt8]()
    bytes.append(contentsOf: writeLE32(a))
    bytes.append(contentsOf: writeLE32(b))
    bytes.append(contentsOf: writeLE32(c))
    bytes.append(contentsOf: writeLE32(d))
    return Data(bytes)
}

// ============================================================================
// Helper: Convert Data to lowercase hex string
// ============================================================================

private func toHex(_ data: Data) -> String {
    return data.map { String(format: "%02x", $0) }.joined()
}

// ============================================================================
// Public API: One-Shot md5
// ============================================================================

/// Compute the MD5 digest of a `Data` value. Returns 16 bytes.
///
/// This is the one-shot API: hash a complete message in a single call.
///
/// **WARNING:** MD5 is cryptographically broken. Do NOT use for passwords,
/// digital signatures, or security-sensitive checksums. Use for UUID v3 or
/// legacy compatibility only.
///
/// RFC 1321 test vectors:
///
///     md5(Data())           -> d41d8cd98f00b204e9800998ecf8427e
///     md5("abc".data(...))  -> 900150983cd24fb0d6963f7d28e17f72
///
/// - Parameter data: The bytes to hash.
/// - Returns: A 16-byte `Data` containing the MD5 digest.
public func md5(_ data: Data) -> Data {
    let bytes = [UInt8](data)
    let padded = pad(bytes)
    var a = INIT_A, b = INIT_B, c = INIT_C, d = INIT_D

    // Process each 64-byte block sequentially.
    var offset = 0
    while offset < padded.count {
        (a, b, c, d) = compress(a, b, c, d, block: padded, blockOffset: offset)
        offset += 64
    }

    return stateToData(a, b, c, d)
}

// ============================================================================
// Public API: Hex Variant
// ============================================================================

/// Compute the MD5 digest and return it as a 32-character lowercase
/// hexadecimal string.
///
/// Equivalent to `toHex(md5(data))`.
///
/// - Parameter data: The bytes to hash.
/// - Returns: A 32-character lowercase hex string.
public func md5Hex(_ data: Data) -> String {
    return toHex(md5(data))
}

// ============================================================================
// Public API: Streaming MD5Hasher
// ============================================================================
//
// When the full message is not available at once -- e.g., reading a large
// file in chunks -- the streaming API allows incremental updates.
//
// Internally, we keep:
//   - state: the four-word running hash (updated after each complete block)
//   - buffer: bytes accumulated but not yet forming a complete 64-byte block
//   - byteCount: total bytes fed so far (needed for the padding length field)
//
// The update() method feeds complete 64-byte blocks to compress()
// immediately and buffers the remainder. The digest() method handles final
// padding without mutating the object state, so it can be called multiple
// times.

/// A streaming MD5 hasher that accepts data in multiple chunks.
///
/// Useful when the full message is not available at once -- for example
/// when reading a large file in chunks or hashing a network stream.
///
/// ```swift
/// var hasher = MD5Hasher()
/// hasher.update(Data("ab".utf8))
/// hasher.update(Data("c".utf8))
/// hasher.hexDigest()  // "900150983cd24fb0d6963f7d28e17f72"
/// ```
///
/// Multiple `update()` calls are equivalent to a single `md5(allData)`.
public struct MD5Hasher: Sendable {
    private var _a: UInt32
    private var _b: UInt32
    private var _c: UInt32
    private var _d: UInt32
    private var _buffer: [UInt8]
    private var _byteCount: Int

    /// Initialize a new MD5 hasher with the standard initial state.
    public init() {
        _a = INIT_A
        _b = INIT_B
        _c = INIT_C
        _d = INIT_D
        _buffer = []
        _byteCount = 0
    }

    /// Feed more bytes into the hasher.
    ///
    /// Can be called multiple times. `update(a); update(b)` is equivalent
    /// to hashing `a + b` in one shot.
    ///
    /// - Parameter data: The bytes to feed into the hash computation.
    public mutating func update(_ data: Data) {
        let bytes = [UInt8](data)
        _byteCount += bytes.count
        _buffer.append(contentsOf: bytes)

        // Process complete 64-byte blocks from the buffer.
        var offset = 0
        while offset + 64 <= _buffer.count {
            (_a, _b, _c, _d) = compress(_a, _b, _c, _d, block: _buffer, blockOffset: offset)
            offset += 64
        }

        // Keep only the unprocessed remainder in the buffer.
        if offset > 0 {
            _buffer = Array(_buffer[offset...])
        }
    }

    /// Return the 16-byte MD5 digest of all data fed so far.
    ///
    /// Non-destructive: the internal state is not modified, so you can
    /// continue calling `update()` after calling `digest()`.
    ///
    /// - Returns: A 16-byte `Data` containing the MD5 digest.
    public func digest() -> Data {
        // Construct the padding tail using the buffered remainder.
        // The length field must reflect the TOTAL byte count, not just
        // the buffer length.
        let bitLen = UInt64(_byteCount) * 8
        let buf = _buffer

        let afterBit = (buf.count + 1) % 64
        let zeroCount = afterBit <= 56 ? 56 - afterBit : 64 + 56 - afterBit

        var tail = [UInt8](repeating: 0, count: buf.count + 1 + zeroCount + 8)
        for i in 0..<buf.count {
            tail[i] = buf[i]
        }
        tail[buf.count] = 0x80

        // Append 64-bit little-endian total bit count.
        let lengthOffset = tail.count - 8
        tail[lengthOffset]     = UInt8(bitLen & 0xFF)
        tail[lengthOffset + 1] = UInt8((bitLen >> 8) & 0xFF)
        tail[lengthOffset + 2] = UInt8((bitLen >> 16) & 0xFF)
        tail[lengthOffset + 3] = UInt8((bitLen >> 24) & 0xFF)
        tail[lengthOffset + 4] = UInt8((bitLen >> 32) & 0xFF)
        tail[lengthOffset + 5] = UInt8((bitLen >> 40) & 0xFF)
        tail[lengthOffset + 6] = UInt8((bitLen >> 48) & 0xFF)
        tail[lengthOffset + 7] = UInt8((bitLen >> 56) & 0xFF)

        // Compress the tail block(s) using a copy of the running state.
        var a = _a, b = _b, c = _c, d = _d
        var offset = 0
        while offset < tail.count {
            (a, b, c, d) = compress(a, b, c, d, block: tail, blockOffset: offset)
            offset += 64
        }

        return stateToData(a, b, c, d)
    }

    /// Return the 32-character lowercase hex string of the digest.
    ///
    /// Equivalent to converting `digest()` to hex.
    ///
    /// - Returns: A 32-character lowercase hex string.
    public func hexDigest() -> String {
        return toHex(digest())
    }

    /// Return an independent copy of the current hasher.
    ///
    /// Useful for computing multiple digests from a common prefix:
    ///
    /// ```swift
    /// var h = MD5Hasher()
    /// h.update(prefix)
    /// var h1 = h.copy()
    /// h1.update(suffix1)
    /// let hash1 = h1.digest()
    /// ```
    ///
    /// - Returns: A new `MD5Hasher` with identical internal state.
    public func copy() -> MD5Hasher {
        var other = MD5Hasher()
        other._a = _a
        other._b = _b
        other._c = _c
        other._d = _d
        other._buffer = _buffer
        other._byteCount = _byteCount
        return other
    }
}
