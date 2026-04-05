// SHA256.swift
// Part of coding-adventures -- an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// SHA-256 Secure Hash Algorithm (FIPS 180-4)
// ============================================================================
//
// SHA-256 is a cryptographic hash function from the SHA-2 family, designed
// by the NSA and published by NIST in 2001 (FIPS 180-2, updated in FIPS
// 180-4). It produces a 256-bit (32-byte) digest, typically shown as a
// 64-character hex string.
//
// Unlike MD5 (broken 2004) and SHA-1 (broken 2017), SHA-256 remains secure
// with no known practical attacks. The birthday bound is 2^128, making
// collision search computationally infeasible.
//
// SHA-256 follows the Merkle-Damgard construction like MD5 and SHA-1, but
// with a wider state (8 x 32-bit words), more rounds (64), and a more
// complex message schedule.
//
// SHA-256 is BIG-ENDIAN throughout: most significant byte first.
//
//   Big-endian:    0x0A0B0C0D -> bytes [0A, 0B, 0C, 0D]
//   Little-endian: 0x0A0B0C0D -> bytes [0D, 0C, 0B, 0A]
//
// Swift UInt32 Arithmetic
// =======================
// Swift's UInt32 type naturally wraps on overflow when we use the &+
// operator (wrapping addition). Standard bitwise operators (&, |, ^, ~,
// <<, >>) work on UInt32 without wrapping concerns since they cannot
// overflow.

import Foundation

// ============================================================================
// Initial Hash Values H0..H7 (FIPS 180-4, Section 5.3.3)
// ============================================================================
//
// First 32 bits of the fractional parts of the square roots of the first
// 8 primes: 2, 3, 5, 7, 11, 13, 17, 19.
//
// Example derivation for H0:
//   sqrt(2) = 1.41421356... -> fractional part = 0.41421356...
//   0.41421356... * 2^32 = 1779033703.95... -> floor = 0x6A09E667

private let INIT_H: (UInt32, UInt32, UInt32, UInt32, UInt32, UInt32, UInt32, UInt32) = (
    0x6A09E667,  // H0 -- sqrt(2)
    0xBB67AE85,  // H1 -- sqrt(3)
    0x3C6EF372,  // H2 -- sqrt(5)
    0xA54FF53A,  // H3 -- sqrt(7)
    0x510E527F,  // H4 -- sqrt(11)
    0x9B05688C,  // H5 -- sqrt(13)
    0x1F83D9AB,  // H6 -- sqrt(17)
    0x5BE0CD19   // H7 -- sqrt(19)
)

// ============================================================================
// Round Constants K0..K63 (FIPS 180-4, Section 4.2.2)
// ============================================================================
//
// First 32 bits of the fractional parts of the cube roots of the first
// 64 primes (2, 3, 5, 7, 11, 13, ..., 311).

private let K: [UInt32] = [
    0x428A2F98, 0x71374491, 0xB5C0FBCF, 0xE9B5DBA5,
    0x3956C25B, 0x59F111F1, 0x923F82A4, 0xAB1C5ED5,
    0xD807AA98, 0x12835B01, 0x243185BE, 0x550C7DC3,
    0x72BE5D74, 0x80DEB1FE, 0x9BDC06A7, 0xC19BF174,
    0xE49B69C1, 0xEFBE4786, 0x0FC19DC6, 0x240CA1CC,
    0x2DE92C6F, 0x4A7484AA, 0x5CB0A9DC, 0x76F988DA,
    0x983E5152, 0xA831C66D, 0xB00327C8, 0xBF597FC7,
    0xC6E00BF3, 0xD5A79147, 0x06CA6351, 0x14292967,
    0x27B70A85, 0x2E1B2138, 0x4D2C6DFC, 0x53380D13,
    0x650A7354, 0x766A0ABB, 0x81C2C92E, 0x92722C85,
    0xA2BFE8A1, 0xA81A664B, 0xC24B8B70, 0xC76C51A3,
    0xD192E819, 0xD6990624, 0xF40E3585, 0x106AA070,
    0x19A4C116, 0x1E376C08, 0x2748774C, 0x34B0BCB5,
    0x391C0CB3, 0x4ED8AA4A, 0x5B9CCA4F, 0x682E6FF3,
    0x748F82EE, 0x78A5636F, 0x84C87814, 0x8CC70208,
    0x90BEFFFA, 0xA4506CEB, 0xBEF9A3F7, 0xC67178F2,
]

// ============================================================================
// Helper: Right Rotation
// ============================================================================
//
// rotr(x, n) rotates x right by n bit positions within a 32-bit word.
// Bits that "fall off" the right end reappear on the left.
//
// SHA-256 uses right rotations (unlike SHA-1 which uses left rotations).

@inline(__always)
private func rotr(_ x: UInt32, _ n: UInt32) -> UInt32 {
    return (x >> n) | (x << (32 - n))
}

// ============================================================================
// Helper: Read/Write big-endian UInt32
// ============================================================================

@inline(__always)
private func readBE32(_ bytes: [UInt8], _ offset: Int) -> UInt32 {
    return (UInt32(bytes[offset]) << 24)
        | (UInt32(bytes[offset + 1]) << 16)
        | (UInt32(bytes[offset + 2]) << 8)
        | UInt32(bytes[offset + 3])
}

@inline(__always)
private func writeBE32(_ value: UInt32) -> [UInt8] {
    return [
        UInt8((value >> 24) & 0xFF),
        UInt8((value >> 16) & 0xFF),
        UInt8((value >> 8) & 0xFF),
        UInt8(value & 0xFF),
    ]
}

// ============================================================================
// SHA-256 Auxiliary Functions (FIPS 180-4, Section 4.1.2)
// ============================================================================
//
// These six functions provide the non-linear mixing that makes SHA-256
// a one-way function.

// Ch(x,y,z) -- "Choice": for each bit, if x=1 pick y, else pick z
@inline(__always)
private func ch(_ x: UInt32, _ y: UInt32, _ z: UInt32) -> UInt32 {
    return (x & y) ^ (~x & z)
}

// Maj(x,y,z) -- "Majority": output is majority vote of the 3 inputs
@inline(__always)
private func maj(_ x: UInt32, _ y: UInt32, _ z: UInt32) -> UInt32 {
    return (x & y) ^ (x & z) ^ (y & z)
}

// Sigma0(x) -- "Big Sigma 0": used on variable 'a' in compression rounds
@inline(__always)
private func bigSigma0(_ x: UInt32) -> UInt32 {
    return rotr(x, 2) ^ rotr(x, 13) ^ rotr(x, 22)
}

// Sigma1(x) -- "Big Sigma 1": used on variable 'e' in compression rounds
@inline(__always)
private func bigSigma1(_ x: UInt32) -> UInt32 {
    return rotr(x, 6) ^ rotr(x, 11) ^ rotr(x, 25)
}

// sigma0(x) -- "Small sigma 0": used in message schedule expansion
// Note: third term is a right SHIFT (not rotate) -- bits fall off permanently
@inline(__always)
private func smallSigma0(_ x: UInt32) -> UInt32 {
    return rotr(x, 7) ^ rotr(x, 18) ^ (x >> 3)
}

// sigma1(x) -- "Small sigma 1": used in message schedule expansion
@inline(__always)
private func smallSigma1(_ x: UInt32) -> UInt32 {
    return rotr(x, 17) ^ rotr(x, 19) ^ (x >> 10)
}

// ============================================================================
// Padding (FIPS 180-4, Section 5.1.1)
// ============================================================================
//
// SHA-256 operates on 512-bit (64-byte) blocks. Padding extends the message
// to a multiple of 64 bytes:
//
//   1. Append 0x80 (the '1' bit followed by seven '0' bits).
//   2. Append 0x00 bytes until length = 56 (mod 64).
//   3. Append the original bit length as a 64-bit BIG-ENDIAN integer.

private func pad(_ data: [UInt8]) -> [UInt8] {
    let bitLen = UInt64(data.count) * 8

    let afterBit = (data.count + 1) % 64
    let zeroCount = afterBit <= 56 ? 56 - afterBit : 64 + 56 - afterBit

    var result = [UInt8](repeating: 0, count: data.count + 1 + zeroCount + 8)
    for i in 0..<data.count {
        result[i] = data[i]
    }
    result[data.count] = 0x80

    // Append 64-bit BIG-endian bit length
    let lengthOffset = result.count - 8
    result[lengthOffset]     = UInt8((bitLen >> 56) & 0xFF)
    result[lengthOffset + 1] = UInt8((bitLen >> 48) & 0xFF)
    result[lengthOffset + 2] = UInt8((bitLen >> 40) & 0xFF)
    result[lengthOffset + 3] = UInt8((bitLen >> 32) & 0xFF)
    result[lengthOffset + 4] = UInt8((bitLen >> 24) & 0xFF)
    result[lengthOffset + 5] = UInt8((bitLen >> 16) & 0xFF)
    result[lengthOffset + 6] = UInt8((bitLen >> 8) & 0xFF)
    result[lengthOffset + 7] = UInt8(bitLen & 0xFF)

    return result
}

// ============================================================================
// Message Schedule + Compression
// ============================================================================
//
// Each 64-byte block is parsed as 16 big-endian 32-bit words (W[0..15]),
// then expanded to 64 words:
//   W[t] = sigma1(W[t-2]) + W[t-7] + sigma0(W[t-15]) + W[t-16]
//
// 64 rounds of compression fold one block into the eight-word state.
// Each round:
//   T1 = h + Sigma1(e) + Ch(e,f,g) + K[t] + W[t]
//   T2 = Sigma0(a) + Maj(a,b,c)
//   h=g, g=f, f=e, e=d+T1, d=c, c=b, b=a, a=T1+T2
//
// Davies-Meyer feed-forward: after all rounds, add the compressed
// output back to the original state.

private func compress(
    _ h0: UInt32, _ h1: UInt32, _ h2: UInt32, _ h3: UInt32,
    _ h4: UInt32, _ h5: UInt32, _ h6: UInt32, _ h7: UInt32,
    block: [UInt8],
    blockOffset: Int
) -> (UInt32, UInt32, UInt32, UInt32, UInt32, UInt32, UInt32, UInt32) {
    // Build 64-word message schedule
    var W = [UInt32](repeating: 0, count: 64)
    for i in 0..<16 {
        W[i] = readBE32(block, blockOffset + i * 4)
    }
    for i in 16..<64 {
        W[i] = smallSigma1(W[i-2]) &+ W[i-7] &+ smallSigma0(W[i-15]) &+ W[i-16]
    }

    var a = h0, b = h1, c = h2, d = h3
    var e = h4, f = h5, g = h6, h = h7

    for t in 0..<64 {
        let T1 = h &+ bigSigma1(e) &+ ch(e, f, g) &+ K[t] &+ W[t]
        let T2 = bigSigma0(a) &+ maj(a, b, c)

        h = g
        g = f
        f = e
        e = d &+ T1
        d = c
        c = b
        b = a
        a = T1 &+ T2
    }

    return (h0 &+ a, h1 &+ b, h2 &+ c, h3 &+ d,
            h4 &+ e, h5 &+ f, h6 &+ g, h7 &+ h)
}

// ============================================================================
// Finalization: Convert state to Data
// ============================================================================

private func stateToData(
    _ h0: UInt32, _ h1: UInt32, _ h2: UInt32, _ h3: UInt32,
    _ h4: UInt32, _ h5: UInt32, _ h6: UInt32, _ h7: UInt32
) -> Data {
    var bytes = [UInt8]()
    bytes.append(contentsOf: writeBE32(h0))
    bytes.append(contentsOf: writeBE32(h1))
    bytes.append(contentsOf: writeBE32(h2))
    bytes.append(contentsOf: writeBE32(h3))
    bytes.append(contentsOf: writeBE32(h4))
    bytes.append(contentsOf: writeBE32(h5))
    bytes.append(contentsOf: writeBE32(h6))
    bytes.append(contentsOf: writeBE32(h7))
    return Data(bytes)
}

// ============================================================================
// Helper: Convert Data to lowercase hex string
// ============================================================================

private func toHex(_ data: Data) -> String {
    return data.map { String(format: "%02x", $0) }.joined()
}

// ============================================================================
// Public API: One-Shot sha256
// ============================================================================

/// Compute the SHA-256 digest of a `Data` value. Returns 32 bytes.
///
/// This is the one-shot API: hash a complete message in a single call.
///
/// FIPS 180-4 test vectors:
///
///     sha256(Data())           -> e3b0c44298fc1c149afbf4c8996fb924...
///     sha256("abc".data(...))  -> ba7816bf8f01cfea414140de5dae2223...
///
/// - Parameter data: The bytes to hash.
/// - Returns: A 32-byte `Data` containing the SHA-256 digest.
public func sha256(_ data: Data) -> Data {
    let bytes = [UInt8](data)
    let padded = pad(bytes)
    var h0 = INIT_H.0, h1 = INIT_H.1, h2 = INIT_H.2, h3 = INIT_H.3
    var h4 = INIT_H.4, h5 = INIT_H.5, h6 = INIT_H.6, h7 = INIT_H.7

    var offset = 0
    while offset < padded.count {
        (h0, h1, h2, h3, h4, h5, h6, h7) = compress(
            h0, h1, h2, h3, h4, h5, h6, h7,
            block: padded, blockOffset: offset
        )
        offset += 64
    }

    return stateToData(h0, h1, h2, h3, h4, h5, h6, h7)
}

// ============================================================================
// Public API: Hex Variant
// ============================================================================

/// Compute the SHA-256 digest and return it as a 64-character lowercase
/// hexadecimal string.
///
/// - Parameter data: The bytes to hash.
/// - Returns: A 64-character lowercase hex string.
public func sha256Hex(_ data: Data) -> String {
    return toHex(sha256(data))
}

// ============================================================================
// Public API: Streaming SHA256Hasher
// ============================================================================
//
// When the full message is not available at once -- e.g., reading a large
// file in chunks -- the streaming API allows incremental updates.
//
// Internally, we keep:
//   - state: the eight-word running hash
//   - buffer: bytes not yet forming a complete 64-byte block
//   - byteCount: total bytes fed so far (needed for the padding length)

/// A streaming SHA-256 hasher that accepts data in multiple chunks.
///
/// ```swift
/// var hasher = SHA256Hasher()
/// hasher.update(Data("ab".utf8))
/// hasher.update(Data("c".utf8))
/// hasher.hexDigest()  // "ba7816bf8f01cfea414140de5dae2223..."
/// ```
///
/// Multiple `update()` calls are equivalent to a single `sha256(allData)`.
public struct SHA256Hasher: Sendable {
    private var _h0: UInt32
    private var _h1: UInt32
    private var _h2: UInt32
    private var _h3: UInt32
    private var _h4: UInt32
    private var _h5: UInt32
    private var _h6: UInt32
    private var _h7: UInt32
    private var _buffer: [UInt8]
    private var _byteCount: Int

    /// Initialize a new SHA-256 hasher with the standard initial state.
    public init() {
        _h0 = INIT_H.0
        _h1 = INIT_H.1
        _h2 = INIT_H.2
        _h3 = INIT_H.3
        _h4 = INIT_H.4
        _h5 = INIT_H.5
        _h6 = INIT_H.6
        _h7 = INIT_H.7
        _buffer = []
        _byteCount = 0
    }

    /// Feed more bytes into the hasher.
    ///
    /// - Parameter data: The bytes to feed into the hash computation.
    public mutating func update(_ data: Data) {
        let bytes = [UInt8](data)
        _byteCount += bytes.count
        _buffer.append(contentsOf: bytes)

        // Process complete 64-byte blocks from the buffer.
        var offset = 0
        while offset + 64 <= _buffer.count {
            (_h0, _h1, _h2, _h3, _h4, _h5, _h6, _h7) = compress(
                _h0, _h1, _h2, _h3, _h4, _h5, _h6, _h7,
                block: _buffer,
                blockOffset: offset
            )
            offset += 64
        }

        // Keep only the unprocessed remainder.
        if offset > 0 {
            _buffer = Array(_buffer[offset...])
        }
    }

    /// Return the 32-byte SHA-256 digest of all data fed so far.
    ///
    /// Non-destructive: the internal state is not modified.
    ///
    /// - Returns: A 32-byte `Data` containing the SHA-256 digest.
    public func digest() -> Data {
        let bitLen = UInt64(_byteCount) * 8
        let buf = _buffer

        let afterBit = (buf.count + 1) % 64
        let zeroCount = afterBit <= 56 ? 56 - afterBit : 64 + 56 - afterBit

        var tail = [UInt8](repeating: 0, count: buf.count + 1 + zeroCount + 8)
        for i in 0..<buf.count {
            tail[i] = buf[i]
        }
        tail[buf.count] = 0x80

        // Append 64-bit big-endian total bit count.
        let lengthOffset = tail.count - 8
        tail[lengthOffset]     = UInt8((bitLen >> 56) & 0xFF)
        tail[lengthOffset + 1] = UInt8((bitLen >> 48) & 0xFF)
        tail[lengthOffset + 2] = UInt8((bitLen >> 40) & 0xFF)
        tail[lengthOffset + 3] = UInt8((bitLen >> 32) & 0xFF)
        tail[lengthOffset + 4] = UInt8((bitLen >> 24) & 0xFF)
        tail[lengthOffset + 5] = UInt8((bitLen >> 16) & 0xFF)
        tail[lengthOffset + 6] = UInt8((bitLen >> 8) & 0xFF)
        tail[lengthOffset + 7] = UInt8(bitLen & 0xFF)

        var h0 = _h0, h1 = _h1, h2 = _h2, h3 = _h3
        var h4 = _h4, h5 = _h5, h6 = _h6, h7 = _h7
        var offset = 0
        while offset < tail.count {
            (h0, h1, h2, h3, h4, h5, h6, h7) = compress(
                h0, h1, h2, h3, h4, h5, h6, h7,
                block: tail, blockOffset: offset
            )
            offset += 64
        }

        return stateToData(h0, h1, h2, h3, h4, h5, h6, h7)
    }

    /// Return the 64-character lowercase hex string of the digest.
    public func hexDigest() -> String {
        return toHex(digest())
    }

    /// Return an independent copy of the current hasher.
    ///
    /// Useful for computing multiple digests from a common prefix.
    public func copy() -> SHA256Hasher {
        var other = SHA256Hasher()
        other._h0 = _h0
        other._h1 = _h1
        other._h2 = _h2
        other._h3 = _h3
        other._h4 = _h4
        other._h5 = _h5
        other._h6 = _h6
        other._h7 = _h7
        other._buffer = _buffer
        other._byteCount = _byteCount
        return other
    }
}
