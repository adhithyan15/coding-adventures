// Argon2i.swift -- Argon2i (RFC 9106), the data-independent Argon2 variant.
//
// Argon2i generates reference-block indices from a deterministic public
// address stream: double-G(0, compress(0, input_block)) where input_block
// carries (pass r, lane, slice sl, m', t_total, TYPE_I, counter) in its
// first seven words.  Because the indices don't depend on the password, the
// memory-access pattern is independent of the secret -- giving side-channel
// resistance at the cost of Argon2d's GPU/ASIC hardening.  For
// general-purpose password hashing prefer Argon2id.
//
// Reference: https://datatracker.ietf.org/doc/html/rfc9106
// See also:  code/specs/KD03-argon2.md
//
// Almost every line of this file is shared with Argon2d; the only real
// difference is fill_segment, which pulls J1/J2 from a synthesised address
// stream rather than from the previous block's payload.

import Foundation
import Blake2b

public enum Argon2i {
    static let blockSize: Int = 1024
    static let blockWords: Int = 128
    static let syncPoints: Int = 4
    public static let argon2Version: Int = 0x13
    static let typeI: Int = 1
    static let addressesPerBlock: Int = 128   // = blockWords

    static let mask32: UInt64 = 0xFFFFFFFF

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

    @inline(__always)
    static func rotr64(_ x: UInt64, _ n: UInt64) -> UInt64 {
        return (x >> n) | (x << (64 - n))
    }

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

    static func compress(_ x: [UInt64], _ y: [UInt64]) -> [UInt64] {
        var r = [UInt64](repeating: 0, count: blockWords)
        for i in 0..<blockWords { r[i] = x[i] ^ y[i] }
        var q = r
        for i in 0..<8 { permutationP(&q, i * 16) }
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

    static func blockToBytes(_ block: [UInt64]) -> [UInt8] {
        var out = [UInt8](repeating: 0, count: blockSize)
        for i in 0..<blockWords {
            let w = block[i]
            for j in 0..<8 { out[i * 8 + j] = UInt8((w >> (8 * j)) & 0xFF) }
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
        return [UInt8(u & 0xFF), UInt8((u >> 8) & 0xFF),
                UInt8((u >> 16) & 0xFF), UInt8((u >> 24) & 0xFF)]
    }

    static func blake2bLong(_ t: Int, _ x: [UInt8]) throws -> [UInt8] {
        precondition(t > 0, "H' output length must be positive")
        var input = le32(t); input.append(contentsOf: x)
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

    static func indexAlpha(_ j1: UInt64, _ r: Int, _ sl: Int, _ c: Int,
                           _ sameLane: Bool, _ q: Int, _ slLen: Int) -> Int {
        let w: Int
        let start: Int
        if r == 0 && sl == 0 { w = c - 1; start = 0 }
        else if r == 0 {
            if sameLane      { w = sl * slLen + c - 1 }
            else if c == 0   { w = sl * slLen - 1 }
            else             { w = sl * slLen }
            start = 0
        } else {
            if sameLane      { w = q - slLen + c - 1 }
            else if c == 0   { w = q - slLen - 1 }
            else             { w = q - slLen }
            start = ((sl + 1) * slLen) % q
        }
        let wU = UInt64(w)
        let x = (j1 &* j1) >> 32
        let y = (wU &* x) >> 32
        let rel = Int(UInt64(w) &- 1 &- y)
        return (start + rel) % q
    }

    // ------------------------------------------------------------------
    // fill_segment (Argon2i-specific).
    //
    // J1/J2 come from a deterministic address stream: `address_block` is
    // refreshed every 128 columns by computing compress(0, compress(0,
    // input_block)), with input_block[6] bumped each time.  The stream is
    // independent of the password -- that's the whole point of Argon2i.
    // ------------------------------------------------------------------
    static func fillSegment(_ memory: inout [[[UInt64]]],
                            _ r: Int, _ lane: Int, _ sl: Int,
                            _ q: Int, _ slLen: Int, _ p: Int,
                            _ mPrime: Int, _ tTotal: Int) {
        var input = [UInt64](repeating: 0, count: blockWords)
        var address = [UInt64](repeating: 0, count: blockWords)
        let zero = [UInt64](repeating: 0, count: blockWords)
        input[0] = UInt64(r)
        input[1] = UInt64(lane)
        input[2] = UInt64(sl)
        input[3] = UInt64(mPrime)
        input[4] = UInt64(tTotal)
        input[5] = UInt64(typeI)

        func refresh() {
            input[6] &+= 1
            let z = compress(zero, input)
            address = compress(zero, z)
        }

        let startingC = (r == 0 && sl == 0) ? 2 : 0
        if startingC != 0 { refresh() }

        for i in startingC..<slLen {
            if i % addressesPerBlock == 0 && !(r == 0 && sl == 0 && i == 2) {
                refresh()
            }
            let col = sl * slLen + i
            let prevCol = col == 0 ? q - 1 : col - 1
            let prevBlock = memory[lane][prevCol]

            let pseudoRand = address[i % addressesPerBlock]
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

    public static func argon2i(password: [UInt8], salt: [UInt8],
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
        h0Input.append(contentsOf: le32(typeI))
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
            memory[i][0] = bytesToBlock(try blake2bLong(blockSize, seed0))
            memory[i][1] = bytesToBlock(try blake2bLong(blockSize, seed1))
        }

        for r in 0..<t {
            for sl in 0..<syncPoints {
                for lane in 0..<p {
                    fillSegment(&memory, r, lane, sl, q, slLen, p, mPrime, t)
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

    public static func argon2iHex(password: [UInt8], salt: [UInt8],
                                  timeCost: Int, memoryCost: Int,
                                  parallelism: Int, tagLength: Int,
                                  key: [UInt8] = [], associatedData: [UInt8] = [],
                                  version: Int = argon2Version) throws -> String {
        let raw = try argon2i(password: password, salt: salt,
                              timeCost: timeCost, memoryCost: memoryCost,
                              parallelism: parallelism, tagLength: tagLength,
                              key: key, associatedData: associatedData,
                              version: version)
        return raw.map { String(format: "%02x", $0) }.joined()
    }
}
