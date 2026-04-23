// Blake2b.swift -- BLAKE2b cryptographic hash function (RFC 7693), from scratch.
//
// BLAKE2b is a modern hash faster than MD5 on 64-bit hardware and as secure
// as SHA-3.  Variable output length (1..64 bytes), single-pass keyed mode,
// salt/personalization parameters.  Sequential mode only; tree hashing,
// BLAKE2s, BLAKE2bp, BLAKE2sp, BLAKE2Xb, and BLAKE3 are out of scope -- see
// `code/specs/HF06-blake2b.md`.
//
// Swift has native `UInt64` with `&+` wrapping add and a stdlib
// `.rotated(right:)`... no wait, it doesn't; we write our own rotate below.
// Everything else is a direct transliteration of the RFC.

import Foundation

public enum Blake2b {
    public static let blockSize = 128
    public static let maxDigest = 64
    public static let maxKey = 64

    // Initial Hash Values -- identical to SHA-512 (fractional parts of sqrt of
    // the first eight primes).
    static let iv: [UInt64] = [
        0x6A09E667F3BCC908,
        0xBB67AE8584CAA73B,
        0x3C6EF372FE94F82B,
        0xA54FF53A5F1D36F1,
        0x510E527FADE682D1,
        0x9B05688C2B3E6C1F,
        0x1F83D9ABFB41BD6B,
        0x5BE0CD19137E2179,
    ]

    // Ten message-schedule permutations.  Rounds 10/11 reuse rows 0/1.
    static let sigma: [[Int]] = [
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
        [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
        [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
        [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
        [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
        [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
        [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
        [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
        [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0],
    ]

    public enum ValidationError: Error, Equatable {
        case invalidDigestSize(Int)
        case keyTooLong(Int)
        case invalidSaltLength(Int)
        case invalidPersonalLength(Int)
    }

    public struct Options {
        public var digestSize: Int
        public var key: [UInt8]
        public var salt: [UInt8]
        public var personal: [UInt8]
        public init(digestSize: Int = 64,
                    key: [UInt8] = [],
                    salt: [UInt8] = [],
                    personal: [UInt8] = []) {
            self.digestSize = digestSize
            self.key = key
            self.salt = salt
            self.personal = personal
        }
    }

    static func validate(_ opts: Options) throws {
        if opts.digestSize < 1 || opts.digestSize > maxDigest {
            throw ValidationError.invalidDigestSize(opts.digestSize)
        }
        if opts.key.count > maxKey {
            throw ValidationError.keyTooLong(opts.key.count)
        }
        if !opts.salt.isEmpty && opts.salt.count != 16 {
            throw ValidationError.invalidSaltLength(opts.salt.count)
        }
        if !opts.personal.isEmpty && opts.personal.count != 16 {
            throw ValidationError.invalidPersonalLength(opts.personal.count)
        }
    }

    // Rotate right within a 64-bit word.
    @inline(__always)
    static func rotr(_ x: UInt64, _ n: UInt64) -> UInt64 {
        return (x >> n) | (x << (64 - n))
    }

    // Parse a 128-byte block as sixteen little-endian u64 words.
    static func parseBlock(_ block: [UInt8]) -> [UInt64] {
        var m = [UInt64](repeating: 0, count: 16)
        for i in 0..<16 {
            var w: UInt64 = 0
            for j in 0..<8 {
                w |= UInt64(block[i * 8 + j]) << (8 * j)
            }
            m[i] = w
        }
        return m
    }

    // BLAKE2b quarter-round G.  Rotation constants (R1..R4) = (32, 24, 16, 63).
    @inline(__always)
    static func mix(_ v: inout [UInt64], _ a: Int, _ b: Int, _ c: Int, _ d: Int, _ x: UInt64, _ y: UInt64) {
        v[a] = v[a] &+ v[b] &+ x
        v[d] = rotr(v[d] ^ v[a], 32)
        v[c] = v[c] &+ v[d]
        v[b] = rotr(v[b] ^ v[c], 24)
        v[a] = v[a] &+ v[b] &+ y
        v[d] = rotr(v[d] ^ v[a], 16)
        v[c] = v[c] &+ v[d]
        v[b] = rotr(v[b] ^ v[c], 63)
    }

    // Compression function F.  `t` is the 128-bit total byte count so far
    // (including the bytes of the current block).  `isFinal` must be true
    // iff this is the last compression call, which triggers the v[14]
    // inversion that domain-separates the final block.
    static func compress(state h: inout [UInt64], block: [UInt8], t: (UInt64, UInt64), isFinal: Bool) {
        let m = parseBlock(block)
        var v = [UInt64](repeating: 0, count: 16)
        for i in 0..<8 { v[i] = h[i] }
        for i in 0..<8 { v[i + 8] = iv[i] }
        v[12] ^= t.0
        v[13] ^= t.1
        if isFinal { v[14] ^= 0xFFFF_FFFF_FFFF_FFFF }

        for i in 0..<12 {
            let s = sigma[i % 10]
            mix(&v, 0, 4, 8, 12, m[s[0]], m[s[1]])
            mix(&v, 1, 5, 9, 13, m[s[2]], m[s[3]])
            mix(&v, 2, 6, 10, 14, m[s[4]], m[s[5]])
            mix(&v, 3, 7, 11, 15, m[s[6]], m[s[7]])
            mix(&v, 0, 5, 10, 15, m[s[8]], m[s[9]])
            mix(&v, 1, 6, 11, 12, m[s[10]], m[s[11]])
            mix(&v, 2, 7, 8, 13, m[s[12]], m[s[13]])
            mix(&v, 3, 4, 9, 14, m[s[14]], m[s[15]])
        }

        for i in 0..<8 { h[i] ^= v[i] ^ v[i + 8] }
    }

    // Build the parameter-block-XOR-ed starting state (sequential mode only,
    // fanout=1, depth=1).
    static func initialState(digestSize: Int, keyLen: Int, salt: [UInt8], personal: [UInt8]) -> [UInt64] {
        var p = [UInt8](repeating: 0, count: 64)
        p[0] = UInt8(digestSize)
        p[1] = UInt8(keyLen)
        p[2] = 1 // fanout
        p[3] = 1 // depth
        for i in 0..<salt.count { p[32 + i] = salt[i] }
        for i in 0..<personal.count { p[48 + i] = personal[i] }

        var state = iv
        for i in 0..<8 {
            var w: UInt64 = 0
            for j in 0..<8 { w |= UInt64(p[i * 8 + j]) << (8 * j) }
            state[i] ^= w
        }
        return state
    }

    // Streaming BLAKE2b hasher.  `digest()` is non-destructive; repeated
    // calls return the same bytes and the hasher stays usable for further
    // `update()` calls.
    public struct Hasher {
        var state: [UInt64]
        var buffer: [UInt8]
        var byteCount: UInt128Emulated
        let digestSize: Int

        public init(options opts: Options = Options()) throws {
            try validate(opts)
            self.digestSize = opts.digestSize
            self.state = initialState(digestSize: opts.digestSize,
                                      keyLen: opts.key.count,
                                      salt: opts.salt,
                                      personal: opts.personal)
            self.byteCount = UInt128Emulated()
            if opts.key.isEmpty {
                self.buffer = []
                self.buffer.reserveCapacity(blockSize)
            } else {
                var b = [UInt8](repeating: 0, count: blockSize)
                for i in 0..<opts.key.count { b[i] = opts.key[i] }
                self.buffer = b
            }
        }

        public mutating func update(_ data: [UInt8]) {
            buffer.append(contentsOf: data)
            while buffer.count > blockSize {
                byteCount.add(UInt64(blockSize))
                let block = Array(buffer[0..<blockSize])
                compress(state: &state, block: block, t: byteCount.split(), isFinal: false)
                buffer.removeFirst(blockSize)
            }
        }

        public func digest() -> [UInt8] {
            var s = state
            var padded = buffer
            if padded.count < blockSize {
                padded.append(contentsOf: [UInt8](repeating: 0, count: blockSize - padded.count))
            }
            var total = byteCount
            total.add(UInt64(buffer.count))
            compress(state: &s, block: padded, t: total.split(), isFinal: true)

            var out = [UInt8](repeating: 0, count: 64)
            for i in 0..<8 {
                let w = s[i]
                for j in 0..<8 {
                    out[i * 8 + j] = UInt8((w >> (8 * j)) & 0xFF)
                }
            }
            return Array(out[0..<digestSize])
        }

        public func hexDigest() -> String {
            return Self.bytesToHex(digest())
        }

        public func copy() -> Hasher { return self }

        static func bytesToHex(_ bytes: [UInt8]) -> String {
            var s = ""
            s.reserveCapacity(bytes.count * 2)
            for b in bytes {
                s.append(String(format: "%02x", b))
            }
            return s
        }
    }

    // One-shot BLAKE2b.  Raw bytes of length `digestSize`.
    public static func hash(_ data: [UInt8], options: Options = Options()) throws -> [UInt8] {
        var h = try Hasher(options: options)
        h.update(data)
        return h.digest()
    }

    public static func hashHex(_ data: [UInt8], options: Options = Options()) throws -> String {
        return Hasher.bytesToHex(try hash(data, options: options))
    }
}

// Minimal 128-bit counter used to feed the RFC's 128-bit byte-count field.
// Messages > 2^64 bytes are rare but the spec reserves two u64 slots, so we
// model the full width.
struct UInt128Emulated: Equatable {
    var low: UInt64 = 0
    var high: UInt64 = 0
    mutating func add(_ n: UInt64) {
        let (sum, carry) = low.addingReportingOverflow(n)
        low = sum
        if carry { high &+= 1 }
    }
    func split() -> (UInt64, UInt64) { return (low, high) }
}
