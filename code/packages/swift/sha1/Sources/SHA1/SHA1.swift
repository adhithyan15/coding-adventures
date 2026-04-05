// SHA1.swift
// Part of coding-adventures -- an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// SHA-1 Secure Hash Algorithm (FIPS 180-4)
// ============================================================================
//
// SHA-1 (Secure Hash Algorithm 1) takes any sequence of bytes and produces a
// fixed-size 20-byte (160-bit) "fingerprint" called a digest. The same input
// always produces the same digest. Change even one bit of input and the
// digest changes completely -- the "avalanche effect".
//
// Published by NIST in 1995 as FIPS PUB 180-1. Designed by the NSA as part
// of the Digital Signature Standard.
//
// SHA-1 is BIG-ENDIAN throughout: most significant byte first. This is the
// opposite of MD5 (which is little-endian) and is the most common source of
// bugs when porting between the two algorithms.
//
//   Big-endian (SHA-1):    0x0A0B0C0D -> bytes [0A, 0B, 0C, 0D]
//   Little-endian (MD5):   0x0A0B0C0D -> bytes [0D, 0C, 0B, 0A]
//
// Swift UInt32 Arithmetic
// =======================
// Swift's UInt32 type naturally wraps on overflow when we use the &+
// operator (wrapping addition) and &<< / &>> (wrapping shifts). This
// means we do NOT need the `>>> 0` trick that JavaScript requires.

import Foundation

// ============================================================================
// Initialization Constants
// ============================================================================
//
// SHA-1 starts with these five 32-bit words as its initial state. They are
// "nothing up my sleeve" numbers -- chosen to have an obvious counting
// pattern that proves no mathematical backdoor is hidden:
//
//   H0 = 0x67452301 -> bytes: 67 45 23 01 -> reverse: 01 23 45 67
//   H1 = 0xEFCDAB89 -> bytes: EF CD AB 89 -> reverse: 89 AB CD EF
//   H2 = 0x98BADCFE -> bytes: 98 BA DC FE -> reverse: FE DC BA 98
//   H3 = 0x10325476 -> bytes: 10 32 54 76 -> reverse: 76 54 32 10
//   H4 = 0xC3D2E1F0 -> bytes: C3 D2 E1 F0 -> reverse: F0 E1 D2 C3

private let INIT_H: (UInt32, UInt32, UInt32, UInt32, UInt32) = (
    0x67452301,  // H0
    0xEFCDAB89,  // H1
    0x98BADCFE,  // H2
    0x10325476,  // H3
    0xC3D2E1F0   // H4
)

// ============================================================================
// Round Constants
// ============================================================================
//
// One constant per 20-round stage, derived from square roots:
//   K0 = floor(sqrt(2)  * 2^30) = 0x5A827999  (rounds 0-19)
//   K1 = floor(sqrt(3)  * 2^30) = 0x6ED9EBA1  (rounds 20-39)
//   K2 = floor(sqrt(5)  * 2^30) = 0x8F1BBCDC  (rounds 40-59)
//   K3 = floor(sqrt(10) * 2^30) = 0xCA62C1D6  (rounds 60-79)
//
// Using irrational numbers (square roots) guarantees no special algebraic
// structure -- they are the "most random" numbers we can choose.

private let K: (UInt32, UInt32, UInt32, UInt32) = (
    0x5A827999,  // rounds 0-19
    0x6ED9EBA1,  // rounds 20-39
    0x8F1BBCDC,  // rounds 40-59
    0xCA62C1D6   // rounds 60-79
)

// ============================================================================
// Helper: Circular Left Rotation
// ============================================================================
//
// rotl(x, n) rotates x left by n bit positions within a 32-bit word.
// Bits that "fall off" the left end reappear on the right.
//
// Example: n=2, x = 0b01101001
//   Regular:  01101001 << 2 = 10100100  (leading 01 is gone)
//   Circular: 01101001 ROTL 2 = 10100110  (leading 01 wraps around)

@inline(__always)
private func rotl(_ x: UInt32, _ n: UInt32) -> UInt32 {
    return (x &<< n) | (x &>> (32 &- n))
}

// ============================================================================
// Helper: Read a big-endian UInt32 from a byte array
// ============================================================================
//
// SHA-1 treats the message as an array of big-endian 32-bit words.
// The first byte is the most significant.
//
// Example: bytes [0x0A, 0x0B, 0x0C, 0x0D] -> 0x0A0B0C0D

@inline(__always)
private func readBE32(_ bytes: [UInt8], _ offset: Int) -> UInt32 {
    return (UInt32(bytes[offset]) << 24)
        | (UInt32(bytes[offset + 1]) << 16)
        | (UInt32(bytes[offset + 2]) << 8)
        | UInt32(bytes[offset + 3])
}

// ============================================================================
// Helper: Write a UInt32 as big-endian bytes
// ============================================================================

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
// Padding
// ============================================================================
//
// SHA-1 operates on 512-bit (64-byte) blocks. Padding extends the message
// to a multiple of 64 bytes per FIPS 180-4 section 5.1.1:
//
//   1. Append 0x80 (the '1' bit followed by seven '0' bits).
//   2. Append 0x00 bytes until length = 56 (mod 64).
//   3. Append the original bit length as a 64-bit BIG-ENDIAN integer.
//
// Example -- "abc" (3 bytes = 24 bits):
//   61 62 63 80 [52 zero bytes] 00 00 00 00 00 00 00 18
//                                                   ^^ 24 in hex

private func pad(_ data: [UInt8]) -> [UInt8] {
    let bitLen = UInt64(data.count) * 8

    // After appending 0x80, total is (data.count + 1) bytes.
    // We want (data.count + 1 + zeroCount) % 64 == 56.
    let afterBit = (data.count + 1) % 64
    let zeroCount = afterBit <= 56 ? 56 - afterBit : 64 + 56 - afterBit

    var result = [UInt8](repeating: 0, count: data.count + 1 + zeroCount + 8)
    for i in 0..<data.count {
        result[i] = data[i]
    }
    result[data.count] = 0x80

    // Append 64-bit BIG-endian bit length.
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
// Message Schedule
// ============================================================================
//
// Each 64-byte block is parsed as 16 big-endian 32-bit words (W[0..15]),
// then expanded to 80 words using this recurrence:
//
//   W[i] = ROTL(1, W[i-3] XOR W[i-8] XOR W[i-14] XOR W[i-16])  for i >= 16
//
// Why expand from 16 to 80 words? More words means more mixing means
// better avalanche. A single bit flip in the input block changes
// W[i-3/8/14/16] at different offsets, so the ripple spreads through the
// entire schedule.

private func schedule(_ block: [UInt8], _ blockOffset: Int) -> [UInt32] {
    var W = [UInt32](repeating: 0, count: 80)
    for i in 0..<16 {
        W[i] = readBE32(block, blockOffset + i * 4)
    }
    for i in 16..<80 {
        W[i] = rotl(W[i-3] ^ W[i-8] ^ W[i-14] ^ W[i-16], 1)
    }
    return W
}

// ============================================================================
// Compression Function
// ============================================================================
//
// 80 rounds of mixing fold one 64-byte block into the five-word state.
//
// Four stages of 20 rounds each, using a different auxiliary function:
//
//   Stage  Rounds  f(B, C, D)                    Purpose
//   -----  ------  --------------------------    ----------------
//     1    0-19    (B & C) | (~B & D)            Selector / mux
//     2    20-39   B ^ C ^ D                     Parity
//     3    40-59   (B&C) | (B&D) | (C&D)         Majority vote
//     4    60-79   B ^ C ^ D                     Parity again
//
// Each round:
//   temp = ROTL(5, a) + f(b,c,d) + e + K + W[t]   (mod 2^32)
//   e=d, d=c, c=ROTL(30,b), b=a, a=temp
//
// Davies-Meyer feed-forward: after all 80 rounds, add the compressed
// output back to the original state.

private func compress(
    _ h0: UInt32, _ h1: UInt32, _ h2: UInt32, _ h3: UInt32, _ h4: UInt32,
    block: [UInt8],
    blockOffset: Int
) -> (UInt32, UInt32, UInt32, UInt32, UInt32) {
    let W = schedule(block, blockOffset)
    var a = h0, b = h1, c = h2, d = h3, e = h4

    for t in 0..<80 {
        let f: UInt32
        let k: UInt32

        if t < 20 {
            // Selector: if b=1 output c, if b=0 output d
            f = (b & c) | (~b & d)
            k = K.0
        } else if t < 40 {
            // Parity: 1 if an odd number of inputs are 1
            f = b ^ c ^ d
            k = K.1
        } else if t < 60 {
            // Majority: 1 if at least 2 of the 3 inputs are 1
            f = (b & c) | (b & d) | (c & d)
            k = K.2
        } else {
            // Parity again (same formula, different constant)
            f = b ^ c ^ d
            k = K.3
        }

        let temp = rotl(a, 5) &+ f &+ e &+ k &+ W[t]
        e = d
        d = c
        c = rotl(b, 30)
        b = a
        a = temp
    }

    return (h0 &+ a, h1 &+ b, h2 &+ c, h3 &+ d, h4 &+ e)
}

// ============================================================================
// Finalization
// ============================================================================
//
// Convert the five 32-bit state words to 20 bytes in big-endian order.

private func stateToData(
    _ h0: UInt32, _ h1: UInt32, _ h2: UInt32, _ h3: UInt32, _ h4: UInt32
) -> Data {
    var bytes = [UInt8]()
    bytes.append(contentsOf: writeBE32(h0))
    bytes.append(contentsOf: writeBE32(h1))
    bytes.append(contentsOf: writeBE32(h2))
    bytes.append(contentsOf: writeBE32(h3))
    bytes.append(contentsOf: writeBE32(h4))
    return Data(bytes)
}

// ============================================================================
// Helper: Convert Data to lowercase hex string
// ============================================================================

private func toHex(_ data: Data) -> String {
    return data.map { String(format: "%02x", $0) }.joined()
}

// ============================================================================
// Public API: One-Shot sha1
// ============================================================================

/// Compute the SHA-1 digest of a `Data` value. Returns 20 bytes.
///
/// This is the one-shot API: hash a complete message in a single call.
///
/// SHA-1 is weakened (SHAttered attack, 2017) but remains safe for UUID v5
/// and Git. For new security applications, use SHA-256 or SHA-3.
///
/// FIPS 180-4 test vectors:
///
///     sha1(Data())           -> da39a3ee5e6b4b0d3255bfef95601890afd80709
///     sha1("abc".data(...))  -> a9993e364706816aba3e25717850c26c9cd0d89d
///
/// - Parameter data: The bytes to hash.
/// - Returns: A 20-byte `Data` containing the SHA-1 digest.
public func sha1(_ data: Data) -> Data {
    let bytes = [UInt8](data)
    let padded = pad(bytes)
    var h0 = INIT_H.0, h1 = INIT_H.1, h2 = INIT_H.2, h3 = INIT_H.3, h4 = INIT_H.4

    var offset = 0
    while offset < padded.count {
        (h0, h1, h2, h3, h4) = compress(h0, h1, h2, h3, h4, block: padded, blockOffset: offset)
        offset += 64
    }

    return stateToData(h0, h1, h2, h3, h4)
}

// ============================================================================
// Public API: Hex Variant
// ============================================================================

/// Compute the SHA-1 digest and return it as a 40-character lowercase
/// hexadecimal string.
///
/// - Parameter data: The bytes to hash.
/// - Returns: A 40-character lowercase hex string.
public func sha1Hex(_ data: Data) -> String {
    return toHex(sha1(data))
}

// ============================================================================
// Public API: Streaming SHA1Hasher
// ============================================================================
//
// When the full message is not available at once -- e.g., reading a large
// file in chunks -- the streaming API allows incremental updates.
//
// Internally, we keep:
//   - state: the five-word running hash
//   - buffer: bytes not yet forming a complete 64-byte block
//   - byteCount: total bytes fed so far (needed for the padding length)

/// A streaming SHA-1 hasher that accepts data in multiple chunks.
///
/// ```swift
/// var hasher = SHA1Hasher()
/// hasher.update(Data("ab".utf8))
/// hasher.update(Data("c".utf8))
/// hasher.hexDigest()  // "a9993e364706816aba3e25717850c26c9cd0d89d"
/// ```
///
/// Multiple `update()` calls are equivalent to a single `sha1(allData)`.
public struct SHA1Hasher: Sendable {
    private var _h0: UInt32
    private var _h1: UInt32
    private var _h2: UInt32
    private var _h3: UInt32
    private var _h4: UInt32
    private var _buffer: [UInt8]
    private var _byteCount: Int

    /// Initialize a new SHA-1 hasher with the standard initial state.
    public init() {
        _h0 = INIT_H.0
        _h1 = INIT_H.1
        _h2 = INIT_H.2
        _h3 = INIT_H.3
        _h4 = INIT_H.4
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
            (_h0, _h1, _h2, _h3, _h4) = compress(
                _h0, _h1, _h2, _h3, _h4,
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

    /// Return the 20-byte SHA-1 digest of all data fed so far.
    ///
    /// Non-destructive: the internal state is not modified.
    ///
    /// - Returns: A 20-byte `Data` containing the SHA-1 digest.
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

        var h0 = _h0, h1 = _h1, h2 = _h2, h3 = _h3, h4 = _h4
        var offset = 0
        while offset < tail.count {
            (h0, h1, h2, h3, h4) = compress(h0, h1, h2, h3, h4, block: tail, blockOffset: offset)
            offset += 64
        }

        return stateToData(h0, h1, h2, h3, h4)
    }

    /// Return the 40-character lowercase hex string of the digest.
    public func hexDigest() -> String {
        return toHex(digest())
    }

    /// Return an independent copy of the current hasher.
    ///
    /// Useful for computing multiple digests from a common prefix.
    public func copy() -> SHA1Hasher {
        var other = SHA1Hasher()
        other._h0 = _h0
        other._h1 = _h1
        other._h2 = _h2
        other._h3 = _h3
        other._h4 = _h4
        other._buffer = _buffer
        other._byteCount = _byteCount
        return other
    }
}
