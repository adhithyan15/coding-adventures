// Brotli.swift — CMP06: Brotli-inspired lossless compression (2013)
// ============================================================================
//
// Brotli is a compression algorithm developed at Google that builds on DEFLATE
// with three major innovations:
//
//   1. Context-dependent literal trees — instead of one Huffman tree for all
//      literals, Brotli assigns each literal to one of 4 context buckets based
//      on the preceding byte. Each bucket gets its own Huffman tree, exploiting
//      the fact that the letter following a space is very different from the
//      letter following another letter.
//
//   2. Insert-and-copy commands — instead of DEFLATE's flat stream of "literal"
//      and "back-reference" tokens, Brotli uses commands that bundle an insert
//      run (raw literals) with a copy operation (back-reference). The lengths
//      of both halves are encoded together in a single Huffman symbol.
//
//   3. Larger sliding window — 65535 bytes vs DEFLATE's 4096 bytes, allowing
//      matches across much longer distances.
//
// ============================================================
// Context Buckets
// ============================================================
//
// We assign each literal to one of 4 buckets based on the preceding byte:
//
//   bucket 0 — space or punctuation (0x00–0x2F, 0x3A–0x40, 0x5B–0x60, 0x7B–0xFF)
//   bucket 1 — digit ('0'–'9')
//   bucket 2 — uppercase letter ('A'–'Z')
//   bucket 3 — lowercase letter ('a'–'z')
//
// At the start of the stream (no previous byte), bucket 0 is used.
//
// ============================================================
// Insert-and-Copy Commands (ICC)
// ============================================================
//
// Every Brotli command has three parts:
//
//   Command {
//     insert_length:  uint — number of raw literal bytes that follow
//     copy_length:    uint — number of bytes to copy from history buffer
//     copy_distance:  uint — how far back (1 = immediately preceding byte)
//   }
//
// The insert_length and copy_length are encoded together as a single ICC
// Huffman symbol (one of 64 codes), plus extra bits to specify the exact
// value within the range.
//
// ============================================================
// Encoding Order (bit stream layout)
// ============================================================
//
// For each regular command (copyLength > 0):
//   1. [ICC symbol]               — Huffman code for (insert, copy) ranges
//   2. [insert_extra bits]        — LSB-first, select exact insert_length
//   3. [copy_extra bits]          — LSB-first, select exact copy_length
//   4. [insert_length literals]   — Huffman-coded per context bucket
//   5. [distance symbol]          — Huffman code for distance range
//   6. [dist_extra bits]          — LSB-first, select exact distance
//
// End of regular commands:
//   7. [ICC=63]                   — sentinel Huffman code
//   8. [flush literals, if any]   — trailing literals after last copy, Huffman-coded
//
// This design lets the decompressor emit flush literals simply by reading
// until output.count == originalLength after seeing ICC=63.
//
// ============================================================
// Wire Format (CMP06)
// ============================================================
//
//   Header (10 bytes):
//   [4B] original_length    — big-endian uint32
//   [1B] icc_entry_count    — uint8 (1–64)
//   [1B] dist_entry_count   — uint8 (0–32)
//   [1B] ctx0_entry_count   — uint8
//   [1B] ctx1_entry_count   — uint8
//   [1B] ctx2_entry_count   — uint8
//   [1B] ctx3_entry_count   — uint8
//
//   ICC code-length table   (icc_entry_count × 2 bytes)
//   Distance code-length    (dist_entry_count × 2 bytes)
//   Literal tree 0          (ctx0_entry_count × 3 bytes)
//   Literal tree 1          (ctx1_entry_count × 3 bytes)
//   Literal tree 2          (ctx2_entry_count × 3 bytes)
//   Literal tree 3          (ctx3_entry_count × 3 bytes)
//   Bit stream              (remaining bytes, LSB-first)
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
//   CMP05 (DEFLATE, 1996) — LZ77 + dual Huffman; ZIP/gzip/PNG/zlib.
//   CMP06 (Brotli,  2013) — Context modeling + insert-copy + large window.  ← this file
//

import Foundation
import HuffmanTree

// ---------------------------------------------------------------------------
// ICC Table (insert-copy codes, 64 entries)
// ---------------------------------------------------------------------------
//
// Each entry describes a range for insert_length and copy_length.
// The exact values within the range are specified by extra bits in the stream.
//
//   insert_length = insertBase + read_bits(insertExtra)
//   copy_length   = copyBase   + read_bits(copyExtra)
//
// Code 63 is the end-of-data sentinel: insert=0, copy=0.
//
// The table has "gaps" — e.g., copy_length=7 is not representable for
// insert=0 (code 2 covers copy=6 exactly, code 3 covers copy=8–9).
// The encoder resolves gaps by using the largest encodable copy ≤ requested,
// then emitting any remaining bytes as additional copy commands.

private struct ICCEntry {
    let insertBase:  Int
    let insertExtra: Int
    let copyBase:    Int
    let copyExtra:   Int
}

private let iccTable: [ICCEntry] = [
    // Codes 0–15: insert_base=0, insert_extra=0
    ICCEntry(insertBase: 0, insertExtra: 0, copyBase:   4, copyExtra: 0),  //  0
    ICCEntry(insertBase: 0, insertExtra: 0, copyBase:   5, copyExtra: 0),  //  1
    ICCEntry(insertBase: 0, insertExtra: 0, copyBase:   6, copyExtra: 0),  //  2
    ICCEntry(insertBase: 0, insertExtra: 0, copyBase:   8, copyExtra: 1),  //  3
    ICCEntry(insertBase: 0, insertExtra: 0, copyBase:  10, copyExtra: 1),  //  4
    ICCEntry(insertBase: 0, insertExtra: 0, copyBase:  14, copyExtra: 2),  //  5
    ICCEntry(insertBase: 0, insertExtra: 0, copyBase:  18, copyExtra: 2),  //  6
    ICCEntry(insertBase: 0, insertExtra: 0, copyBase:  26, copyExtra: 3),  //  7
    ICCEntry(insertBase: 0, insertExtra: 0, copyBase:  34, copyExtra: 3),  //  8
    ICCEntry(insertBase: 0, insertExtra: 0, copyBase:  50, copyExtra: 4),  //  9
    ICCEntry(insertBase: 0, insertExtra: 0, copyBase:  66, copyExtra: 4),  // 10
    ICCEntry(insertBase: 0, insertExtra: 0, copyBase:  98, copyExtra: 5),  // 11
    ICCEntry(insertBase: 0, insertExtra: 0, copyBase: 130, copyExtra: 5),  // 12
    ICCEntry(insertBase: 0, insertExtra: 0, copyBase: 194, copyExtra: 6),  // 13
    ICCEntry(insertBase: 0, insertExtra: 0, copyBase: 258, copyExtra: 7),  // 14
    ICCEntry(insertBase: 0, insertExtra: 0, copyBase: 514, copyExtra: 8),  // 15
    // Codes 16–23: insert_base=1, insert_extra=0
    ICCEntry(insertBase: 1, insertExtra: 0, copyBase:  4, copyExtra: 0),  // 16
    ICCEntry(insertBase: 1, insertExtra: 0, copyBase:  5, copyExtra: 0),  // 17
    ICCEntry(insertBase: 1, insertExtra: 0, copyBase:  6, copyExtra: 0),  // 18
    ICCEntry(insertBase: 1, insertExtra: 0, copyBase:  8, copyExtra: 1),  // 19
    ICCEntry(insertBase: 1, insertExtra: 0, copyBase: 10, copyExtra: 1),  // 20
    ICCEntry(insertBase: 1, insertExtra: 0, copyBase: 14, copyExtra: 2),  // 21
    ICCEntry(insertBase: 1, insertExtra: 0, copyBase: 18, copyExtra: 2),  // 22
    ICCEntry(insertBase: 1, insertExtra: 0, copyBase: 26, copyExtra: 3),  // 23
    // Codes 24–31: insert_base=2, insert_extra=0
    ICCEntry(insertBase: 2, insertExtra: 0, copyBase:  4, copyExtra: 0),  // 24
    ICCEntry(insertBase: 2, insertExtra: 0, copyBase:  5, copyExtra: 0),  // 25
    ICCEntry(insertBase: 2, insertExtra: 0, copyBase:  6, copyExtra: 0),  // 26
    ICCEntry(insertBase: 2, insertExtra: 0, copyBase:  8, copyExtra: 1),  // 27
    ICCEntry(insertBase: 2, insertExtra: 0, copyBase: 10, copyExtra: 1),  // 28
    ICCEntry(insertBase: 2, insertExtra: 0, copyBase: 14, copyExtra: 2),  // 29
    ICCEntry(insertBase: 2, insertExtra: 0, copyBase: 18, copyExtra: 2),  // 30
    ICCEntry(insertBase: 2, insertExtra: 0, copyBase: 26, copyExtra: 3),  // 31
    // Codes 32–39: insert_base=3, insert_extra=1
    ICCEntry(insertBase: 3, insertExtra: 1, copyBase:  4, copyExtra: 0),  // 32
    ICCEntry(insertBase: 3, insertExtra: 1, copyBase:  5, copyExtra: 0),  // 33
    ICCEntry(insertBase: 3, insertExtra: 1, copyBase:  6, copyExtra: 0),  // 34
    ICCEntry(insertBase: 3, insertExtra: 1, copyBase:  8, copyExtra: 1),  // 35
    ICCEntry(insertBase: 3, insertExtra: 1, copyBase: 10, copyExtra: 1),  // 36
    ICCEntry(insertBase: 3, insertExtra: 1, copyBase: 14, copyExtra: 2),  // 37
    ICCEntry(insertBase: 3, insertExtra: 1, copyBase: 18, copyExtra: 2),  // 38
    ICCEntry(insertBase: 3, insertExtra: 1, copyBase: 26, copyExtra: 3),  // 39
    // Codes 40–47: insert_base=5, insert_extra=2
    ICCEntry(insertBase: 5, insertExtra: 2, copyBase:  4, copyExtra: 0),  // 40
    ICCEntry(insertBase: 5, insertExtra: 2, copyBase:  5, copyExtra: 0),  // 41
    ICCEntry(insertBase: 5, insertExtra: 2, copyBase:  6, copyExtra: 0),  // 42
    ICCEntry(insertBase: 5, insertExtra: 2, copyBase:  8, copyExtra: 1),  // 43
    ICCEntry(insertBase: 5, insertExtra: 2, copyBase: 10, copyExtra: 1),  // 44
    ICCEntry(insertBase: 5, insertExtra: 2, copyBase: 14, copyExtra: 2),  // 45
    ICCEntry(insertBase: 5, insertExtra: 2, copyBase: 18, copyExtra: 2),  // 46
    ICCEntry(insertBase: 5, insertExtra: 2, copyBase: 26, copyExtra: 3),  // 47
    // Codes 48–55: insert_base=9, insert_extra=3
    ICCEntry(insertBase: 9, insertExtra: 3, copyBase:  4, copyExtra: 0),  // 48
    ICCEntry(insertBase: 9, insertExtra: 3, copyBase:  5, copyExtra: 0),  // 49
    ICCEntry(insertBase: 9, insertExtra: 3, copyBase:  6, copyExtra: 0),  // 50
    ICCEntry(insertBase: 9, insertExtra: 3, copyBase:  8, copyExtra: 1),  // 51
    ICCEntry(insertBase: 9, insertExtra: 3, copyBase: 10, copyExtra: 1),  // 52
    ICCEntry(insertBase: 9, insertExtra: 3, copyBase: 14, copyExtra: 2),  // 53
    ICCEntry(insertBase: 9, insertExtra: 3, copyBase: 18, copyExtra: 2),  // 54
    ICCEntry(insertBase: 9, insertExtra: 3, copyBase: 26, copyExtra: 3),  // 55
    // Codes 56–62: insert_base=17, insert_extra=4
    ICCEntry(insertBase: 17, insertExtra: 4, copyBase:  4, copyExtra: 0),  // 56
    ICCEntry(insertBase: 17, insertExtra: 4, copyBase:  5, copyExtra: 0),  // 57
    ICCEntry(insertBase: 17, insertExtra: 4, copyBase:  6, copyExtra: 0),  // 58
    ICCEntry(insertBase: 17, insertExtra: 4, copyBase:  8, copyExtra: 1),  // 59
    ICCEntry(insertBase: 17, insertExtra: 4, copyBase: 10, copyExtra: 1),  // 60
    ICCEntry(insertBase: 17, insertExtra: 4, copyBase: 14, copyExtra: 2),  // 61
    ICCEntry(insertBase: 17, insertExtra: 4, copyBase: 18, copyExtra: 2),  // 62
    // Code 63: end-of-data sentinel
    ICCEntry(insertBase: 0, insertExtra: 0, copyBase: 0, copyExtra: 0),   // 63
]

// Maximum insert length encodable by a single ICC code.
// Codes 56-62: insertBase=17, insertExtra=4 → max = 17 + (1<<4) - 1 = 32.
private let maxInsertPerICC = 32

// ---------------------------------------------------------------------------
// Distance Table (codes 0–31)
// ---------------------------------------------------------------------------
//
// Each entry: (base_distance, extra_bits)
//   distance = base + read_bits(extra)
//
// Codes 0–23 match CMP05 DEFLATE (up to 4096 bytes).
// Codes 24–31 extend the window to 65535 bytes.

private struct DistEntry {
    let base:  Int
    let extra: Int
}

private let distTable: [DistEntry] = [
    DistEntry(base:     1, extra:  0), DistEntry(base:     2, extra:  0),
    DistEntry(base:     3, extra:  0), DistEntry(base:     4, extra:  0),
    DistEntry(base:     5, extra:  1), DistEntry(base:     7, extra:  1),
    DistEntry(base:     9, extra:  2), DistEntry(base:    13, extra:  2),
    DistEntry(base:    17, extra:  3), DistEntry(base:    25, extra:  3),
    DistEntry(base:    33, extra:  4), DistEntry(base:    49, extra:  4),
    DistEntry(base:    65, extra:  5), DistEntry(base:    97, extra:  5),
    DistEntry(base:   129, extra:  6), DistEntry(base:   193, extra:  6),
    DistEntry(base:   257, extra:  7), DistEntry(base:   385, extra:  7),
    DistEntry(base:   513, extra:  8), DistEntry(base:   769, extra:  8),
    DistEntry(base:  1025, extra:  9), DistEntry(base:  1537, extra:  9),
    DistEntry(base:  2049, extra: 10), DistEntry(base:  3073, extra: 10),
    DistEntry(base:  4097, extra: 11), DistEntry(base:  6145, extra: 11),
    DistEntry(base:  8193, extra: 12), DistEntry(base: 12289, extra: 12),
    DistEntry(base: 16385, extra: 13), DistEntry(base: 24577, extra: 13),
    DistEntry(base: 32769, extra: 14), DistEntry(base: 49153, extra: 14),
]

// ---------------------------------------------------------------------------
// Context function
// ---------------------------------------------------------------------------
//
// Maps the last emitted byte (nil = start-of-stream) to a context bucket 0–3.
//
// The choice of bucket captures statistical structure in natural language:
//   - After a space or punct, we're likely starting a new word.
//   - After a digit, the next byte is likely another digit.
//   - After a letter, the next byte is likely another letter.

private func literalContext(_ p1: UInt8?) -> Int {
    guard let b = p1 else { return 0 }
    if b >= UInt8(ascii: "a") && b <= UInt8(ascii: "z") { return 3 }
    if b >= UInt8(ascii: "A") && b <= UInt8(ascii: "Z") { return 2 }
    if b >= UInt8(ascii: "0") && b <= UInt8(ascii: "9") { return 1 }
    return 0
}

// ---------------------------------------------------------------------------
// ICC code lookup helpers
// ---------------------------------------------------------------------------
//
// Finding the right ICC code for a given (insertLength, copyLength) pair:
//
//   1. The ICC table has gaps in copy coverage (e.g., copy=7 is not
//      representable for insert=0 because code 2 covers exactly 6,
//      code 3 covers 8–9). We resolve this by finding the largest
//      encodable copy ≤ requested and adjusting the match accordingly.
//
//   2. If insertLength > maxInsertPerICC (32), excess bytes become flush
//      literals (emitted after the sentinel).

/// Find the largest copy length ≤ `requested` that has a valid ICC code
/// for the given `insertLen`. Returns the adjusted copy length.
private func bestICCCopy(insertLen: Int, copyLen: Int) -> Int {
    var best = 0
    for code in 0..<63 {
        let e = iccTable[code]
        let maxIns = e.insertBase + (1 << e.insertExtra) - 1
        guard insertLen >= e.insertBase && insertLen <= maxIns else { continue }
        let copyMax = e.copyBase + (1 << e.copyExtra) - 1
        if copyLen >= e.copyBase && copyLen <= copyMax {
            return copyLen // exact match
        }
        if copyMax <= copyLen && copyMax > best {
            best = copyMax
        }
    }
    return best >= 4 ? best : 4  // minimum match length
}

/// Find the ICC code (0–62) that exactly covers (insertLen, copyLen).
/// Precondition: use bestICCCopy() to ensure the pair is representable.
private func iccCodeFor(insertLen: Int, copyLen: Int) -> Int {
    for code in 0..<63 {
        let e = iccTable[code]
        let maxIns  = e.insertBase + (1 << e.insertExtra) - 1
        let maxCopy = e.copyBase   + (1 << e.copyExtra)   - 1
        if insertLen >= e.insertBase && insertLen <= maxIns &&
           copyLen   >= e.copyBase   && copyLen   <= maxCopy {
            return code
        }
    }
    // Fallback: find a copy-only code (insert=0) for this copy length.
    for code in 0..<16 {
        let e = iccTable[code]
        let maxCopy = e.copyBase + (1 << e.copyExtra) - 1
        if copyLen >= e.copyBase && copyLen <= maxCopy { return code }
    }
    return 0
}

// ---------------------------------------------------------------------------
// Distance code lookup
// ---------------------------------------------------------------------------

private func distCodeFor(_ distance: Int) -> Int {
    for (code, entry) in distTable.enumerated() {
        let maxDist = entry.base + (1 << entry.extra) - 1
        if distance <= maxDist { return code }
    }
    return distTable.count - 1
}

// ---------------------------------------------------------------------------
// LZ matching
// ---------------------------------------------------------------------------
//
// Find the longest match in the sliding window for the bytes at `pos`.
// Window size: 65535 bytes. Minimum match length: 4. Maximum: 258.
//
// O(n²) scan — correct and easy to follow for an educational implementation.
// Production Brotli uses hash chains for O(1) amortized lookup.

private let windowSize  = 65535
private let minMatch    = 4
private let maxMatchLen = 258

private func findLongestMatch(data: [UInt8], pos: Int) -> (distance: Int, length: Int) {
    let n = data.count
    guard pos + minMatch <= n else { return (0, 0) }

    let windowStart = max(0, pos - windowSize)
    var bestLen = 0
    var bestDist = 0

    var start = pos - 1
    while start >= windowStart {
        var matchLen = 0
        let maxLen = min(maxMatchLen, n - pos)
        while matchLen < maxLen && data[start + matchLen] == data[pos + matchLen] {
            matchLen += 1
        }
        if matchLen > bestLen {
            bestLen  = matchLen
            bestDist = pos - start
            if bestLen == maxMatchLen { break }
        }
        start -= 1
    }

    return bestLen >= minMatch ? (bestDist, bestLen) : (0, 0)
}

// ---------------------------------------------------------------------------
// Command structure
// ---------------------------------------------------------------------------
//
// Represents one insert-and-copy command (regular commands only).
// The final sentinel and flush literals are handled separately.

private struct BrotliCommand {
    let insertLength:  Int
    let copyLength:    Int
    let copyDistance:  Int
    let literals:      [UInt8]
}

// ---------------------------------------------------------------------------
// Pass 1: LZ matching → commands + flush literals
// ---------------------------------------------------------------------------
//
// Strategy:
//   - Scan forward, accumulating literals in `insertBuf`.
//   - When a match of length ≥ 4 is found AND insertBuf.count ≤ maxInsertPerICC:
//     · Emit an insert-and-copy command.
//     · Handle ICC gaps by clamping copy length to the largest representable value.
//   - Otherwise, accumulate another literal.
//   - After all input is consumed:
//     · Remaining insertBuf bytes become `flushLiterals` (emitted after sentinel).
//
// Why flush literals instead of a final "insert-only" command?
//   The ICC table has no "insert_only" code — every ICC code with
//   copyLength > 0 wastes at least a distance code in the stream.
//   Flush literals (emitted after the sentinel ICC=63) cost zero extra
//   overhead: the decompressor reads them until output == originalLength.

private func buildCommands(data: [UInt8]) -> (commands: [BrotliCommand], flushLiterals: [UInt8]) {
    var commands: [BrotliCommand] = []
    var insertBuf: [UInt8] = []
    var pos = 0
    let n = data.count

    while pos < n {
        let (dist, length) = findLongestMatch(data: data, pos: pos)

        if length >= minMatch && insertBuf.count <= maxInsertPerICC {
            // Clamp copy length to the largest value representable by some
            // ICC code that also covers the current insert buffer size.
            let actualCopy = bestICCCopy(insertLen: insertBuf.count, copyLen: length)
            commands.append(BrotliCommand(
                insertLength:  insertBuf.count,
                copyLength:    actualCopy,
                copyDistance:  dist,
                literals:      insertBuf
            ))
            insertBuf = []
            pos += actualCopy
        } else {
            insertBuf.append(data[pos])
            pos += 1
        }
    }

    // Remaining bytes become flush literals (emitted after the sentinel).
    return (commands, insertBuf)
}

// ---------------------------------------------------------------------------
// BitBuilder — LSB-first bit packing
// ---------------------------------------------------------------------------
//
// Huffman codes are strings of '0'/'1' characters (MSB is the first character).
// They are packed into bytes LSB-first: the first bit of the first code lands
// in bit position 0 of byte 0.
//
// Example: bit string "1011" packed LSB-first → byte = 0b00001101 = 0x0D
//
// The builder accumulates bits in a 64-bit register, spilling 8 bytes to the
// output array every time the register is full.

private struct BitBuilder {
    private var buf:    UInt64 = 0
    private var bitPos: Int    = 0
    private var out:    [UInt8] = []

    mutating func writeBitString(_ s: String) {
        for ch in s {
            if ch == "1" { buf |= (UInt64(1) << bitPos) }
            bitPos += 1
            if bitPos == 64 { spill() }
        }
    }

    mutating func writeRawBitsLSB(_ val: Int, _ n: Int) {
        for i in 0..<n {
            if (val >> i) & 1 == 1 { buf |= (UInt64(1) << bitPos) }
            bitPos += 1
            if bitPos == 64 { spill() }
        }
    }

    mutating func flush() {
        var remaining = bitPos
        while remaining > 0 {
            out.append(UInt8(buf & 0xFF))
            buf >>= 8
            if remaining >= 8 { remaining -= 8 } else { remaining = 0 }
        }
        bitPos = 0
    }

    func bytes() -> [UInt8] { out }

    private mutating func spill() {
        for _ in 0..<8 {
            out.append(UInt8(buf & 0xFF))
            buf >>= 8
        }
        bitPos = 0
    }
}

// ---------------------------------------------------------------------------
// Bit unpacking
// ---------------------------------------------------------------------------
//
// Convert a byte array into a flat array of 0/1 integers, LSB-first.
// Bit 0 of byte 0 becomes index 0 in the output.

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

// ---------------------------------------------------------------------------
// Canonical code reconstruction (for decompressor)
// ---------------------------------------------------------------------------
//
// Given a sorted list of (symbol, code_length) pairs, reconstruct the
// canonical Huffman decode table (bit string → symbol).
// Single-symbol case: the one symbol is always encoded as "0" (1 bit).

private func buildDecodeTable(_ lengths: [(Int, Int)]) -> [String: Int] {
    if lengths.isEmpty { return [:] }
    if lengths.count == 1 { return ["0": lengths[0].0] }
    var result: [String: Int] = [:]
    var code    = 0
    var prevLen = lengths[0].1
    for (sym, codeLen) in lengths {
        if codeLen > prevLen { code <<= (codeLen - prevLen) }
        let bits   = String(code, radix: 2)
        let padded = String(repeating: "0", count: max(0, codeLen - bits.count)) + bits
        result[padded] = sym
        code   += 1
        prevLen = codeLen
    }
    return result
}

// ---------------------------------------------------------------------------
// Endian helpers
// ---------------------------------------------------------------------------

private func uint32BE(_ n: Int) -> [UInt8] {
    let v = UInt32(n)
    return [UInt8(v >> 24), UInt8((v >> 16) & 0xFF), UInt8((v >> 8) & 0xFF), UInt8(v & 0xFF)]
}

private func readUInt32BE(_ data: [UInt8], at offset: Int) -> UInt32 {
    UInt32(data[offset]) << 24
        | UInt32(data[offset + 1]) << 16
        | UInt32(data[offset + 2]) << 8
        | UInt32(data[offset + 3])
}

// ---------------------------------------------------------------------------
// Sort code table into wire-format order
// ---------------------------------------------------------------------------
//
// The wire format stores (symbol, code_length) pairs sorted by
// (code_length ASC, symbol ASC), enabling canonical code reconstruction.

private func sortedPairs(_ table: [Int: String]) -> [(Int, Int)] {
    var pairs = table.map { ($0.key, $0.value.count) }
    pairs.sort { $0.1 != $1.1 ? $0.1 < $1.1 : $0.0 < $1.0 }
    return pairs
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Brotli-inspired compression and decompression (CMP06).
///
/// Captures Brotli's three key innovations:
///   - Context-dependent literal trees (4 buckets)
///   - Insert-and-copy commands (ICC)
///   - 65535-byte sliding window
///
/// The static dictionary from RFC 7932 is omitted.
public struct Brotli {

    // -----------------------------------------------------------------------
    // compress
    // -----------------------------------------------------------------------

    /// Compress data using the Brotli-inspired CMP06 algorithm.
    ///
    /// - Parameter data: Raw bytes to compress.
    /// - Returns: Compressed bytes in CMP06 wire format.
    /// - Throws: `HuffmanTree.HuffmanError` if tree construction fails;
    ///           `BrotliError` if the encoding state is inconsistent.
    public static func compress(_ data: [UInt8]) throws -> [UInt8] {
        let originalLength = data.count

        // ── Empty input special case ─────────────────────────────────────────
        //
        // Empty input: only ICC sentinel (code 63), encoded as "0".
        // Wire: 10 header + 2 ICC entry + 1 bit-stream byte = 13 bytes.
        if originalLength == 0 {
            return [
                0x00, 0x00, 0x00, 0x00,  // original_length = 0
                0x01,                     // icc_entry_count = 1
                0x00,                     // dist_entry_count = 0
                0x00, 0x00, 0x00, 0x00,  // ctx0–3 entry counts = 0
                63, 1,                    // ICC entry: symbol=63, code_length=1
                0x00,                     // bit stream: "0" padded to byte
            ]
        }

        // ── Pass 1: LZ matching → commands + flush literals ──────────────────
        let (commands, flushLiterals) = buildCommands(data: data)

        // ── Pass 2a: Tally symbol frequencies ────────────────────────────────
        //
        // Walk through commands and flush literals, tracking context (last byte)
        // so we know which literal tree each byte belongs to.

        var litFreq: [[Int: Int]] = [[:], [:], [:], [:]]  // 4 context buckets
        var iccFreq:  [Int: Int] = [:]
        var distFreq: [Int: Int] = [:]

        // Simulated output for context tracking during frequency counting.
        var p1: UInt8? = nil           // last emitted byte
        var histBuf: [UInt8] = []      // for copy simulation

        for cmd in commands {
            let icc = iccCodeFor(insertLen: cmd.insertLength, copyLen: cmd.copyLength)
            iccFreq[icc, default: 0] += 1

            let dc = distCodeFor(cmd.copyDistance)
            distFreq[dc, default: 0] += 1

            for byte in cmd.literals {
                let ctx = literalContext(p1)
                litFreq[ctx][Int(byte), default: 0] += 1
                histBuf.append(byte)
                p1 = byte
            }

            // Simulate copy to advance context.
            let copyStart = histBuf.count - cmd.copyDistance
            for i in 0..<cmd.copyLength {
                let b = histBuf[copyStart + i]
                histBuf.append(b)
                p1 = b
            }
        }

        // Sentinel always present.
        iccFreq[63, default: 0] += 1

        // Tally flush literals (emitted after the sentinel).
        // p1 continues from wherever the regular commands left off.
        for byte in flushLiterals {
            let ctx = literalContext(p1)
            litFreq[ctx][Int(byte), default: 0] += 1
            p1 = byte
        }

        // ── Pass 2b: Build Huffman trees ─────────────────────────────────────

        let iccWeights  = iccFreq.map  { (symbol: $0.key, frequency: $0.value) }
        let iccTree     = try HuffmanTree.build(iccWeights)
        let iccCodeTbl  = iccTree.canonicalCodeTable()

        var distCodeTbl: [Int: String] = [:]
        if !distFreq.isEmpty {
            let distWeights = distFreq.map { (symbol: $0.key, frequency: $0.value) }
            let distTree    = try HuffmanTree.build(distWeights)
            distCodeTbl     = distTree.canonicalCodeTable()
        }

        var litCodeTbls: [[Int: String]] = [[:], [:], [:], [:]]
        for ctx in 0..<4 {
            if !litFreq[ctx].isEmpty {
                let weights = litFreq[ctx].map { (symbol: $0.key, frequency: $0.value) }
                let tree    = try HuffmanTree.build(weights)
                litCodeTbls[ctx] = tree.canonicalCodeTable()
            }
        }

        // ── Pass 2c: Encode ───────────────────────────────────────────────────
        //
        // Bit stream layout:
        //   [for each regular command]
        //     [ICC symbol] [insert_extras] [copy_extras]
        //     [literals via context trees]
        //     [dist symbol] [dist_extras]
        //   [ICC=63 sentinel]
        //   [flush literals via context trees]

        var bb = BitBuilder()
        p1     = nil
        var encHist: [UInt8] = []

        for cmd in commands {
            let icc  = iccCodeFor(insertLen: cmd.insertLength, copyLen: cmd.copyLength)
            let e    = iccTable[icc]
            guard let iccCode = iccCodeTbl[icc] else {
                throw BrotliError.missingCode("ICC \(icc)")
            }
            bb.writeBitString(iccCode)
            bb.writeRawBitsLSB(cmd.insertLength - e.insertBase, e.insertExtra)
            bb.writeRawBitsLSB(cmd.copyLength   - e.copyBase,   e.copyExtra)

            for byte in cmd.literals {
                let ctx = literalContext(p1)
                guard let code = litCodeTbls[ctx][Int(byte)] else {
                    throw BrotliError.missingCode("literal \(byte) ctx \(ctx)")
                }
                bb.writeBitString(code)
                encHist.append(byte)
                p1 = byte
            }

            let dc = distCodeFor(cmd.copyDistance)
            guard let dcCode = distCodeTbl[dc] else {
                throw BrotliError.missingCode("dist code \(dc)")
            }
            bb.writeBitString(dcCode)
            bb.writeRawBitsLSB(cmd.copyDistance - distTable[dc].base, distTable[dc].extra)

            let copyStart = encHist.count - cmd.copyDistance
            for i in 0..<cmd.copyLength {
                let b = encHist[copyStart + i]
                encHist.append(b)
                p1 = b
            }
        }

        // Emit sentinel ICC=63.
        guard let sentCode = iccCodeTbl[63] else {
            throw BrotliError.missingCode("sentinel ICC 63")
        }
        bb.writeBitString(sentCode)

        // Emit flush literals (if any) after the sentinel.
        for byte in flushLiterals {
            let ctx = literalContext(p1)
            guard let code = litCodeTbls[ctx][Int(byte)] else {
                throw BrotliError.missingCode("flush literal \(byte) ctx \(ctx)")
            }
            bb.writeBitString(code)
            p1 = byte
        }

        bb.flush()
        let packedBits = bb.bytes()

        // ── Assemble wire format ──────────────────────────────────────────────

        let iccPairs  = sortedPairs(iccCodeTbl)
        let distPairs = sortedPairs(distCodeTbl)
        let litPairs  = (0..<4).map { sortedPairs(litCodeTbls[$0]) }

        var out: [UInt8] = []
        // Header
        out.append(contentsOf: uint32BE(originalLength))
        out.append(UInt8(iccPairs.count))
        out.append(UInt8(distPairs.count))
        out.append(UInt8(litPairs[0].count))
        out.append(UInt8(litPairs[1].count))
        out.append(UInt8(litPairs[2].count))
        out.append(UInt8(litPairs[3].count))

        // ICC table: [symbol uint8][code_length uint8]
        for (sym, len) in iccPairs {
            out.append(UInt8(sym))
            out.append(UInt8(len))
        }
        // Distance table: [symbol uint8][code_length uint8]
        for (sym, len) in distPairs {
            out.append(UInt8(sym))
            out.append(UInt8(len))
        }
        // Literal tables: [symbol uint16 BE][code_length uint8]
        for pairs in litPairs {
            for (sym, len) in pairs {
                out.append(UInt8((sym >> 8) & 0xFF))
                out.append(UInt8(sym & 0xFF))
                out.append(UInt8(len))
            }
        }
        out.append(contentsOf: packedBits)
        return out
    }

    // -----------------------------------------------------------------------
    // decompress
    // -----------------------------------------------------------------------

    /// Decompress CMP06 wire-format data and return the original bytes.
    ///
    /// - Parameter data: Compressed bytes produced by `compress(_:)`.
    /// - Returns: Original uncompressed bytes.
    /// - Throws: `BrotliError` if the data is malformed.
    public static func decompress(_ data: [UInt8]) throws -> [UInt8] {
        guard data.count >= 10 else {
            throw BrotliError.truncatedHeader
        }

        let originalLength  = Int(readUInt32BE(data, at: 0))
        let iccEntryCount   = Int(data[4])
        let distEntryCount  = Int(data[5])
        let ctxCounts       = [Int(data[6]), Int(data[7]), Int(data[8]), Int(data[9])]

        if originalLength == 0 { return [] }

        var off = 10

        // ── Parse ICC code-length table ──────────────────────────────────────
        var iccLengths: [(Int, Int)] = []
        iccLengths.reserveCapacity(iccEntryCount)
        for _ in 0..<iccEntryCount {
            guard off + 2 <= data.count else { throw BrotliError.truncatedHeader }
            iccLengths.append((Int(data[off]), Int(data[off + 1])))
            off += 2
        }

        // ── Parse distance code-length table ─────────────────────────────────
        var distLengths: [(Int, Int)] = []
        distLengths.reserveCapacity(distEntryCount)
        for _ in 0..<distEntryCount {
            guard off + 2 <= data.count else { throw BrotliError.truncatedHeader }
            distLengths.append((Int(data[off]), Int(data[off + 1])))
            off += 2
        }

        // ── Parse four literal code-length tables ────────────────────────────
        var litLengths: [[(Int, Int)]] = [[], [], [], []]
        for ctx in 0..<4 {
            litLengths[ctx].reserveCapacity(ctxCounts[ctx])
            for _ in 0..<ctxCounts[ctx] {
                guard off + 3 <= data.count else { throw BrotliError.truncatedHeader }
                let sym  = Int(data[off]) << 8 | Int(data[off + 1])
                let clen = Int(data[off + 2])
                litLengths[ctx].append((sym, clen))
                off += 3
            }
        }

        // ── Reconstruct canonical decode tables ──────────────────────────────
        let iccRevMap  = buildDecodeTable(iccLengths)
        let distRevMap = buildDecodeTable(distLengths)
        let litRevMaps = litLengths.map { buildDecodeTable($0) }

        // ── Unpack bit stream ─────────────────────────────────────────────────
        let bits = unpackBits(Array(data[off...]))
        var bitPos = 0

        // Read n bits LSB-first, return the integer value.
        func readBits(_ n: Int) -> Int {
            var val = 0
            for i in 0..<n {
                guard bitPos < bits.count else { break }
                val |= bits[bitPos] << i
                bitPos += 1
            }
            return val
        }

        // Decode one Huffman symbol by accumulating bits until we find a
        // prefix-code match in the reverse map.
        func nextSymbol(_ revMap: [String: Int]) throws -> Int {
            var acc = ""
            while true {
                guard bitPos < bits.count else {
                    throw BrotliError.bitStreamExhausted
                }
                acc += bits[bitPos] == 1 ? "1" : "0"
                bitPos += 1
                if let sym = revMap[acc] { return sym }
                guard acc.count <= 20 else {
                    throw BrotliError.invalidBitStream
                }
            }
        }

        // ── Decode command stream ─────────────────────────────────────────────
        var output: [UInt8] = []
        output.reserveCapacity(originalLength)
        var p1: UInt8? = nil  // last emitted byte

        while true {
            let icc = try nextSymbol(iccRevMap)

            if icc == 63 {
                // End-of-data sentinel. Decode flush literals until we reach
                // originalLength. These were emitted by the encoder AFTER the
                // sentinel in the bit stream.
                while output.count < originalLength {
                    let ctx  = literalContext(p1)
                    let byte = try nextSymbol(litRevMaps[ctx])
                    output.append(UInt8(byte))
                    p1 = UInt8(byte)
                }
                break
            }

            let e            = iccTable[icc]
            let insertLength = e.insertBase + readBits(e.insertExtra)
            let copyLength   = e.copyBase   + readBits(e.copyExtra)

            // Decode and emit insert_length literal bytes.
            for _ in 0..<insertLength {
                let ctx  = literalContext(p1)
                let byte = try nextSymbol(litRevMaps[ctx])
                output.append(UInt8(byte))
                p1 = UInt8(byte)
            }

            // Decode and perform copy.
            let dc           = try nextSymbol(distRevMap)
            let distEntry    = distTable[dc]
            let distExtra    = readBits(distEntry.extra)
            let copyDistance = distEntry.base + distExtra

            let copyStart = output.count - copyDistance
            guard copyStart >= 0 else {
                throw BrotliError.invalidCopyDistance(copyDistance)
            }
            for i in 0..<copyLength {
                let b = output[copyStart + i]
                output.append(b)
                p1 = b
            }
        }

        return output
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    private static func readUInt32BE(_ data: [UInt8], at offset: Int) -> UInt32 {
        UInt32(data[offset])     << 24
            | UInt32(data[offset + 1]) << 16
            | UInt32(data[offset + 2]) << 8
            | UInt32(data[offset + 3])
    }
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors thrown by Brotli compress/decompress operations.
public enum BrotliError: Error, Equatable {
    case missingCode(String)
    case truncatedHeader
    case bitStreamExhausted
    case invalidBitStream
    case invalidCopyDistance(Int)
}
