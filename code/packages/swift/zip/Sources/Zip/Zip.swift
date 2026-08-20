// Zip.swift — CMP09: ZIP archive format (PKZIP, 1989).
//
// ZIP bundles one or more files into a single `.zip` archive, compressing
// each entry independently with DEFLATE (method 8) or storing it verbatim
// (method 0). The same format underlies Java JARs, Office Open XML (.docx),
// Android APKs, Python wheels, and many more.
//
// Architecture
// ────────────
//
//   ┌─────────────────────────────────────────────────────┐
//   │  [Local File Header + File Data]  ← entry 1         │
//   │  [Local File Header + File Data]  ← entry 2         │
//   │  ...                                                │
//   │  ══════════ Central Directory ══════════            │
//   │  [Central Dir Header]  ← entry 1 (has local offset)│
//   │  [Central Dir Header]  ← entry 2                   │
//   │  [End of Central Directory Record]                  │
//   └─────────────────────────────────────────────────────┘
//
// The dual-header design enables two workflows:
//   - Sequential write: append Local Headers one-by-one, write CD at the end.
//   - Random-access read: seek to EOCD at the end, read CD, jump to any entry.
//
// Wire constants (all integers little-endian):
//
//   LOCAL_SIG  = 0x04034B50
//   CD_SIG     = 0x02014B50
//   EOCD_SIG   = 0x06054B50
//   FLAGS      = 0x0800  (UTF-8 filename)
//
// DEFLATE inside ZIP
// ──────────────────
// ZIP method 8 stores raw RFC 1951 DEFLATE — no zlib wrapper.  This
// implementation uses fixed Huffman blocks (BTYPE=01) with the LZSS package
// for LZ77 match-finding (32 KB window, max match 255, min match 3).
//
// Series
// ──────
//   CMP02 (LZSS,    1982) — LZ77 + flag bits        ← dependency
//   CMP05 (DEFLATE, 1996) — LZ77 + Huffman           ← inlined here (raw RFC 1951)
//   CMP09 (ZIP,     1989) — DEFLATE container        ← this package

import LZSS

// ============================================================================
// CRC-32
// ============================================================================
//
// CRC-32 uses polynomial 0xEDB88320 (reflected form of 0x04C11DB7).
// It detects accidental corruption of decompressed content.
//
// Table-driven algorithm:
//   1. Build a 256-entry table: for each byte value, compute the CRC of that
//      single byte using 8 rounds of the polynomial.
//   2. For each input byte: crc = table[(crc XOR byte) & 0xFF] XOR (crc >> 8)
//
// The XOR with 0xFFFFFFFF at the start and end is the standard CRC-32 framing:
// it makes a stream of all-zeros produce a non-zero CRC.

private let crcTable: [UInt32] = {
    var table = [UInt32](repeating: 0, count: 256)
    for i in 0..<256 {
        var c = UInt32(i)
        for _ in 0..<8 {
            if c & 1 != 0 {
                c = 0xEDB8_8320 ^ (c >> 1)
            } else {
                c >>= 1
            }
        }
        table[i] = c
    }
    return table
}()

/// Compute CRC-32 over `data`, starting from `initial` (use 0 for a fresh hash,
/// or the previous result for an incremental update).
///
/// ```swift
/// assert(crc32("hello world".utf8Bytes) == 0x0D4A_1185)
/// ```
public func crc32(_ data: [UInt8], initial: UInt32 = 0) -> UInt32 {
    var crc = initial ^ 0xFFFF_FFFF
    for byte in data {
        crc = crcTable[Int((crc ^ UInt32(byte)) & 0xFF)] ^ (crc >> 8)
    }
    return crc ^ 0xFFFF_FFFF
}

// ============================================================================
// MS-DOS Date / Time Encoding
// ============================================================================
//
// ZIP stores timestamps in the 16-bit MS-DOS packed format inherited from FAT:
//
//   Time (16-bit): bits 15-11=hours, bits 10-5=minutes, bits 4-0=seconds/2
//   Date (16-bit): bits 15-9=year-1980, bits 8-5=month, bits 4-0=day
//
// The combined 32-bit value is (date << 16) | time.
// Year 0 in DOS time = 1980; max representable = 2107.

/// Encode a (year, month, day, hour, minute, second) tuple into the 32-bit
/// MS-DOS datetime used by ZIP Local and Central Directory headers.
public func dosDatetime(year: UInt16, month: UInt16, day: UInt16,
                        hour: UInt16 = 0, minute: UInt16 = 0, second: UInt16 = 0) -> UInt32 {
    let t = (hour << 11) | (minute << 5) | (second / 2)
    let d = ((year > 1980 ? year - 1980 : 0) << 9) | (month << 5) | day
    return (UInt32(d) << 16) | UInt32(t)
}

/// Fixed timestamp 1980-01-01 00:00:00 used when no real mtime is available.
/// date field: (0<<9)|(1<<5)|1 = 33 = 0x0021; time = 0 → 0x00210000.
public let dosEpoch: UInt32 = 0x0021_0000

/// Default and hard ceiling for raw RFC 1951 inflation: 256 MiB.
public let rawInflateMaxOutput = 268_435_456

/// Closed, payload-blind error taxonomy for the portable raw inflater.
public let rawInflateErrorCodes = [
    "invalid-output-limit",
    "unexpected-eof",
    "reserved-block-type",
    "stored-length-mismatch",
    "huffman-oversubscribed",
    "incomplete-code-length-tree",
    "incomplete-literal-length-tree",
    "incomplete-distance-tree",
    "repeat-without-previous",
    "repeat-overrun",
    "invalid-literal-length-symbol",
    "reserved-distance-symbol",
    "invalid-back-reference",
    "output-limit-exceeded",
]

/// A stable raw-inflate failure. Its description is exactly its payload-blind code.
public struct RawInflateError: Error, Equatable, CustomStringConvertible {
    public let code: String

    fileprivate init(_ code: String) {
        self.code = code
    }

    public var description: String { code }
}

/// Successful raw-inflate output plus the exact number of input bytes consumed.
public struct RawInflateResult: Equatable {
    public let output: [UInt8]
    public let bytesConsumed: Int

    public init(output: [UInt8], bytesConsumed: Int) {
        self.output = output
        self.bytesConsumed = bytesConsumed
    }
}

// ============================================================================
// RFC 1951 DEFLATE — Bit I/O
// ============================================================================
//
// RFC 1951 packs bits LSB-first within bytes. Huffman codes are sent MSB-first
// logically — so before writing a Huffman code we reverse its bits and then
// write the reversed value LSB-first.  Extra bits (length/distance extras,
// stored block headers) are written directly LSB-first without reversal.

/// Writes bits into a byte stream, LSB-first.
private struct BitWriter {
    var buf: UInt64 = 0
    var bits: Int = 0
    var out: [UInt8] = []

    /// Write `nbits` low bits of `value`, LSB-first (for extra bits and headers).
    mutating func writeLSB(_ value: UInt32, nbits: Int) {
        buf |= UInt64(value) << bits
        bits += nbits
        while bits >= 8 {
            out.append(UInt8(buf & 0xFF))
            buf >>= 8
            bits -= 8
        }
    }

    /// Write a Huffman code (MSB-first logically → bit-reverse then write LSB-first).
    mutating func writeHuffman(_ code: UInt32, nbits: Int) {
        // Reverse the top `nbits` bits of `code`.
        var c = code
        var reversed: UInt32 = 0
        for _ in 0..<nbits {
            reversed = (reversed << 1) | (c & 1)
            c >>= 1
        }
        writeLSB(reversed, nbits: nbits)
    }

    /// Align to the next byte boundary (used before stored blocks).
    mutating func align() {
        if bits > 0 {
            out.append(UInt8(buf & 0xFF))
            buf = 0
            bits = 0
        }
    }

    mutating func finish() -> [UInt8] {
        align()
        return out
    }
}

/// Reads bits from a byte slice, LSB-first.
private struct BitReader {
    let data: [UInt8]
    var pos: Int = 0
    var buf: UInt64 = 0
    var bits: Int = 0

    init(_ data: [UInt8]) { self.data = data }

    /// Fill the buffer with more bytes until we have at least `need` bits.
    mutating func fill(_ need: Int) -> Bool {
        while bits < need {
            guard pos < data.count else { return false }
            buf |= UInt64(data[pos]) << bits
            pos += 1
            bits += 8
        }
        return true
    }

    /// Read `nbits` bits LSB-first. Returns nil on EOF.
    mutating func readLSB(_ nbits: Int) -> UInt32? {
        guard nbits > 0 else { return 0 }
        guard fill(nbits) else { return nil }
        let mask = UInt64((1 << nbits) - 1)
        let val = UInt32(buf & mask)
        buf >>= nbits
        bits -= nbits
        return val
    }

    /// Discard any partial byte, aligning to the next byte boundary.
    mutating func align() {
        let discard = bits % 8
        if discard > 0 {
            buf >>= discard
            bits -= discard
        }
    }

}

// ============================================================================
// RFC 1951 DEFLATE — Fixed Huffman Tables
// ============================================================================
//
// RFC 1951 §3.2.6 specifies fixed (pre-defined) Huffman code lengths.
// Using fixed Huffman blocks (BTYPE=01) means we never transmit code tables —
// both encoder and decoder know the tables in advance.
//
// Literal/Length code lengths:
//   Symbols   0–143: 8-bit codes, starting at 0b00110000 (= 48)
//   Symbols 144–255: 9-bit codes, starting at 0b110010000 (= 400)
//   Symbols 256–279: 7-bit codes, starting at 0b0000000 (= 0)
//   Symbols 280–287: 8-bit codes, starting at 0b11000000 (= 192)
//
// Distance codes:
//   Symbols 0–29: 5-bit codes equal to the symbol number.

/// Returns the RFC 1951 fixed Huffman code and bit-width for a LL symbol 0-287.
private func fixedLLEncode(_ sym: Int) -> (code: UInt32, nbits: Int) {
    switch sym {
    case 0...143:   return (UInt32(0b0011_0000) + UInt32(sym), 8)
    case 144...255: return (UInt32(0b1_1001_0000) + UInt32(sym - 144), 9)
    case 256...279: return (UInt32(sym - 256), 7)
    case 280...287: return (UInt32(0b1100_0000) + UInt32(sym - 280), 8)
    default: fatalError("fixedLLEncode: invalid LL symbol \(sym)")
    }
}

// ============================================================================
// RFC 1951 DEFLATE — Length / Distance Tables
// ============================================================================
//
// Match lengths (3-255) map to LL symbols 257-284 + extra bits.
// Match distances (1-32768) map to distance codes 0-29 + extra bits.
// RFC 1951 §3.2.5: symbol 285 = length 258, 0 extra bits (special case).

/// (base_length, extra_bits) for LL symbols 257..=285.
private let lengthTable: [(base: Int, extra: Int)] = [
    (3, 0), (4, 0), (5, 0), (6, 0), (7, 0), (8, 0), (9, 0), (10, 0), // 257-264
    (11, 1), (13, 1), (15, 1), (17, 1),                                 // 265-268
    (19, 2), (23, 2), (27, 2), (31, 2),                                 // 269-272
    (35, 3), (43, 3), (51, 3), (59, 3),                                 // 273-276
    (67, 4), (83, 4), (99, 4), (115, 4),                               // 277-280
    (131, 5), (163, 5), (195, 5), (227, 5),                            // 281-284
    (258, 0),                                                           // 285
]

/// (base_offset, extra_bits) for distance codes 0..=29.
private let distTable: [(base: Int, extra: Int)] = [
    (1, 0), (2, 0), (3, 0), (4, 0),
    (5, 1), (7, 1), (9, 2), (13, 2),
    (17, 3), (25, 3), (33, 4), (49, 4),
    (65, 5), (97, 5), (129, 6), (193, 6),
    (257, 7), (385, 7), (513, 8), (769, 8),
    (1025, 9), (1537, 9), (2049, 10), (3073, 10),
    (4097, 11), (6145, 11), (8193, 12), (12289, 12),
    (16385, 13), (24577, 13),
]

/// Map a match length (3-258) to its RFC 1951 LL symbol index into lengthTable.
private func encodeLength(_ length: Int) -> (sym: Int, base: Int, extra: Int) {
    for i in stride(from: lengthTable.count - 1, through: 0, by: -1) {
        if length >= lengthTable[i].base {
            return (257 + i, lengthTable[i].base, lengthTable[i].extra)
        }
    }
    fatalError("encodeLength: unreachable for length=\(length)")
}

/// Map a match offset (1-32768) to its distance code index.
private func encodeDist(_ offset: Int) -> (code: Int, base: Int, extra: Int) {
    for i in stride(from: distTable.count - 1, through: 0, by: -1) {
        if offset >= distTable[i].base {
            return (i, distTable[i].base, distTable[i].extra)
        }
    }
    fatalError("encodeDist: unreachable for offset=\(offset)")
}

// ============================================================================
// RFC 1951 DEFLATE — Compress (fixed Huffman, BTYPE=01)
// ============================================================================
//
// Strategy:
//   1. Run LZ77/LZSS match-finding (window=32768, max match=255, min=3).
//   2. Emit a single BTYPE=01 (fixed Huffman) block containing the token stream.
//   3. Literal bytes → fixed LL Huffman code.
//   4. Match (offset, length) → length LL code + extra bits + distance code + extra.
//   5. End-of-block symbol (256) → fixed LL Huffman code.

/// Compress `data` to a raw RFC 1951 DEFLATE bit-stream (fixed Huffman, single block).
/// The output starts directly with the 3-bit block header — no zlib wrapper.
func deflateCompress(_ data: [UInt8]) -> [UInt8] {
    var bw = BitWriter()

    if data.isEmpty {
        // Empty stored block: BFINAL=1 BTYPE=00 + padding + LEN=0 + NLEN=0xFFFF.
        // Hardcoded as 5 bytes: [0x01, 0x00, 0x00, 0xFF, 0xFF]
        bw.writeLSB(1, nbits: 1) // BFINAL=1
        bw.writeLSB(0, nbits: 2) // BTYPE=00 (stored)
        bw.align()
        bw.writeLSB(0x0000, nbits: 16) // LEN=0
        bw.writeLSB(0xFFFF, nbits: 16) // NLEN=~0
        return bw.finish()
    }

    // Run LZ77/LZSS tokenizer. Window=32768 so every match maps into RFC 1951
    // distance table (1-32768); max_match=255 maps into length table (3-255).
    let tokens = encode(data, windowSize: 32768, maxMatch: 255, minMatch: 3)

    // Block header: BFINAL=1 (last block), BTYPE=01 (fixed Huffman).
    bw.writeLSB(1, nbits: 1) // BFINAL
    bw.writeLSB(1, nbits: 1) // BTYPE bit 0 = 1
    bw.writeLSB(0, nbits: 1) // BTYPE bit 1 = 0  →  BTYPE = 01

    for tok in tokens {
        switch tok {
        case .literal(let b):
            let (code, nbits) = fixedLLEncode(Int(b))
            bw.writeHuffman(code, nbits: nbits)

        case .match(let offset, let length):
            // ── Length ──────────────────────────────────────────────────────
            let (sym, basLen, extraLenBits) = encodeLength(Int(length))
            let (llCode, llNbits) = fixedLLEncode(sym)
            bw.writeHuffman(llCode, nbits: llNbits)
            if extraLenBits > 0 {
                bw.writeLSB(UInt32(Int(length) - basLen), nbits: extraLenBits)
            }

            // ── Distance ────────────────────────────────────────────────────
            let (distCode, baseDist, extraDistBits) = encodeDist(Int(offset))
            // Distance codes are 5-bit fixed codes equal to the code number.
            bw.writeHuffman(UInt32(distCode), nbits: 5)
            if extraDistBits > 0 {
                bw.writeLSB(UInt32(Int(offset) - baseDist), nbits: extraDistBits)
            }
        }
    }

    // End-of-block symbol (256).
    let (eobCode, eobNbits) = fixedLLEncode(256)
    bw.writeHuffman(eobCode, nbits: eobNbits)

    return bw.finish()
}

/// Compress bytes to a raw RFC 1951 stream with no ZIP, zlib, or gzip framing.
public func rawDeflate(_ data: [UInt8]) -> [UInt8] {
    deflateCompress(data)
}

// ============================================================================
// RFC 1951 DEFLATE — Decompress
// ============================================================================
//
// Handles stored, fixed-Huffman, dynamic-Huffman, and multi-block streams.
// Output is bounded by a validated caller ceiling no greater than 256 MiB.

private struct HuffmanTable {
    let codesByLength: [[UInt32: Int]]
    let maximumLength: Int

    func decode(_ reader: inout BitReader) throws -> Int {
        guard maximumLength > 0 else {
            throw RawInflateError("unexpected-eof")
        }
        var code: UInt32 = 0
        for length in 1...maximumLength {
            guard let bit = reader.readLSB(1) else {
                throw RawInflateError("unexpected-eof")
            }
            code = (code << 1) | bit
            if let symbol = codesByLength[length][code] {
                return symbol
            }
        }
        throw RawInflateError("unexpected-eof")
    }
}

private enum HuffmanCompleteness {
    case codeLength
    case literalLength
    case distance
}

private func buildHuffman(
    lengths: [Int],
    completeness: HuffmanCompleteness
) throws -> HuffmanTable {
    var counts = [Int](repeating: 0, count: 16)
    for length in lengths where length > 0 {
        guard length <= 15 else {
            throw RawInflateError("huffman-oversubscribed")
        }
        counts[length] += 1
    }

    var left = 1
    for length in 1...15 {
        left = (left * 2) - counts[length]
        if left < 0 {
            throw RawInflateError("huffman-oversubscribed")
        }
    }

    let symbolCount = counts.reduce(0, +)
    if left != 0 {
        switch completeness {
        case .codeLength:
            throw RawInflateError("incomplete-code-length-tree")
        case .literalLength:
            throw RawInflateError("incomplete-literal-length-tree")
        case .distance:
            let permittedSingleCode = symbolCount == 1 && counts[1] == 1
            guard symbolCount == 0 || permittedSingleCode else {
                throw RawInflateError("incomplete-distance-tree")
            }
        }
    }

    var nextCode = [Int](repeating: 0, count: 16)
    var code = 0
    for length in 1...15 {
        code = (code + counts[length - 1]) << 1
        nextCode[length] = code
    }

    var tables = Array(repeating: [UInt32: Int](), count: 16)
    var maximumLength = 0
    for (symbol, length) in lengths.enumerated() where length > 0 {
        tables[length][UInt32(nextCode[length])] = symbol
        nextCode[length] += 1
        maximumLength = max(maximumLength, length)
    }
    return HuffmanTable(codesByLength: tables, maximumLength: maximumLength)
}

private func fixedTables() throws -> (literalLength: HuffmanTable, distance: HuffmanTable) {
    var literalLengths = [Int](repeating: 0, count: 288)
    for symbol in 0...143 { literalLengths[symbol] = 8 }
    for symbol in 144...255 { literalLengths[symbol] = 9 }
    for symbol in 256...279 { literalLengths[symbol] = 7 }
    for symbol in 280...287 { literalLengths[symbol] = 8 }
    let distanceLengths = [Int](repeating: 5, count: 32)
    return (
        try buildHuffman(lengths: literalLengths, completeness: .literalLength),
        try buildHuffman(lengths: distanceLengths, completeness: .distance)
    )
}

private func readDynamicTables(
    _ reader: inout BitReader
) throws -> (literalLength: HuffmanTable, distance: HuffmanTable) {
    guard let rawLiteralCount = reader.readLSB(5),
          let rawDistanceCount = reader.readLSB(5),
          let rawCodeLengthCount = reader.readLSB(4) else {
        throw RawInflateError("unexpected-eof")
    }
    let literalCount = Int(rawLiteralCount) + 257
    let distanceCount = Int(rawDistanceCount) + 1
    let codeLengthCount = Int(rawCodeLengthCount) + 4
    guard literalCount <= 286 else {
        throw RawInflateError("invalid-literal-length-symbol")
    }

    let order = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15]
    var codeLengths = [Int](repeating: 0, count: 19)
    for index in 0..<codeLengthCount {
        guard let length = reader.readLSB(3) else {
            throw RawInflateError("unexpected-eof")
        }
        codeLengths[order[index]] = Int(length)
    }
    let codeLengthTable = try buildHuffman(lengths: codeLengths, completeness: .codeLength)

    let total = literalCount + distanceCount
    var lengths: [Int] = []
    lengths.reserveCapacity(total)
    while lengths.count < total {
        let symbol = try codeLengthTable.decode(&reader)
        switch symbol {
        case 0...15:
            lengths.append(symbol)
        case 16:
            guard let previous = lengths.last else {
                throw RawInflateError("repeat-without-previous")
            }
            guard let extra = reader.readLSB(2) else {
                throw RawInflateError("unexpected-eof")
            }
            let count = Int(extra) + 3
            guard lengths.count + count <= total else {
                throw RawInflateError("repeat-overrun")
            }
            lengths.append(contentsOf: repeatElement(previous, count: count))
        case 17:
            guard let extra = reader.readLSB(3) else {
                throw RawInflateError("unexpected-eof")
            }
            let count = Int(extra) + 3
            guard lengths.count + count <= total else {
                throw RawInflateError("repeat-overrun")
            }
            lengths.append(contentsOf: repeatElement(0, count: count))
        case 18:
            guard let extra = reader.readLSB(7) else {
                throw RawInflateError("unexpected-eof")
            }
            let count = Int(extra) + 11
            guard lengths.count + count <= total else {
                throw RawInflateError("repeat-overrun")
            }
            lengths.append(contentsOf: repeatElement(0, count: count))
        default:
            throw RawInflateError("unexpected-eof")
        }
    }

    let literalLengths = Array(lengths[..<literalCount])
    let distanceLengths = Array(lengths[literalCount...])
    guard literalLengths[256] != 0 else {
        throw RawInflateError("incomplete-literal-length-tree")
    }
    return (
        try buildHuffman(lengths: literalLengths, completeness: .literalLength),
        try buildHuffman(lengths: distanceLengths, completeness: .distance)
    )
}

private func ensureOutputCapacity(_ count: Int, outputCount: Int, maximum: Int) throws {
    guard count <= maximum - outputCount else {
        throw RawInflateError("output-limit-exceeded")
    }
}

private func decodeCompressedBlock(
    reader: inout BitReader,
    output: inout [UInt8],
    literalLength: HuffmanTable,
    distance: HuffmanTable,
    maximumOutput: Int
) throws {
    while true {
        let symbol = try literalLength.decode(&reader)
        switch symbol {
        case 0...255:
            try ensureOutputCapacity(1, outputCount: output.count, maximum: maximumOutput)
            output.append(UInt8(symbol))
        case 256:
            return
        case 257...285:
            let (baseLength, extraLengthBits) = lengthTable[symbol - 257]
            guard let extraLength = reader.readLSB(extraLengthBits) else {
                throw RawInflateError("unexpected-eof")
            }
            let length = baseLength + Int(extraLength)
            let distanceSymbol = try distance.decode(&reader)
            guard distanceSymbol < 30 else {
                throw RawInflateError("reserved-distance-symbol")
            }
            let (baseDistance, extraDistanceBits) = distTable[distanceSymbol]
            guard let extraDistance = reader.readLSB(extraDistanceBits) else {
                throw RawInflateError("unexpected-eof")
            }
            let backwardDistance = baseDistance + Int(extraDistance)
            guard backwardDistance > 0, backwardDistance <= output.count else {
                throw RawInflateError("invalid-back-reference")
            }
            try ensureOutputCapacity(length, outputCount: output.count, maximum: maximumOutput)
            for _ in 0..<length {
                try ensureOutputCapacity(1, outputCount: output.count, maximum: maximumOutput)
                output.append(output[output.count - backwardDistance])
            }
        default:
            throw RawInflateError("invalid-literal-length-symbol")
        }
    }
}

/// Inflate a raw RFC 1951 stream and report the exact final byte reached.
public func rawInflateCounted(
    _ data: [UInt8],
    maxOutput: Int = rawInflateMaxOutput
) throws -> RawInflateResult {
    guard maxOutput >= 0, maxOutput <= rawInflateMaxOutput else {
        throw RawInflateError("invalid-output-limit")
    }
    var reader = BitReader(data)
    var output: [UInt8] = []

    while true {
        guard let final = reader.readLSB(1), let type = reader.readLSB(2) else {
            throw RawInflateError("unexpected-eof")
        }
        switch type {
        case 0:
            reader.align()
            guard let length = reader.readLSB(16),
                  let complement = reader.readLSB(16) else {
                throw RawInflateError("unexpected-eof")
            }
            guard length == (complement ^ 0xFFFF) else {
                throw RawInflateError("stored-length-mismatch")
            }
            try ensureOutputCapacity(Int(length), outputCount: output.count, maximum: maxOutput)
            for _ in 0..<Int(length) {
                try ensureOutputCapacity(1, outputCount: output.count, maximum: maxOutput)
                guard let byte = reader.readLSB(8) else {
                    throw RawInflateError("unexpected-eof")
                }
                output.append(UInt8(byte))
            }
        case 1:
            let tables = try fixedTables()
            try decodeCompressedBlock(
                reader: &reader,
                output: &output,
                literalLength: tables.literalLength,
                distance: tables.distance,
                maximumOutput: maxOutput
            )
        case 2:
            let tables = try readDynamicTables(&reader)
            try decodeCompressedBlock(
                reader: &reader,
                output: &output,
                literalLength: tables.literalLength,
                distance: tables.distance,
                maximumOutput: maxOutput
            )
        default:
            throw RawInflateError("reserved-block-type")
        }
        if final == 1 {
            return RawInflateResult(output: output, bytesConsumed: reader.pos)
        }
    }
}

/// Inflate a raw RFC 1951 stream with a caller-lowerable output ceiling.
public func rawInflate(
    _ data: [UInt8],
    maxOutput: Int = rawInflateMaxOutput
) throws -> [UInt8] {
    try rawInflateCounted(data, maxOutput: maxOutput).output
}

/// Historical package-private name retained for source compatibility.
func deflateDecompress(_ data: [UInt8]) throws -> [UInt8] {
    try rawInflate(data)
}

// ============================================================================
// ZIP Error
// ============================================================================

/// Errors thrown by ZipWriter and ZipReader.
public enum ZipError: Error, Equatable {
    case malformed(String)
    case crcMismatch(String)
    case notFound(String)
    case unsupported(String)
}

// ============================================================================
// Little-endian helpers
// ============================================================================

private func le16(_ v: UInt16) -> [UInt8] {
    [UInt8(v & 0xFF), UInt8((v >> 8) & 0xFF)]
}

private func le32(_ v: UInt32) -> [UInt8] {
    [UInt8(v & 0xFF), UInt8((v >> 8) & 0xFF),
     UInt8((v >> 16) & 0xFF), UInt8((v >> 24) & 0xFF)]
}

private func readLE16(_ data: [UInt8], at offset: Int) -> UInt16? {
    guard offset + 2 <= data.count else { return nil }
    return UInt16(data[offset]) | (UInt16(data[offset + 1]) << 8)
}

private func readLE32(_ data: [UInt8], at offset: Int) -> UInt32? {
    guard offset + 4 <= data.count else { return nil }
    return UInt32(data[offset])
        | (UInt32(data[offset + 1]) << 8)
        | (UInt32(data[offset + 2]) << 16)
        | (UInt32(data[offset + 3]) << 24)
}

// ============================================================================
// ZIP Write — ZipWriter
// ============================================================================
//
// ZipWriter accumulates entries in memory: for each file it writes a Local
// File Header immediately, then the (possibly compressed) data, records the
// metadata needed for the Central Directory, and assembles the full archive
// on `finish()`.
//
// Auto-compression policy:
//   - Try DEFLATE. If the compressed output is smaller than the original,
//     use method=8 (DEFLATE).
//   - Otherwise use method=0 (Stored) — common for already-compressed formats.

private struct CdRecord {
    var name: [UInt8]
    var method: UInt16
    var crc: UInt32
    var compressedSize: UInt32
    var uncompressedSize: UInt32
    var localOffset: UInt32
    var externalAttrs: UInt32
}

/// Builds a ZIP archive incrementally in memory.
///
/// ```swift
/// var w = ZipWriter()
/// w.addFile("hello.txt", data: Array("hello".utf8), compress: true)
/// w.addDirectory("mydir/")
/// let bytes = w.finish()
/// ```
public struct ZipWriter {
    private var buf: [UInt8] = []
    private var entries: [CdRecord] = []

    public init() {}

    /// Add a file entry. Set `compress: true` to attempt DEFLATE compression.
    /// - Precondition: the archive must have fewer than 65535 entries (ZIP EOCD UInt16 limit).
    public mutating func addFile(_ name: String, data: [UInt8], compress: Bool = true) {
        precondition(entries.count < 65535, "zip: entry count exceeds ZIP UInt16 limit of 65535")
        addEntry(name, data: data, compress: compress, unixMode: 0o100_644)
    }

    /// Add a directory entry (name should end with '/').
    /// - Precondition: the archive must have fewer than 65535 entries (ZIP EOCD UInt16 limit).
    public mutating func addDirectory(_ name: String) {
        precondition(entries.count < 65535, "zip: entry count exceeds ZIP UInt16 limit of 65535")
        addEntry(name, data: [], compress: false, unixMode: 0o040_755)
    }

    /// Finish writing: append Central Directory and EOCD, return archive bytes.
    public mutating func finish() -> [UInt8] {
        let cdOffset = UInt32(buf.count)
        let numEntries = UInt16(entries.count)

        // ── Central Directory ─────────────────────────────────────────────
        let cdStart = buf.count
        for e in entries {
            let versionNeeded: UInt16 = e.method == 8 ? 20 : 10
            buf += le32(0x02014B50)                    // CD signature
            buf += le16(0x031E)                        // version made by (Unix, v30)
            buf += le16(versionNeeded)
            buf += le16(0x0800)                        // flags (UTF-8)
            buf += le16(e.method)
            buf += le16(UInt16(dosEpoch & 0xFFFF))     // mod_time
            buf += le16(UInt16(dosEpoch >> 16))        // mod_date
            buf += le32(e.crc)
            buf += le32(e.compressedSize)
            buf += le32(e.uncompressedSize)
            buf += le16(UInt16(e.name.count))
            buf += le16(0)                             // extra_len
            buf += le16(0)                             // comment_len
            buf += le16(0)                             // disk_start
            buf += le16(0)                             // internal_attrs
            buf += le32(e.externalAttrs)
            buf += le32(e.localOffset)
            buf += e.name
        }
        let cdSize = UInt32(buf.count - cdStart)

        // ── End of Central Directory Record ──────────────────────────────
        buf += le32(0x06054B50)  // EOCD signature
        buf += le16(0)           // disk_number
        buf += le16(0)           // cd_disk
        buf += le16(numEntries)  // entries this disk
        buf += le16(numEntries)  // entries total
        buf += le32(cdSize)
        buf += le32(cdOffset)
        buf += le16(0)           // comment_len

        return buf
    }

    private mutating func addEntry(_ name: String, data: [UInt8],
                                    compress: Bool, unixMode: UInt32) {
        let nameBytes = Array(name.utf8)
        let checksum = crc32(data)
        let uncompressedSize = UInt32(data.count)

        let (method, fileData): (UInt16, [UInt8]) = {
            if compress && !data.isEmpty {
                let compressed = deflateCompress(data)
                if compressed.count < data.count {
                    return (8, compressed)
                }
            }
            return (0, data)
        }()

        let compressedSize = UInt32(fileData.count)
        let localOffset = UInt32(buf.count)
        let versionNeeded: UInt16 = method == 8 ? 20 : 10

        // ── Local File Header ─────────────────────────────────────────────
        buf += le32(0x04034B50)                  // signature
        buf += le16(versionNeeded)
        buf += le16(0x0800)                      // flags (UTF-8)
        buf += le16(method)
        buf += le16(UInt16(dosEpoch & 0xFFFF))   // mod_time
        buf += le16(UInt16(dosEpoch >> 16))      // mod_date
        buf += le32(checksum)
        buf += le32(compressedSize)
        buf += le32(uncompressedSize)
        buf += le16(UInt16(nameBytes.count))
        buf += le16(0)                           // extra_field_length = 0
        buf += nameBytes
        buf += fileData

        entries.append(CdRecord(
            name: nameBytes,
            method: method,
            crc: checksum,
            compressedSize: compressedSize,
            uncompressedSize: uncompressedSize,
            localOffset: localOffset,
            externalAttrs: unixMode << 16
        ))
    }
}

// ============================================================================
// ZIP Read — ZipEntry and ZipReader
// ============================================================================
//
// ZipReader uses the "EOCD-first" strategy for reliable random-access:
//
//   1. Scan backwards for the EOCD signature (PK\x05\x06).
//      Limit the scan to the last 65535 + 22 bytes.
//   2. Read the CD offset and size from EOCD.
//   3. Parse all Central Directory headers into ZipEntry objects.
//   4. On `read(entry)`: seek to the Local Header via `local_offset`, skip
//      the variable-length name + extra fields, read compressed data,
//      decompress, verify CRC-32.

/// Metadata for a single entry inside a ZIP archive.
public struct ZipEntry {
    /// File name (UTF-8).
    public let name: String
    /// Uncompressed size in bytes.
    public let size: UInt32
    /// Compressed size in bytes.
    public let compressedSize: UInt32
    /// Compression method: 0 = Stored, 8 = DEFLATE.
    public let method: UInt16
    /// CRC-32 of the uncompressed content.
    public let crc32: UInt32
    /// True if this entry is a directory (name ends with '/').
    public let isDirectory: Bool
    let localOffset: UInt32
}

/// Reads entries from an in-memory ZIP archive.
///
/// ```swift
/// let reader = try ZipReader([UInt8](archiveData))
/// for entry in reader.entries() {
///     print("\(entry.name): \(entry.size) bytes")
/// }
/// ```
public struct ZipReader {
    private let data: [UInt8]
    private let _entries: [ZipEntry]

    /// Parse an in-memory ZIP archive. Throws if no valid EOCD is found.
    public init(_ data: [UInt8]) throws {
        self.data = data
        guard let eocdOffset = ZipReader.findEOCD(data) else {
            throw ZipError.malformed("zip: no End of Central Directory record found")
        }

        guard let cdOffset32 = readLE32(data, at: eocdOffset + 16),
              let cdSize32   = readLE32(data, at: eocdOffset + 12) else {
            throw ZipError.malformed("zip: EOCD too short")
        }
        let cdOffset = Int(cdOffset32)
        let cdSize   = Int(cdSize32)

        if cdOffset + cdSize > data.count {
            throw ZipError.malformed("zip: Central Directory out of bounds")
        }

        var entries = [ZipEntry]()
        var pos = cdOffset

        while pos + 4 <= cdOffset + cdSize {
            guard let sig = readLE32(data, at: pos), sig == 0x02014B50 else { break }

            guard pos + 46 <= data.count else {
                throw ZipError.malformed("zip: CD entry header out of bounds")
            }

            guard let method          = readLE16(data, at: pos + 10),
                  let crc             = readLE32(data, at: pos + 16),
                  let compressedSize  = readLE32(data, at: pos + 20),
                  let size            = readLE32(data, at: pos + 24),
                  let nameLen16       = readLE16(data, at: pos + 28),
                  let extraLen16      = readLE16(data, at: pos + 30),
                  let commentLen16    = readLE16(data, at: pos + 32),
                  let localOffset     = readLE32(data, at: pos + 42) else {
                throw ZipError.malformed("zip: CD entry fields truncated")
            }

            let nameLen    = Int(nameLen16)
            let extraLen   = Int(extraLen16)
            let commentLen = Int(commentLen16)

            let nameStart = pos + 46
            let nameEnd   = nameStart + nameLen
            guard nameEnd <= data.count else {
                throw ZipError.malformed("zip: CD entry name out of bounds")
            }

            let nameBytes = Array(data[nameStart..<nameEnd])
            guard let name = String(bytes: nameBytes, encoding: .utf8) else {
                throw ZipError.malformed("zip: entry name is not valid UTF-8 at CD offset \(pos)")
            }

            // Validate entry name: reject null bytes, backslashes, absolute paths,
            // and path traversal components ("..").
            if name.contains("\0") {
                throw ZipError.malformed("zip: entry name contains null byte")
            }
            if name.contains("\\") {
                throw ZipError.malformed("zip: entry name contains backslash")
            }
            if name.hasPrefix("/") {
                throw ZipError.malformed("zip: entry name is an absolute path: \(name)")
            }
            let segments = name.components(separatedBy: "/")
            if segments.contains("..") {
                throw ZipError.malformed("zip: entry name contains path traversal (..): \(name)")
            }

            let nextPos = nameEnd + extraLen + commentLen
            guard nextPos <= cdOffset + cdSize else {
                throw ZipError.malformed("zip: CD entry advance out of bounds")
            }

            entries.append(ZipEntry(
                name: name,
                size: size,
                compressedSize: compressedSize,
                method: method,
                crc32: crc,
                isDirectory: name.hasSuffix("/"),
                localOffset: localOffset
            ))
            pos = nextPos
        }

        self._entries = entries
    }

    /// Return all entries in the archive.
    public func entries() -> [ZipEntry] { _entries }

    /// Decompress and return the data for `entry`. Verifies CRC-32.
    public func read(_ entry: ZipEntry) throws -> [UInt8] {
        if entry.isDirectory { return [] }

        let lhOff = Int(entry.localOffset)
        guard let localFlags = readLE16(data, at: lhOff + 6) else {
            throw ZipError.malformed("zip: local header out of bounds")
        }
        if localFlags & 1 != 0 {
            throw ZipError.unsupported("zip: entry '\(entry.name)' is encrypted")
        }

        guard let lhNameLen  = readLE16(data, at: lhOff + 26),
              let lhExtraLen = readLE16(data, at: lhOff + 28) else {
            throw ZipError.malformed("zip: local header fields out of bounds for '\(entry.name)'")
        }

        let dataStart = lhOff + 30 + Int(lhNameLen) + Int(lhExtraLen)
        let dataEnd   = dataStart + Int(entry.compressedSize)
        guard dataEnd <= data.count else {
            throw ZipError.malformed("zip: entry '\(entry.name)' data out of bounds")
        }

        let compressed = Array(data[dataStart..<dataEnd])

        let decompressed: [UInt8]
        switch entry.method {
        case 0:
            decompressed = compressed
        case 8:
            let result = try rawInflateCounted(compressed, maxOutput: Int(entry.size))
            guard result.bytesConsumed == compressed.count else {
                throw ZipError.malformed("zip: compressed payload contains trailing bytes")
            }
            decompressed = result.output
        default:
            throw ZipError.unsupported("zip: unsupported compression method \(entry.method) for '\(entry.name)'")
        }

        guard decompressed.count == Int(entry.size) else {
            throw ZipError.malformed("zip: uncompressed size does not match the directory")
        }

        let actualCRC = crc32(decompressed)
        if actualCRC != entry.crc32 {
            throw ZipError.crcMismatch(
                "zip: CRC-32 mismatch for '\(entry.name)': expected \(String(entry.crc32, radix: 16)), got \(String(actualCRC, radix: 16))"
            )
        }

        return decompressed
    }

    /// Find an entry by name and return its decompressed data.
    public func readByName(_ name: String) throws -> [UInt8] {
        guard let entry = _entries.first(where: { $0.name == name }) else {
            throw ZipError.notFound("zip: entry '\(name)' not found")
        }
        return try read(entry)
    }

    /// Scan backwards from the end of `data` for the EOCD signature 0x06054B50.
    private static func findEOCD(_ data: [UInt8]) -> Int? {
        let minSize = 22
        let maxComment = 65535
        guard data.count >= minSize else { return nil }
        let scanStart = max(0, data.count - minSize - maxComment)
        var i = data.count - minSize
        while i >= scanStart {
            if let sig = readLE32(data, at: i), sig == 0x06054B50 {
                if let commentLen = readLE16(data, at: i + 20) {
                    if i + minSize + Int(commentLen) == data.count {
                        return i
                    }
                }
            }
            i -= 1
        }
        return nil
    }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Compress a list of `(name, data)` pairs into a ZIP archive.
///
/// Each file is compressed with DEFLATE if it reduces size; otherwise stored.
///
/// ```swift
/// let archive = zip([("hello.txt", Array("Hello, ZIP!".utf8))])
/// ```
public func zip(_ entries: [(name: String, data: [UInt8])], compress: Bool = true) -> [UInt8] {
    var w = ZipWriter()
    for (name, data) in entries {
        w.addFile(name, data: data, compress: compress)
    }
    return w.finish()
}

/// Decompress all file entries from a ZIP archive.
///
/// Returns a dictionary mapping entry name to decompressed bytes.
/// Directories (names ending with '/') are skipped.
///
/// Throws on the first corrupted or unsupported entry.
public func unzip(_ data: [UInt8]) throws -> [String: [UInt8]] {
    let reader = try ZipReader(data)
    var result = [String: [UInt8]]()
    for entry in reader.entries() where !entry.isDirectory {
        // Reject duplicate names: a second entry with the same name could shadow
        // a scanned/verified first entry — classic "entry smuggling" attack.
        if result[entry.name] != nil {
            throw ZipError.malformed("zip: duplicate entry name '\(entry.name)'")
        }
        result[entry.name] = try reader.read(entry)
    }
    return result
}
