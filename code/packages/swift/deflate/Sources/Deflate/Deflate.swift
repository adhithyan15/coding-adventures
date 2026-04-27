// Deflate.swift — CMP05: DEFLATE lossless compression (1996)
// ============================================================================
//
// DEFLATE is the dominant general-purpose lossless compression algorithm,
// powering ZIP, gzip, PNG, and HTTP/2 HPACK header compression. It combines:
//
// 1. LZSS tokenization (CMP02) — replace repeated substrings with
//    back-references into a 4096-byte sliding window.
//
// 2. Dual canonical Huffman coding (DT27) — entropy-code the token stream
//    with two separate Huffman trees:
//    - LL tree:   literals (0-255), end-of-data (256), length codes (257-284)
//    - Dist tree: distance codes (0-23, for offsets 1-4096)
//
// ============================================================
// The Expanded LL Alphabet
// ============================================================
//
// DEFLATE merges literal bytes and match lengths into one alphabet:
//
//   Symbols 0-255:   literal byte values
//   Symbol  256:     end-of-data marker
//   Symbols 257-284: length codes (each covers a range via extra bits)
//
// Length codes use "extra bits": after emitting the Huffman code for a length
// symbol, a few raw bits specify the exact length within the symbol's range.
// This shrinks the length alphabet from 253 symbols (3-255) to 28 symbols.
//
// ============================================================
// Wire Format (CMP05)
// ============================================================
//
//   [4B] original_length    big-endian uint32
//   [2B] ll_entry_count     big-endian uint16
//   [2B] dist_entry_count   big-endian uint16 (0 if no matches)
//   [ll_entry_count × 3B]   (symbol uint16 BE, code_length uint8)
//   [dist_entry_count × 3B] same format
//   [remaining bytes]       LSB-first packed bit stream
//
// ============================================================
// Series
// ============================================================
//
//   CMP00 (LZ77,    1977) — Sliding-window backreferences.
//   CMP01 (LZ78,    1978) — Explicit dictionary (trie).
//   CMP02 (LZSS,    1982) — LZ77 + flag bits; no wasted literals.
//   CMP03 (LZW,     1984) — LZ78 + pre-initialized dict; powers GIF.
//   CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE.
//   CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.  ← this file
//

import Foundation
import HuffmanTree
import LZSS

// ---------------------------------------------------------------------------
// Length code table (LL symbols 257-284)
// ---------------------------------------------------------------------------
//
// Each entry: (symbol, base_length, extra_bits)

private let lengthTable: [(Int, Int, Int)] = [
    (257,   3, 0), (258,   4, 0), (259,   5, 0), (260,   6, 0),
    (261,   7, 0), (262,   8, 0), (263,   9, 0), (264,  10, 0),
    (265,  11, 1), (266,  13, 1), (267,  15, 1), (268,  17, 1),
    (269,  19, 2), (270,  23, 2), (271,  27, 2), (272,  31, 2),
    (273,  35, 3), (274,  43, 3), (275,  51, 3), (276,  59, 3),
    (277,  67, 4), (278,  83, 4), (279,  99, 4), (280, 115, 4),
    (281, 131, 5), (282, 163, 5), (283, 195, 5), (284, 227, 5),
]

private let lengthBase:  [Int: Int] = Dictionary(uniqueKeysWithValues: lengthTable.map { ($0.0, $0.1) })
private let lengthExtra: [Int: Int] = Dictionary(uniqueKeysWithValues: lengthTable.map { ($0.0, $0.2) })

// ---------------------------------------------------------------------------
// Distance code table (codes 0-23)
// ---------------------------------------------------------------------------
//
// Each entry: (code, base_dist, extra_bits)

private let distTable: [(Int, Int, Int)] = [
    ( 0,    1,  0), ( 1,    2,  0), ( 2,    3,  0), ( 3,    4,  0),
    ( 4,    5,  1), ( 5,    7,  1), ( 6,    9,  2), ( 7,   13,  2),
    ( 8,   17,  3), ( 9,   25,  3), (10,   33,  4), (11,   49,  4),
    (12,   65,  5), (13,   97,  5), (14,  129,  6), (15,  193,  6),
    (16,  257,  7), (17,  385,  7), (18,  513,  8), (19,  769,  8),
    (20, 1025,  9), (21, 1537,  9), (22, 2049, 10), (23, 3073, 10),
]

private let distBase:  [Int: Int] = Dictionary(uniqueKeysWithValues: distTable.map { ($0.0, $0.1) })
private let distExtra: [Int: Int] = Dictionary(uniqueKeysWithValues: distTable.map { ($0.0, $0.2) })

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

private func lengthSymbol(_ length: Int) -> Int {
    for (sym, base, extra) in lengthTable {
        let maxLen = base + (1 << extra) - 1
        if length <= maxLen { return sym }
    }
    return 284
}

private func distCode(_ offset: Int) -> Int {
    for (code, base, extra) in distTable {
        let maxDist = base + (1 << extra) - 1
        if offset <= maxDist { return code }
    }
    return 23
}

// ---------------------------------------------------------------------------
// Bit I/O
// ---------------------------------------------------------------------------

private struct BitBuilder {
    private var buf: UInt64 = 0
    private var bitPos: Int = 0
    private var out: [UInt8] = []

    mutating func writeBitString(_ s: String) {
        for ch in s {
            if ch == "1" { buf |= (1 << bitPos) }
            bitPos += 1
            if bitPos == 64 {
                for _ in 0..<8 {
                    out.append(UInt8(buf & 0xFF))
                    buf >>= 8
                }
                bitPos = 0
            }
        }
    }

    mutating func writeRawBitsLSB(_ val: Int, _ n: Int) {
        for i in 0..<n {
            if (val >> i) & 1 == 1 { buf |= (1 << bitPos) }
            bitPos += 1
            if bitPos == 64 {
                for _ in 0..<8 {
                    out.append(UInt8(buf & 0xFF))
                    buf >>= 8
                }
                bitPos = 0
            }
        }
    }

    mutating func flush() {
        var remaining = bitPos
        while remaining > 0 {
            out.append(UInt8(buf & 0xFF))
            buf >>= 8
            remaining = remaining >= 8 ? remaining - 8 : 0
        }
        bitPos = 0
    }

    func bytes() -> [UInt8] { out }
}

private func unpackBits(_ data: [UInt8]) -> [Int] {
    var bits: [Int] = []
    bits.reserveCapacity(data.count * 8)
    for byte in data {
        for i in 0..<8 {
            bits.append(Int((byte >> i) & 1))
        }
    }
    return bits
}

private func reconstructCanonicalCodes(_ lengths: [(Int, Int)]) -> [String: Int] {
    if lengths.isEmpty { return [:] }
    if lengths.count == 1 { return ["0": lengths[0].0] }
    var result: [String: Int] = [:]
    var code = 0
    var prevLen = lengths[0].1
    for (sym, codeLen) in lengths {
        if codeLen > prevLen {
            code <<= (codeLen - prevLen)
        }
        let bitStr = String(code, radix: 2).leftPadded(toLength: codeLen, with: "0")
        result[bitStr] = sym
        code += 1
        prevLen = codeLen
    }
    return result
}

private extension String {
    func leftPadded(toLength length: Int, with char: Character) -> String {
        guard self.count < length else { return self }
        return String(repeating: char, count: length - self.count) + self
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Deflate compression and decompression (CMP05).
public struct Deflate {

    // -----------------------------------------------------------------------
    // compress
    // -----------------------------------------------------------------------

    /// Compress data using DEFLATE (CMP05) and return wire-format bytes.
    ///
    /// - Parameter data: The raw bytes to compress.
    /// - Returns: Compressed bytes in CMP05 wire format.
    /// - Throws: `HuffmanTree.HuffmanError` if tree construction fails.
    public static func compress(_ data: [UInt8]) throws -> [UInt8] {
        let originalLength = data.count

        if originalLength == 0 {
            // Empty input: LL tree has only symbol 256 (end-of-data), code "0".
            var out: [UInt8] = []
            out.append(contentsOf: uint32BE(0))
            out.append(contentsOf: uint16BE(1)) // ll_entry_count = 1
            out.append(contentsOf: uint16BE(0)) // dist_entry_count = 0
            out.append(contentsOf: uint16BE(256)) // symbol = 256
            out.append(1) // code_length = 1
            out.append(0x00) // bit stream: "0"
            return out
        }

        // Pass 1: LZSS tokenization.
        let tokens = LZSS.encode(data, windowSize: 4096, maxMatch: 255, minMatch: 3)

        // Pass 2a: Tally frequencies.
        var llFreq: [Int: Int] = [:]
        var distFreq: [Int: Int] = [:]

        for tok in tokens {
            switch tok {
            case .literal(let b):
                llFreq[Int(b), default: 0] += 1
            case .match(let offset, let length):
                let sym = lengthSymbol(Int(length))
                llFreq[sym, default: 0] += 1
                let dc = distCode(Int(offset))
                distFreq[dc, default: 0] += 1
            }
        }
        llFreq[256, default: 0] += 1

        // Pass 2b: Build canonical Huffman trees.
        let llWeights = llFreq.map { (symbol: $0.key, frequency: $0.value) }
        let llTree = try HuffmanTree.build(llWeights)
        let llCodeTable = llTree.canonicalCodeTable() // [Int: String]

        var distCodeTable: [Int: String] = [:]
        if !distFreq.isEmpty {
            let distWeights = distFreq.map { (symbol: $0.key, frequency: $0.value) }
            let distTree = try HuffmanTree.build(distWeights)
            distCodeTable = distTree.canonicalCodeTable()
        }

        // Pass 2c: Encode token stream.
        var bb = BitBuilder()
        for tok in tokens {
            switch tok {
            case .literal(let b):
                guard let code = llCodeTable[Int(b)] else {
                    throw DeflateError.missingCode("literal \(b)")
                }
                bb.writeBitString(code)
            case .match(let offset, let length):
                let sym = lengthSymbol(Int(length))
                guard let code = llCodeTable[sym] else {
                    throw DeflateError.missingCode("length symbol \(sym)")
                }
                bb.writeBitString(code)
                let extra = lengthExtra[sym] ?? 0
                let extraVal = Int(length) - (lengthBase[sym] ?? 0)
                bb.writeRawBitsLSB(extraVal, extra)

                let dc = distCode(Int(offset))
                guard let dcode = distCodeTable[dc] else {
                    throw DeflateError.missingCode("dist code \(dc)")
                }
                bb.writeBitString(dcode)
                let dextra = distExtra[dc] ?? 0
                let dextraVal = Int(offset) - (distBase[dc] ?? 0)
                bb.writeRawBitsLSB(dextraVal, dextra)
            }
        }
        guard let eodCode = llCodeTable[256] else {
            throw DeflateError.missingCode("end-of-data (256)")
        }
        bb.writeBitString(eodCode)
        bb.flush()
        let packedBits = bb.bytes()

        // Assemble wire format.
        var llPairs = llCodeTable.map { ($0.key, $0.value.count) }
        llPairs.sort { $0.1 != $1.1 ? $0.1 < $1.1 : $0.0 < $1.0 }

        var distPairs = distCodeTable.map { ($0.key, $0.value.count) }
        distPairs.sort { $0.1 != $1.1 ? $0.1 < $1.1 : $0.0 < $1.0 }

        var out: [UInt8] = []
        out.reserveCapacity(8 + 3 * llPairs.count + 3 * distPairs.count + packedBits.count)
        out.append(contentsOf: uint32BE(originalLength))
        out.append(contentsOf: uint16BE(llPairs.count))
        out.append(contentsOf: uint16BE(distPairs.count))

        for (sym, len) in llPairs {
            out.append(contentsOf: uint16BE(sym))
            out.append(UInt8(len))
        }
        for (sym, len) in distPairs {
            out.append(contentsOf: uint16BE(sym))
            out.append(UInt8(len))
        }
        out.append(contentsOf: packedBits)

        return out
    }

    // -----------------------------------------------------------------------
    // decompress
    // -----------------------------------------------------------------------

    /// Decompress CMP05 wire-format data and return the original bytes.
    ///
    /// - Parameter data: Compressed bytes produced by `compress(_:)`.
    /// - Returns: Original uncompressed bytes.
    /// - Throws: `DeflateError` if the data is malformed.
    public static func decompress(_ data: [UInt8]) throws -> [UInt8] {
        guard data.count >= 8 else { return [] }

        let originalLength = Int(readUInt32BE(data, at: 0))
        let llEntryCount   = Int(readUInt16BE(data, at: 4))
        let distEntryCount = Int(readUInt16BE(data, at: 6))

        if originalLength == 0 { return [] }

        var off = 8

        // Parse LL code-length table.
        var llLengths: [(Int, Int)] = []
        for _ in 0..<llEntryCount {
            let sym   = Int(readUInt16BE(data, at: off))
            let clen  = Int(data[off + 2])
            llLengths.append((sym, clen))
            off += 3
        }

        // Parse dist code-length table.
        var distLengths: [(Int, Int)] = []
        for _ in 0..<distEntryCount {
            let sym   = Int(readUInt16BE(data, at: off))
            let clen  = Int(data[off + 2])
            distLengths.append((sym, clen))
            off += 3
        }

        // Reconstruct canonical codes.
        let llRevMap   = reconstructCanonicalCodes(llLengths)
        let distRevMap = reconstructCanonicalCodes(distLengths)

        // Unpack bit stream.
        let bits = unpackBits(Array(data[off...]))
        var bitPos = 0

        func readBits(_ n: Int) -> Int {
            var val = 0
            for i in 0..<n { val |= bits[bitPos + i] << i }
            bitPos += n
            return val
        }

        func nextHuffmanSymbol(_ revMap: [String: Int]) throws -> Int {
            var acc = ""
            while true {
                guard bitPos < bits.count else {
                    throw DeflateError.bitStreamExhausted
                }
                acc += bits[bitPos] == 1 ? "1" : "0"
                bitPos += 1
                if let sym = revMap[acc] { return sym }
            }
        }

        // Decode token stream.
        var output: [UInt8] = []
        output.reserveCapacity(originalLength)

        while true {
            let llSym = try nextHuffmanSymbol(llRevMap)

            if llSym == 256 {
                break // end-of-data
            } else if llSym < 256 {
                output.append(UInt8(llSym))
            } else {
                // Length code 257-284.
                let extra = lengthExtra[llSym] ?? 0
                let lengthVal = (lengthBase[llSym] ?? 0) + readBits(extra)

                let distSym = try nextHuffmanSymbol(distRevMap)
                let dextra = distExtra[distSym] ?? 0
                let distOffset = (distBase[distSym] ?? 0) + readBits(dextra)

                // Copy byte-by-byte (supports overlapping matches).
                let start = output.count - distOffset
                for i in 0..<lengthVal {
                    output.append(output[start + i])
                }
            }
        }

        return output
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    private static func uint32BE(_ n: Int) -> [UInt8] {
        let v = UInt32(n)
        return [UInt8(v >> 24), UInt8((v >> 16) & 0xFF), UInt8((v >> 8) & 0xFF), UInt8(v & 0xFF)]
    }

    private static func uint16BE(_ n: Int) -> [UInt8] {
        let v = UInt16(n)
        return [UInt8(v >> 8), UInt8(v & 0xFF)]
    }

    private static func readUInt32BE(_ data: [UInt8], at offset: Int) -> UInt32 {
        UInt32(data[offset]) << 24
            | UInt32(data[offset + 1]) << 16
            | UInt32(data[offset + 2]) << 8
            | UInt32(data[offset + 3])
    }

    private static func readUInt16BE(_ data: [UInt8], at offset: Int) -> UInt16 {
        UInt16(data[offset]) << 8 | UInt16(data[offset + 1])
    }
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

public enum DeflateError: Error, Equatable {
    case missingCode(String)
    case bitStreamExhausted
}
