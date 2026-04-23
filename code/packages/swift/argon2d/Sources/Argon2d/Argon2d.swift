// Argon2d.swift -- Argon2d (RFC 9106), the data-dependent Argon2 variant.
//
// Argon2d picks every reference block from the low 64 bits of the previously
// computed block.  That means the memory-access pattern is correlated with
// the password, which maximises GPU/ASIC resistance but leaks a timing side
// channel.  Use Argon2d only when side-channel attacks are NOT in the threat
// model (proof-of-work schemes, etc.).  For password hashing prefer Argon2id.
//
// Reference: https://datatracker.ietf.org/doc/html/rfc9106
// See also:  code/specs/KD03-argon2.md
//
// SWIFT 64-BIT NOTES
// ------------------
// Swift's UInt64 has native wrapping operators `&+`, `&*`, `&<<`, `&>>`.
// We use `&+`/`&*` in the G-mixer's add-multiply cross term and plain `^`,
// `|`, `&` for bitwise work.  The module is pure computation — no I/O, no
// unsafe blocks, no randomness.  All byte packing is little-endian, matching
// the RFC.

import Foundation
import Blake2b

public enum Argon2d {
    // ------------------------------------------------------------------
    // Constants (RFC 9106 §3)
    // ------------------------------------------------------------------
    static let blockSize: Int = 1024            // bytes per Argon2 block
    static let blockWords: Int = 128            // 64-bit words per block
    static let syncPoints: Int = 4              // slices per pass
    public static let argon2Version: Int = 0x13 // only v1.3 is approved
    static let typeD: Int = 0                   // primitive type code

    static let mask32: UInt64 = 0xFFFFFFFF

    // ------------------------------------------------------------------
    // Errors
    // ------------------------------------------------------------------
    public enum ValidationError: Error, Equatable {
        case passwordTooLong
        case saltTooShort
        case saltTooLong
        case keyTooLong
        case associatedDataTooLong
        case tagLengthTooSmall
        case tagLengthTooLarge
        case parallelismOutOfRange
        case memoryCostTooSmall
        case memoryCostTooLarge
        case timeCostTooSmall
        case unsupportedVersion
    }

    // ------------------------------------------------------------------
    // _rotr64 -- right-rotate a 64-bit word by n bits.
    // ------------------------------------------------------------------
    @inline(__always)
    static func rotr64(_ x: UInt64, _ n: UInt64) -> UInt64 {
        return (x >> n) | (x << (64 - n))
    }

    // ------------------------------------------------------------------
    // G-mixer (RFC 9106 §3.5).
    //
    // Identical to BLAKE2's quarter-round except each add carries an
    // additional `2 * trunc32(a) * trunc32(b)` cross-term.  That term is
    // what makes "prune and extend" attacks quadratic in block size.
    // ------------------------------------------------------------------
    @inline(__always)
    static func gb(_ v: inout [UInt64], _ a: Int, _ b: Int, _ c: Int, _ d: Int) {
        var va = v[a], vb = v[b], vc = v[c], vd = v[d]

        va = va &+ vb &+ (2 &* (va & mask32) &* (vb & mask32))
        vd = rotr64(vd ^ va, 32)
        vc = vc &+ vd &+ (2 &* (vc & mask32) &* (vd & mask32))
        vb = rotr64(vb ^ vc, 24)
        va = va &+ vb &+ (2 &* (va & mask32) &* (vb & mask32))
        vd = rotr64(vd ^ va, 16)
        vc = vc &+ vd &+ (2 &* (vc & mask32) &* (vd & mask32))
        vb = rotr64(vb ^ vc, 63)

        v[a] = va; v[b] = vb; v[c] = vc; v[d] = vd
    }

    // Eight G-rounds over a 16-word slice.  Four "column" then four
    // "diagonal" rounds -- the classic double-round layout.
    @inline(__always)
    static func permutationP(_ v: inout [UInt64], _ off: Int) {
        gb(&v, off + 0, off + 4, off +  8, off + 12)
        gb(&v, off + 1, off + 5, off +  9, off + 13)
        gb(&v, off + 2, off + 6, off + 10, off + 14)
        gb(&v, off + 3, off + 7, off + 11, off + 15)
        gb(&v, off + 0, off + 5, off + 10, off + 15)
        gb(&v, off + 1, off + 6, off + 11, off + 12)
        gb(&v, off + 2, off + 7, off +  8, off + 13)
        gb(&v, off + 3, off + 4, off +  9, off + 14)
    }

    // ------------------------------------------------------------------
    // Compression G:  r := x XOR y; row-pass, column-pass; r XOR q.
    //
    // The column pass gathers 16-word "columns" (two words per row), permutes
    // them, and scatters back -- flipping the 8x8 matrix of 128-bit registers
    // along its diagonal.
    // ------------------------------------------------------------------
    static func compress(_ x: [UInt64], _ y: [UInt64]) -> [UInt64] {
        var r = [UInt64](repeating: 0, count: blockWords)
        for i in 0..<blockWords { r[i] = x[i] ^ y[i] }
        var q = r

        // Row pass
        for i in 0..<8 { permutationP(&q, i * 16) }

        // Column pass
        var col = [UInt64](repeating: 0, count: 16)
        for c in 0..<8 {
            for rr in 0..<8 {
                col[2 * rr]     = q[rr * 16 + 2 * c]
                col[2 * rr + 1] = q[rr * 16 + 2 * c + 1]
            }
            permutationP(&col, 0)
            for rr in 0..<8 {
                q[rr * 16 + 2 * c]     = col[2 * rr]
                q[rr * 16 + 2 * c + 1] = col[2 * rr + 1]
            }
        }

        var out = [UInt64](repeating: 0, count: blockWords)
        for i in 0..<blockWords { out[i] = r[i] ^ q[i] }
        return out
    }

    // ------------------------------------------------------------------
    // Byte <-> block helpers.  Argon2 is little-endian everywhere.
    // ------------------------------------------------------------------
    static func blockToBytes(_ block: [UInt64]) -> [UInt8] {
        var out = [UInt8](repeating: 0, count: blockSize)
        for i in 0..<blockWords {
            let w = block[i]
            for j in 0..<8 {
                out[i * 8 + j] = UInt8((w >> (8 * j)) & 0xFF)
            }
        }
        return out
    }

    static func bytesToBlock(_ data: [UInt8]) -> [UInt64] {
        var out = [UInt64](repeating: 0, count: blockWords)
        for i in 0..<blockWords {
            var w: UInt64 = 0
            for j in 0..<8 { w |= UInt64(data[i * 8 + j]) << (8 * j) }
            out[i] = w
        }
        return out
    }

    static func le32(_ n: Int) -> [UInt8] {
        let u = UInt32(truncatingIfNeeded: n)
        return [
            UInt8(u & 0xFF),
            UInt8((u >> 8) & 0xFF),
            UInt8((u >> 16) & 0xFF),
            UInt8((u >> 24) & 0xFF),
        ]
    }

    // ------------------------------------------------------------------
    // blake2b_long -- Argon2's variable-length hash H' (RFC 9106 §3.3).
    //
    // Output = concat of 32-byte halves of repeated 64-byte BLAKE2b chains,
    // with the last call sized to fit exactly.  The initial input prefix is
    // LE32(t) || x.  For t <= 64 the standard BLAKE2b already produces
    // exactly t bytes in one call.
    // ------------------------------------------------------------------
    static func blake2bLong(_ t: Int, _ x: [UInt8]) throws -> [UInt8] {
        precondition(t > 0, "H' output length must be positive")
        var input = le32(t)
        input.append(contentsOf: x)

        if t <= 64 {
            return try Blake2b.hash(input, options: .init(digestSize: t))
        }

        let r = (t + 31) / 32 - 2
        var v = try Blake2b.hash(input, options: .init(digestSize: 64))
        var out = Array(v[0..<32])
        if r > 1 {
            for _ in 1..<r {
                v = try Blake2b.hash(v, options: .init(digestSize: 64))
                out.append(contentsOf: v[0..<32])
            }
        }
        let finalSize = t - 32 * r
        v = try Blake2b.hash(v, options: .init(digestSize: finalSize))
        out.append(contentsOf: v)
        return out
    }

    // ------------------------------------------------------------------
    // index_alpha (RFC 9106 §3.4.1.1) -- map J1 to an eligible column.
    //
    // Window [start, start+W) names which already-filled columns may be
    // referenced; J1 biases the pick toward recent blocks via (W * x) >> 32.
    // ------------------------------------------------------------------
    static func indexAlpha(_ j1: UInt64, _ r: Int, _ sl: Int, _ c: Int,
                           _ sameLane: Bool, _ q: Int, _ slLen: Int) -> Int {
        let w: Int
        let start: Int
        if r == 0 && sl == 0 {
            w = c - 1
            start = 0
        } else if r == 0 {
            if sameLane {
                w = sl * slLen + c - 1
            } else if c == 0 {
                w = sl * slLen - 1
            } else {
                w = sl * slLen
            }
            start = 0
        } else {
            if sameLane {
                w = q - slLen + c - 1
            } else if c == 0 {
                w = q - slLen - 1
            } else {
                w = q - slLen
            }
            start = ((sl + 1) * slLen) % q
        }
        let wU = UInt64(w)
        let x = (j1 &* j1) >> 32
        let y = (wU &* x) >> 32
        let rel = Int(UInt64(w) &- 1 &- y)
        return (start + rel) % q
    }

    // ------------------------------------------------------------------
    // fill_segment -- one (pass, slice, lane) segment.  Argon2d is entirely
    // data-dependent: the 64 low / high bits of the previous block supply
    // J1 / J2.
    // ------------------------------------------------------------------
    static func fillSegment(_ memory: inout [[[UInt64]]],
                            _ r: Int, _ lane: Int, _ sl: Int,
                            _ q: Int, _ slLen: Int, _ p: Int) {
        let startingC = (r == 0 && sl == 0) ? 2 : 0
        for i in startingC..<slLen {
            let col = sl * slLen + i
            let prevCol = col == 0 ? q - 1 : col - 1
            let prevBlock = memory[lane][prevCol]

            let pseudoRand = prevBlock[0]
            let j1 = pseudoRand & mask32
            let j2 = (pseudoRand >> 32) & mask32

            var lPrime = lane
            if !(r == 0 && sl == 0) { lPrime = Int(j2) % p }

            let zPrime = indexAlpha(j1, r, sl, i, lPrime == lane, q, slLen)
            let refBlock = memory[lPrime][zPrime]

            let newBlock = compress(prevBlock, refBlock)
            if r == 0 {
                memory[lane][col] = newBlock
            } else {
                var merged = [UInt64](repeating: 0, count: blockWords)
                let existing = memory[lane][col]
                for k in 0..<blockWords { merged[k] = existing[k] ^ newBlock[k] }
                memory[lane][col] = merged
            }
        }
    }

    // ------------------------------------------------------------------
    // Parameter validation (RFC 9106 §3.1 bounds).
    // ------------------------------------------------------------------
    static func validate(password: [UInt8], salt: [UInt8], timeCost: Int,
                         memoryCost: Int, parallelism: Int, tagLength: Int,
                         key: [UInt8], ad: [UInt8], version: Int) throws {
        if UInt64(password.count) > mask32 { throw ValidationError.passwordTooLong }
        if salt.count < 8  { throw ValidationError.saltTooShort }
        if UInt64(salt.count) > mask32 { throw ValidationError.saltTooLong }
        if UInt64(key.count)  > mask32 { throw ValidationError.keyTooLong }
        if UInt64(ad.count)   > mask32 { throw ValidationError.associatedDataTooLong }
        if tagLength < 4   { throw ValidationError.tagLengthTooSmall }
        if UInt64(tagLength) > mask32 { throw ValidationError.tagLengthTooLarge }
        if parallelism < 1 || parallelism > 0xFFFFFF {
            throw ValidationError.parallelismOutOfRange
        }
        if memoryCost < 8 * parallelism { throw ValidationError.memoryCostTooSmall }
        if UInt64(memoryCost) > mask32 { throw ValidationError.memoryCostTooLarge }
        if timeCost < 1 { throw ValidationError.timeCostTooSmall }
        if version != argon2Version { throw ValidationError.unsupportedVersion }
    }

    // ------------------------------------------------------------------
    // argon2d -- compute the Argon2d tag (RFC 9106 §3).
    //
    // - password:       secret input, arbitrary byte length
    // - salt:           >= 8 bytes, 16+ recommended
    // - timeCost:       passes `t`, >= 1
    // - memoryCost:     KiB `m`, >= 8 * parallelism
    // - parallelism:    lanes `p`, 1..=2^24-1
    // - tagLength:      output bytes `T`, >= 4
    // - key, ad:        optional MAC / context binding, default empty
    // - version:        only 0x13 supported
    // ------------------------------------------------------------------
    public static func argon2d(password: [UInt8], salt: [UInt8],
                               timeCost: Int, memoryCost: Int,
                               parallelism: Int, tagLength: Int,
                               key: [UInt8] = [], associatedData: [UInt8] = [],
                               version: Int = argon2Version) throws -> [UInt8] {
        try validate(password: password, salt: salt, timeCost: timeCost,
                     memoryCost: memoryCost, parallelism: parallelism,
                     tagLength: tagLength, key: key, ad: associatedData,
                     version: version)

        let segmentLength = memoryCost / (syncPoints * parallelism)
        let mPrime = segmentLength * syncPoints * parallelism
        let q = mPrime / parallelism
        let slLen = segmentLength
        let p = parallelism
        let t = timeCost

        var h0Input = [UInt8]()
        h0Input.append(contentsOf: le32(p))
        h0Input.append(contentsOf: le32(tagLength))
        h0Input.append(contentsOf: le32(memoryCost))
        h0Input.append(contentsOf: le32(t))
        h0Input.append(contentsOf: le32(version))
        h0Input.append(contentsOf: le32(typeD))
        h0Input.append(contentsOf: le32(password.count)); h0Input.append(contentsOf: password)
        h0Input.append(contentsOf: le32(salt.count));     h0Input.append(contentsOf: salt)
        h0Input.append(contentsOf: le32(key.count));      h0Input.append(contentsOf: key)
        h0Input.append(contentsOf: le32(associatedData.count))
        h0Input.append(contentsOf: associatedData)
        let h0 = try Blake2b.hash(h0Input, options: .init(digestSize: 64))

        var memory: [[[UInt64]]] = Array(
            repeating: Array(repeating: [UInt64](repeating: 0, count: blockWords),
                             count: q),
            count: p
        )
        for i in 0..<p {
            var seed0 = h0; seed0.append(contentsOf: le32(0)); seed0.append(contentsOf: le32(i))
            var seed1 = h0; seed1.append(contentsOf: le32(1)); seed1.append(contentsOf: le32(i))
            let b0 = try blake2bLong(blockSize, seed0)
            let b1 = try blake2bLong(blockSize, seed1)
            memory[i][0] = bytesToBlock(b0)
            memory[i][1] = bytesToBlock(b1)
        }

        for r in 0..<t {
            for sl in 0..<syncPoints {
                for lane in 0..<p {
                    fillSegment(&memory, r, lane, sl, q, slLen, p)
                }
            }
        }

        var finalBlock = memory[0][q - 1]
        for lane in 1..<p {
            let last = memory[lane][q - 1]
            for k in 0..<blockWords { finalBlock[k] ^= last[k] }
        }

        return try blake2bLong(tagLength, blockToBytes(finalBlock))
    }

    // argon2d_hex -- lowercase hex convenience wrapper.
    public static func argon2dHex(password: [UInt8], salt: [UInt8],
                                  timeCost: Int, memoryCost: Int,
                                  parallelism: Int, tagLength: Int,
                                  key: [UInt8] = [], associatedData: [UInt8] = [],
                                  version: Int = argon2Version) throws -> String {
        let raw = try argon2d(password: password, salt: salt,
                              timeCost: timeCost, memoryCost: memoryCost,
                              parallelism: parallelism, tagLength: tagLength,
                              key: key, associatedData: associatedData,
                              version: version)
        return raw.map { String(format: "%02x", $0) }.joined()
    }
}
