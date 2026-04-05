// SHA512.swift
// Part of coding-adventures -- an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// SHA-512 Secure Hash Algorithm (FIPS 180-4)
// ============================================================================
//
// SHA-512 is the 64-bit sibling of SHA-256 in the SHA-2 family. It produces
// a 512-bit (64-byte) digest using eight 64-bit state words and 80 rounds of
// compression. On 64-bit platforms, SHA-512 is often faster than SHA-256
// because it processes 128-byte blocks using native 64-bit arithmetic.
//
// Key differences from SHA-256:
//   - State: 8 x 64-bit words (vs 32-bit)
//   - Block size: 128 bytes (vs 64 bytes)
//   - Rounds: 80 (vs 64)
//   - Round constants: 80 x 64-bit (cube roots of first 80 primes)
//   - Length field: 128-bit big-endian (vs 64-bit)
//   - Rotation amounts differ (tuned for 64-bit words)
//
// Swift UInt64 Arithmetic
// =======================
// Swift's UInt64 type naturally wraps on overflow when we use the &+
// operator (wrapping addition). Bitwise operators (&, |, ^, <<, >>)
// work on unsigned values without sign issues.

import Foundation

// ============================================================================
// Initialization Constants (FIPS 180-4, Section 5.3.5)
// ============================================================================
//
// First 64 bits of the fractional parts of the square roots of the first
// 8 prime numbers (2, 3, 5, 7, 11, 13, 17, 19).

private let INIT_H: [UInt64] = [
    0x6a09e667f3bcc908,
    0xbb67ae8584caa73b,
    0x3c6ef372fe94f82b,
    0xa54ff53a5f1d36f1,
    0x510e527fade682d1,
    0x9b05688c2b3e6c1f,
    0x1f83d9abfb41bd6b,
    0x5be0cd19137e2179,
]

// ============================================================================
// Round Constants (FIPS 180-4, Section 4.2.3)
// ============================================================================
//
// First 64 bits of the fractional parts of the cube roots of the first
// 80 prime numbers (2, 3, 5, ..., 409).

private let K: [UInt64] = [
    0x428a2f98d728ae22, 0x7137449123ef65cd, 0xb5c0fbcfec4d3b2f, 0xe9b5dba58189dbbc,
    0x3956c25bf348b538, 0x59f111f1b605d019, 0x923f82a4af194f9b, 0xab1c5ed5da6d8118,
    0xd807aa98a3030242, 0x12835b0145706fbe, 0x243185be4ee4b28c, 0x550c7dc3d5ffb4e2,
    0x72be5d74f27b896f, 0x80deb1fe3b1696b1, 0x9bdc06a725c71235, 0xc19bf174cf692694,
    0xe49b69c19ef14ad2, 0xefbe4786384f25e3, 0x0fc19dc68b8cd5b5, 0x240ca1cc77ac9c65,
    0x2de92c6f592b0275, 0x4a7484aa6ea6e483, 0x5cb0a9dcbd41fbd4, 0x76f988da831153b5,
    0x983e5152ee66dfab, 0xa831c66d2db43210, 0xb00327c898fb213f, 0xbf597fc7beef0ee4,
    0xc6e00bf33da88fc2, 0xd5a79147930aa725, 0x06ca6351e003826f, 0x142929670a0e6e70,
    0x27b70a8546d22ffc, 0x2e1b21385c26c926, 0x4d2c6dfc5ac42aed, 0x53380d139d95b3df,
    0x650a73548baf63de, 0x766a0abb3c77b2a8, 0x81c2c92e47edaee6, 0x92722c851482353b,
    0xa2bfe8a14cf10364, 0xa81a664bbc423001, 0xc24b8b70d0f89791, 0xc76c51a30654be30,
    0xd192e819d6ef5218, 0xd69906245565a910, 0xf40e35855771202a, 0x106aa07032bbd1b8,
    0x19a4c116b8d2d0c8, 0x1e376c085141ab53, 0x2748774cdf8eeb99, 0x34b0bcb5e19b48a8,
    0x391c0cb3c5c95a63, 0x4ed8aa4ae3418acb, 0x5b9cca4f7763e373, 0x682e6ff3d6b2b8a3,
    0x748f82ee5defb2fc, 0x78a5636f43172f60, 0x84c87814a1f0ab72, 0x8cc702081a6439ec,
    0x90befffa23631e28, 0xa4506cebde82bde9, 0xbef9a3f7b2c67915, 0xc67178f2e372532b,
    0xca273eceea26619c, 0xd186b8c721c0c207, 0xeada7dd6cde0eb1e, 0xf57d4f7fee6ed178,
    0x06f067aa72176fba, 0x0a637dc5a2c898a6, 0x113f9804bef90dae, 0x1b710b35131c471b,
    0x28db77f523047d84, 0x32caab7b40c72493, 0x3c9ebe0a15c9bebc, 0x431d67c49c100d4c,
    0x4cc5d4becb3e42b6, 0x597f299cfc657e2a, 0x5fcb6fab3ad6faec, 0x6c44198c4a475817,
]

// ============================================================================
// Helper: Circular Right Rotation
// ============================================================================
//
// rotr(x, n) rotates x right by n bit positions within a 64-bit word.
// Bits that "fall off" the right end reappear on the left.

@inline(__always)
private func rotr(_ x: UInt64, _ n: UInt64) -> UInt64 {
    return (x &>> n) | (x &<< (64 &- n))
}

// ============================================================================
// SHA-512 Auxiliary Functions (FIPS 180-4, Section 4.1.3)
// ============================================================================

/// Sigma0(x) = ROTR(28,x) XOR ROTR(34,x) XOR ROTR(39,x)
@inline(__always)
private func bigSigma0(_ x: UInt64) -> UInt64 {
    return rotr(x, 28) ^ rotr(x, 34) ^ rotr(x, 39)
}

/// Sigma1(x) = ROTR(14,x) XOR ROTR(18,x) XOR ROTR(41,x)
@inline(__always)
private func bigSigma1(_ x: UInt64) -> UInt64 {
    return rotr(x, 14) ^ rotr(x, 18) ^ rotr(x, 41)
}

/// sigma0(x) = ROTR(1,x) XOR ROTR(8,x) XOR SHR(7,x)
@inline(__always)
private func smallSigma0(_ x: UInt64) -> UInt64 {
    return rotr(x, 1) ^ rotr(x, 8) ^ (x >> 7)
}

/// sigma1(x) = ROTR(19,x) XOR ROTR(61,x) XOR SHR(6,x)
@inline(__always)
private func smallSigma1(_ x: UInt64) -> UInt64 {
    return rotr(x, 19) ^ rotr(x, 61) ^ (x >> 6)
}

/// Ch(x,y,z) = (x AND y) XOR (NOT x AND z)
/// "Choice": for each bit, if x=1 choose y, if x=0 choose z.
@inline(__always)
private func ch(_ x: UInt64, _ y: UInt64, _ z: UInt64) -> UInt64 {
    return (x & y) ^ (~x & z)
}

/// Maj(x,y,z) = (x AND y) XOR (x AND z) XOR (y AND z)
/// "Majority": output 1 if at least 2 of 3 inputs are 1.
@inline(__always)
private func maj(_ x: UInt64, _ y: UInt64, _ z: UInt64) -> UInt64 {
    return (x & y) ^ (x & z) ^ (y & z)
}

// ============================================================================
// Helper: Read a big-endian UInt64 from a byte array
// ============================================================================

@inline(__always)
private func readBE64(_ bytes: [UInt8], _ offset: Int) -> UInt64 {
    // Break into sub-expressions to help the Swift type-checker
    // (long chained expressions cause "unable to type-check" errors).
    let hi: UInt64 = (UInt64(bytes[offset]) << 56)
        | (UInt64(bytes[offset + 1]) << 48)
        | (UInt64(bytes[offset + 2]) << 40)
        | (UInt64(bytes[offset + 3]) << 32)
    let lo: UInt64 = (UInt64(bytes[offset + 4]) << 24)
        | (UInt64(bytes[offset + 5]) << 16)
        | (UInt64(bytes[offset + 6]) << 8)
        | UInt64(bytes[offset + 7])
    return hi | lo
}

// ============================================================================
// Helper: Write a UInt64 as big-endian bytes
// ============================================================================

@inline(__always)
private func writeBE64(_ value: UInt64) -> [UInt8] {
    return [
        UInt8((value >> 56) & 0xFF),
        UInt8((value >> 48) & 0xFF),
        UInt8((value >> 40) & 0xFF),
        UInt8((value >> 32) & 0xFF),
        UInt8((value >> 24) & 0xFF),
        UInt8((value >> 16) & 0xFF),
        UInt8((value >> 8) & 0xFF),
        UInt8(value & 0xFF),
    ]
}

// ============================================================================
// Padding (FIPS 180-4, Section 5.1.2)
// ============================================================================
//
// SHA-512 operates on 1024-bit (128-byte) blocks. Padding extends the message
// to a multiple of 128 bytes:
//
//   1. Append 0x80 (the '1' bit followed by seven '0' bits).
//   2. Append 0x00 bytes until length = 112 (mod 128).
//   3. Append the original bit length as a 128-bit BIG-ENDIAN integer.

private func pad(_ data: [UInt8]) -> [UInt8] {
    let bitLen = UInt64(data.count) * 8

    let afterBit = (data.count + 1) % 128
    let zeroCount = afterBit <= 112 ? 112 - afterBit : 128 + 112 - afterBit

    // Total: data + 0x80 + zeros + 16 bytes (128-bit length)
    var result = [UInt8](repeating: 0, count: data.count + 1 + zeroCount + 16)
    for i in 0..<data.count {
        result[i] = data[i]
    }
    result[data.count] = 0x80

    // Append 128-bit BIG-endian bit length.
    // For messages < 2^64 bits, the high 64 bits are zero.
    let lengthOffset = result.count - 16
    // High 64 bits are already zero (initialized to 0)
    // Low 64 bits:
    result[lengthOffset + 8]  = UInt8((bitLen >> 56) & 0xFF)
    result[lengthOffset + 9]  = UInt8((bitLen >> 48) & 0xFF)
    result[lengthOffset + 10] = UInt8((bitLen >> 40) & 0xFF)
    result[lengthOffset + 11] = UInt8((bitLen >> 32) & 0xFF)
    result[lengthOffset + 12] = UInt8((bitLen >> 24) & 0xFF)
    result[lengthOffset + 13] = UInt8((bitLen >> 16) & 0xFF)
    result[lengthOffset + 14] = UInt8((bitLen >> 8) & 0xFF)
    result[lengthOffset + 15] = UInt8(bitLen & 0xFF)

    return result
}

// ============================================================================
// Message Schedule
// ============================================================================
//
// Each 128-byte block is parsed as 16 big-endian 64-bit words (W[0..15]),
// then expanded to 80 words:
//
//   W[t] = sigma1(W[t-2]) + W[t-7] + sigma0(W[t-15]) + W[t-16]

private func schedule(_ block: [UInt8], _ blockOffset: Int) -> [UInt64] {
    var W = [UInt64](repeating: 0, count: 80)
    for i in 0..<16 {
        W[i] = readBE64(block, blockOffset + i * 8)
    }
    for i in 16..<80 {
        W[i] = smallSigma1(W[i-2]) &+ W[i-7] &+ smallSigma0(W[i-15]) &+ W[i-16]
    }
    return W
}

// ============================================================================
// Compression Function
// ============================================================================
//
// 80 rounds of mixing fold one 128-byte block into the eight-word state.
//
// Each round:
//   T1 = h + Sigma1(e) + Ch(e,f,g) + K[t] + W[t]
//   T2 = Sigma0(a) + Maj(a,b,c)
//   h=g, g=f, f=e, e=d+T1, d=c, c=b, b=a, a=T1+T2

private func compress(
    _ state: [UInt64],
    block: [UInt8],
    blockOffset: Int
) -> [UInt64] {
    let W = schedule(block, blockOffset)
    var a = state[0], b = state[1], c = state[2], d = state[3]
    var e = state[4], f = state[5], g = state[6], h = state[7]

    for t in 0..<80 {
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

    return [
        state[0] &+ a, state[1] &+ b, state[2] &+ c, state[3] &+ d,
        state[4] &+ e, state[5] &+ f, state[6] &+ g, state[7] &+ h,
    ]
}

// ============================================================================
// Finalization
// ============================================================================

private func stateToData(_ state: [UInt64]) -> Data {
    var bytes = [UInt8]()
    for word in state {
        bytes.append(contentsOf: writeBE64(word))
    }
    return Data(bytes)
}

// ============================================================================
// Helper: Convert Data to lowercase hex string
// ============================================================================

private func toHex(_ data: Data) -> String {
    return data.map { String(format: "%02x", $0) }.joined()
}

// ============================================================================
// Public API: One-Shot sha512
// ============================================================================

/// Compute the SHA-512 digest of a `Data` value. Returns 64 bytes.
///
/// FIPS 180-4 test vectors:
///
///     sha512(Data())           -> cf83e1357eefb8bd...
///     sha512("abc".data(...))  -> ddaf35a193617aba...
///
/// - Parameter data: The bytes to hash.
/// - Returns: A 64-byte `Data` containing the SHA-512 digest.
public func sha512(_ data: Data) -> Data {
    let bytes = [UInt8](data)
    let padded = pad(bytes)
    var state = INIT_H

    var offset = 0
    while offset < padded.count {
        state = compress(state, block: padded, blockOffset: offset)
        offset += 128
    }

    return stateToData(state)
}

// ============================================================================
// Public API: Hex Variant
// ============================================================================

/// Compute the SHA-512 digest and return it as a 128-character lowercase
/// hexadecimal string.
///
/// - Parameter data: The bytes to hash.
/// - Returns: A 128-character lowercase hex string.
public func sha512Hex(_ data: Data) -> String {
    return toHex(sha512(data))
}

// ============================================================================
// Public API: Streaming SHA512Hasher
// ============================================================================
//
// When the full message is not available at once -- e.g., reading a large
// file in chunks -- the streaming API allows incremental updates.

/// A streaming SHA-512 hasher that accepts data in multiple chunks.
///
/// ```swift
/// var hasher = SHA512Hasher()
/// hasher.update(Data("ab".utf8))
/// hasher.update(Data("c".utf8))
/// hasher.hexDigest()  // "ddaf35a193617aba..."
/// ```
public struct SHA512Hasher: Sendable {
    private var _state: [UInt64]
    private var _buffer: [UInt8]
    private var _byteCount: Int

    /// Initialize a new SHA-512 hasher with the standard initial state.
    public init() {
        _state = INIT_H
        _buffer = []
        _byteCount = 0
    }

    /// Feed more bytes into the hasher.
    public mutating func update(_ data: Data) {
        let bytes = [UInt8](data)
        _byteCount += bytes.count
        _buffer.append(contentsOf: bytes)

        // Process complete 128-byte blocks from the buffer.
        var offset = 0
        while offset + 128 <= _buffer.count {
            _state = compress(_state, block: _buffer, blockOffset: offset)
            offset += 128
        }

        // Keep only the unprocessed remainder.
        if offset > 0 {
            _buffer = Array(_buffer[offset...])
        }
    }

    /// Return the 64-byte SHA-512 digest of all data fed so far.
    ///
    /// Non-destructive: the internal state is not modified.
    public func digest() -> Data {
        let bitLen = UInt64(_byteCount) * 8
        let buf = _buffer

        let afterBit = (buf.count + 1) % 128
        let zeroCount = afterBit <= 112 ? 112 - afterBit : 128 + 112 - afterBit

        var tail = [UInt8](repeating: 0, count: buf.count + 1 + zeroCount + 16)
        for i in 0..<buf.count {
            tail[i] = buf[i]
        }
        tail[buf.count] = 0x80

        // Append 128-bit big-endian total bit count.
        let lengthOffset = tail.count - 16
        // High 64 bits are zero
        tail[lengthOffset + 8]  = UInt8((bitLen >> 56) & 0xFF)
        tail[lengthOffset + 9]  = UInt8((bitLen >> 48) & 0xFF)
        tail[lengthOffset + 10] = UInt8((bitLen >> 40) & 0xFF)
        tail[lengthOffset + 11] = UInt8((bitLen >> 32) & 0xFF)
        tail[lengthOffset + 12] = UInt8((bitLen >> 24) & 0xFF)
        tail[lengthOffset + 13] = UInt8((bitLen >> 16) & 0xFF)
        tail[lengthOffset + 14] = UInt8((bitLen >> 8) & 0xFF)
        tail[lengthOffset + 15] = UInt8(bitLen & 0xFF)

        var state = _state
        var offset = 0
        while offset < tail.count {
            state = compress(state, block: tail, blockOffset: offset)
            offset += 128
        }

        return stateToData(state)
    }

    /// Return the 128-character lowercase hex string of the digest.
    public func hexDigest() -> String {
        return toHex(digest())
    }

    /// Return an independent copy of the current hasher.
    public func copy() -> SHA512Hasher {
        var other = SHA512Hasher()
        other._state = _state
        other._buffer = _buffer
        other._byteCount = _byteCount
        return other
    }
}
