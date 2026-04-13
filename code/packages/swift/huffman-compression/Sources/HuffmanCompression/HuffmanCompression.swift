// HuffmanCompression.swift — CMP04: Huffman Compression
// ============================================================================
//
// Huffman compression (1952) is an entropy coding technique invented by
// David A. Huffman as a student at MIT. It assigns variable-length bit codes
// to symbols: frequent symbols get short codes, rare symbols get long codes.
// The result is a "prefix-free" code — no code is a prefix of any other — so
// the bit stream can be decoded unambiguously without separator characters.
//
// Think of it like Morse code. "E" is "." (one dot) because it is the most
// common letter in English. "Z" is "--.." (four symbols). Huffman's algorithm
// does this automatically and provably optimally for any frequency distribution.
//
// ============================================================
// Canonical Codes and the Wire Format
// ============================================================
//
// A naive approach would transmit the full tree structure so the decoder can
// reconstruct it. But there is a smarter way: "canonical codes."
//
// Given only the set of (symbol, code_length) pairs, you can deterministically
// reconstruct the exact same code table, because canonical codes are assigned
// by a fixed rule:
//
//   1. Sort symbols by (code_length, symbol_value) ascending.
//   2. Start with code = 0.
//   3. For each symbol: assign it `code` left-padded to `code_length` bits.
//      Then increment code. If the next symbol has a longer length, shift
//      code left by (next_length - current_length).
//
// This means the wire format only needs to transmit:
//   - The original data length (to know when to stop decoding).
//   - The (symbol, code_length) pairs — NOT the tree, NOT the codes.
//   - The packed bit stream of encoded symbols.
//
// This is essentially what DEFLATE (ZIP, gzip, PNG, zlib) does.
//
// ============================================================
// CMP04 Wire Format
// ============================================================
//
//   Bytes 0–3:    original_length  (big-endian uint32)
//   Bytes 4–7:    symbol_count     (big-endian uint32)
//   Bytes 8–8+2N: code-lengths table: N pairs of 2 bytes each
//                   [0]: symbol value (uint8, 0–255)
//                   [1]: code length  (uint8, 1–16)
//                 Sorted by (code_length, symbol_value) ascending.
//   Bytes 8+2N+:  bit stream, packed LSB-first, zero-padded to byte boundary.
//
// Example — "AAABBC" (6 bytes, 3 distinct symbols):
//
//   Frequencies: A=3, B=2, C=1
//   Huffman tree (greedy, canonical):
//     A → "0"   (length 1)
//     B → "10"  (length 2)
//     C → "11"  (length 2)
//
//   Wire format:
//     [0,0,0,6]       original_length = 6
//     [0,0,0,3]       symbol_count    = 3
//     [65,1]          A has code length 1
//     [66,2]          B has code length 2
//     [67,2]          C has code length 2
//     bit stream for "AAABBC":
//       A→"0", A→"0", A→"0", B→"10", B→"10", C→"11"
//       concatenated: "000101011" (9 bits)
//       packed LSB-first:
//         byte 0: bits 0-7 = "00010101" → 0b10101000 wait, LSB-first means
//                 position 0 = bit 0, so "0" goes to bit 0
//                 "000101011" → pad to 16 bits → "0001010110000000"
//                 byte 0: bit0=0, bit1=0, bit2=0, bit3=1, bit4=0, bit5=1, bit6=0, bit7=1 → 0xA8
//                 byte 1: bit0=1, rest=0 → 0x01
//       → [0xA8, 0x01]
//
// ============================================================
// LSB-First Bit Packing
// ============================================================
//
// "LSB-first" means we pack bits starting from the least-significant bit of
// each output byte. So the first bit of the bit stream goes into bit 0 of
// byte 0, the second into bit 1, and so on. When byte 0 fills up (8 bits),
// we start on byte 1.
//
// For "000101011" (the 9-bit stream above):
//   Byte 0: bit positions 0–7
//     pos 0: '0' → byte = 0b00000000
//     pos 1: '0' → byte = 0b00000000
//     pos 2: '0' → byte = 0b00000000
//     pos 3: '1' → byte = 0b00001000
//     pos 4: '0' → byte = 0b00001000
//     pos 5: '1' → byte = 0b00101000
//     pos 6: '0' → byte = 0b00101000
//     pos 7: '1' → byte = 0b10101000 = 0xA8
//   Byte 1: bit position 0
//     pos 0: '1' → byte = 0b00000001 = 0x01
//
// ============================================================
// The Series: CMP00 → CMP05
// ============================================================
//
//   CMP00 (LZ77,    1977) — Sliding-window backreferences.
//   CMP01 (LZ78,    1978) — Explicit dictionary (trie), no sliding window.
//   CMP02 (LZSS,    1982) — LZ77 + flag bits; eliminates wasted literals.
//   CMP03 (LZW,     1984) — Pre-initialized dictionary; powers GIF.
//   CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE.  ← YOU ARE HERE
//   CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib standard.
// ============================================================================

import HuffmanTree

// MARK: - Errors

/// Errors that can be thrown by HuffmanCompression operations.
///
/// Each case describes a specific failure mode with enough context to
/// diagnose what went wrong.
public enum HuffmanCompressionError: Error, Equatable {
    /// The compressed data is shorter than the minimum valid header (8 bytes).
    case dataTooShort(length: Int)

    /// The data claims symbol_count entries but doesn't have enough bytes.
    case truncatedCodeTable(expected: Int, available: Int)

    /// A code-length entry has a zero length, which is invalid.
    case invalidCodeLength(symbol: Int, length: Int)

    /// The bit stream ended before all symbols were decoded.
    case bitStreamExhausted(decoded: Int, expected: Int)

    /// The HuffmanTree returned an error (forwarded).
    case huffmanTreeError(String)
}

// MARK: - String extension: left-padding

extension String {
    /// Returns `self` left-padded to `length` characters using `char`.
    ///
    /// If `self` is already at least `length` characters long, returns `self`
    /// unchanged.
    ///
    /// Example:
    /// ```
    /// "101".leftPadded(toLength: 5, with: "0")  → "00101"
    /// "hello".leftPadded(toLength: 3, with: "0") → "hello"
    /// ```
    func leftPadded(toLength length: Int, with char: Character) -> String {
        guard self.count < length else { return self }
        return String(repeating: char, count: length - self.count) + self
    }
}

// MARK: - Big-Endian UInt32 helpers

/// Encodes a UInt32 as 4 bytes in big-endian (network byte) order.
///
/// Big-endian means the most-significant byte comes first:
///
///   0x01020304 → [0x01, 0x02, 0x03, 0x04]
///
/// This is the standard network byte order and matches the CMP04 wire format.
private func writeUInt32BE(_ value: UInt32) -> [UInt8] {
    return [
        UInt8((value >> 24) & 0xFF),
        UInt8((value >> 16) & 0xFF),
        UInt8((value >>  8) & 0xFF),
        UInt8( value        & 0xFF),
    ]
}

/// Reads 4 bytes from `data` at `offset` and interprets them as a big-endian UInt32.
///
/// Inverse of `writeUInt32BE`. The caller must ensure `data` has at least
/// `offset + 4` elements.
///
/// Example:
///   data = [0x00, 0x00, 0x00, 0x06, ...]
///   readUInt32BE(data, offset: 0) → 6
private func readUInt32BE(_ data: [UInt8], offset: Int) -> UInt32 {
    return UInt32(data[offset])     << 24
         | UInt32(data[offset + 1]) << 16
         | UInt32(data[offset + 2]) <<  8
         | UInt32(data[offset + 3])
}

// MARK: - Bit Packing helpers

/// Packs a string of '0' and '1' characters into bytes, LSB-first.
///
/// The first character of `bits` becomes bit 0 (the least-significant bit)
/// of the first output byte. Bits 0–7 fill byte 0, bits 8–15 fill byte 1,
/// and so on. If the total bit count is not a multiple of 8, the last byte
/// is zero-padded on the most-significant side.
///
/// Example:
///   "000101011" (9 bits)
///     byte 0: 0·0·0·1·0·1·0·1 (bits 0–7) → 0b10101000 = 0xA8
///     byte 1: 1·0·0·0·0·0·0·0 (bits 8–15) → 0b00000001 = 0x01
///   → [0xA8, 0x01]
private func packBitsLsbFirst(_ bits: String) -> [UInt8] {
    var output = [UInt8]()
    var buffer: UInt8 = 0
    var bitPos = 0

    for ch in bits {
        // If this bit is '1', set the corresponding bit position in buffer.
        // bitPos 0 = least-significant bit, bitPos 7 = most-significant bit.
        if ch == "1" {
            buffer |= 1 << bitPos
        }
        bitPos += 1

        // Once we've filled 8 bits, emit the byte and reset.
        if bitPos == 8 {
            output.append(buffer)
            buffer = 0
            bitPos = 0
        }
    }

    // Flush any partial byte (it's already zero-padded by the initialisation).
    if bitPos > 0 {
        output.append(buffer)
    }

    return output
}

/// Unpacks bytes into a string of '0' and '1' characters, LSB-first.
///
/// Inverse of `packBitsLsbFirst`. For each byte, bit 0 (LSB) is extracted first
/// and appended as the next character in the output string.
///
/// Example:
///   [0xA8, 0x01]
///     0xA8 = 0b10101000:
///       bit 0 = 0, bit 1 = 0, bit 2 = 0, bit 3 = 1, bit 4 = 0, bit 5 = 1, bit 6 = 0, bit 7 = 1
///       → "00010101"
///     0x01 = 0b00000001:
///       bit 0 = 1, bit 1–7 = 0
///       → "10000000"
///   → "0001010110000000"
private func unpackBitsLsbFirst(_ data: [UInt8]) -> String {
    var bits = ""
    for byte in data {
        for i in 0..<8 {
            bits.append((byte >> i) & 1 == 1 ? "1" : "0")
        }
    }
    return bits
}

// MARK: - Canonical Code Reconstruction

/// Reconstructs a canonical code table (bit-string → symbol) from sorted
/// (symbol, length) pairs.
///
/// This is the inverse of the canonical code assignment algorithm. Given the
/// same sorted (symbol, length) list that was used when encoding, this function
/// regenerates the exact same bit-string-to-symbol mapping.
///
/// The algorithm:
///   1. Start with `code = 0` and `prevLen = lengths[0].length`.
///   2. For each (sym, len) in order:
///      a. If `len > prevLen`, shift `code` left by `(len - prevLen)` — this
///         "skips" the gap in code space caused by the length increase.
///      b. Format `code` as a zero-padded binary string of length `len`.
///      c. Map that string → sym in the output dictionary.
///      d. Increment `code`.
///
/// The `lengths` array must already be sorted by `(length, symbol)` ascending,
/// which is how the encoder stored the table in the wire format.
///
/// - Parameter lengths: Sorted array of `(symbol: Int, length: Int)` pairs.
/// - Returns: Dictionary mapping each canonical code string to its symbol.
private func canonicalCodesFromLengths(_ lengths: [(symbol: Int, length: Int)]) -> [String: Int] {
    guard !lengths.isEmpty else { return [:] }

    var codeToSym = [String: Int]()
    var code = 0
    var prevLen = lengths[0].length

    for (sym, len) in lengths {
        // When the code length increases, we must shift left to preserve the
        // prefix-free property. Each additional bit doubles the code space,
        // and the canonical algorithm fills codes sequentially within each
        // length, leaving room for longer codes to follow.
        if len > prevLen {
            code <<= (len - prevLen)
        }

        // Format as zero-padded binary string.
        let bits = String(code, radix: 2).leftPadded(toLength: len, with: "0")
        codeToSym[bits] = sym

        code += 1
        prevLen = len
    }

    return codeToSym
}

// MARK: - Public API

/// Compresses `data` using Huffman coding and returns the CMP04 wire-format bytes.
///
/// The algorithm:
///   1. Count the frequency of each byte value in the input.
///   2. Build a Huffman tree from the frequency table.
///   3. Derive canonical codes from the tree (`canonicalCodeTable()`).
///   4. Encode the input by concatenating each symbol's canonical code string.
///   5. Pack the concatenated bits LSB-first into bytes.
///   6. Assemble the wire format: header + code-lengths table + packed bits.
///
/// Empty input returns a valid 8-byte header (original_length=0, symbol_count=0)
/// with no code-lengths table and no bit stream.
///
/// - Parameter data: The bytes to compress.
/// - Returns: Compressed bytes in CMP04 wire format.
/// - Throws: `HuffmanCompressionError.huffmanTreeError` if the tree can't be built.
public func compress(_ data: [UInt8]) throws -> [UInt8] {
    // ── Step 1: Empty input ───────────────────────────────────────────────────
    // Empty input is a valid (if trivial) case. We store original_length=0
    // and symbol_count=0 with no table entries or bit stream.
    if data.isEmpty {
        return writeUInt32BE(0) + writeUInt32BE(0)
    }

    // ── Step 2: Count frequencies ─────────────────────────────────────────────
    // We need to know how often each byte value (0–255) appears so we can
    // assign shorter codes to more-frequent symbols.
    //
    // Example for "AAABBC":
    //   freq = {65: 3, 66: 2, 67: 1}
    var freq = [Int: Int]()
    for b in data {
        freq[Int(b), default: 0] += 1
    }

    // ── Step 3: Build the Huffman tree ────────────────────────────────────────
    // Pass the frequency table to the DT27 HuffmanTree implementation.
    // The tree is a full binary tree where leaves hold symbols and internal
    // nodes represent merges of sub-trees. Greedy construction via min-heap
    // guarantees the optimal (minimum total bits) assignment.
    let weights = freq.map { (symbol: $0.key, frequency: $0.value) }
    let tree: HuffmanTree
    do {
        tree = try HuffmanTree.build(weights)
    } catch {
        throw HuffmanCompressionError.huffmanTreeError("\(error)")
    }

    // ── Step 4: Get canonical codes ───────────────────────────────────────────
    // `canonicalCodeTable()` returns [symbol: bitString] using the canonical
    // assignment rule. Two trees with the same symbol lengths produce the same
    // canonical codes, which makes the table (not the tree) the unit of storage.
    //
    // Example (AAABBC):
    //   table = {65: "0", 66: "10", 67: "11"}
    let table = tree.canonicalCodeTable()

    // ── Step 5: Build the sorted code-lengths list ────────────────────────────
    // The wire format stores (symbol, length) pairs sorted by (length, symbol).
    // This sorted order is also what the decoder needs to reconstruct the codes.
    //
    // Example: [(65,1), (66,2), (67,2)]
    let sortedLengths: [(symbol: Int, length: Int)] = table
        .map { (symbol: $0.key, length: $0.value.count) }
        .sorted {
            if $0.length != $1.length { return $0.length < $1.length }
            return $0.symbol < $1.symbol
        }

    // ── Step 6: Encode the input as a bit string ──────────────────────────────
    // For each byte in `data`, look up its canonical code and concatenate.
    // The result is a (potentially long) string of '0' and '1' characters.
    //
    // Example: "AAABBC" → "0" + "0" + "0" + "10" + "10" + "11" = "000101011"
    var bitString = ""
    bitString.reserveCapacity(data.count * 4)
    for b in data {
        guard let code = table[Int(b)] else {
            // This should never happen — every byte in data has a frequency > 0
            // and therefore a code in the table.
            continue
        }
        bitString += code
    }

    // ── Step 7: Pack bits LSB-first ───────────────────────────────────────────
    let packedBits = packBitsLsbFirst(bitString)

    // ── Step 8: Assemble wire format ──────────────────────────────────────────
    // Header: original_length (4 bytes) + symbol_count (4 bytes)
    var result = writeUInt32BE(UInt32(data.count))
    result += writeUInt32BE(UInt32(sortedLengths.count))

    // Code-lengths table: N × 2 bytes = [symbol, length] per entry
    for (sym, len) in sortedLengths {
        result.append(UInt8(sym))
        result.append(UInt8(len))
    }

    // Bit stream
    result += packedBits

    return result
}

/// Decompresses CMP04 wire-format bytes and returns the original data.
///
/// The algorithm:
///   1. Parse the header (8 bytes): original_length and symbol_count.
///   2. Parse the code-lengths table (symbol_count × 2 bytes).
///   3. Reconstruct canonical codes from the table.
///   4. Unpack the bit stream from LSB-first packed bytes.
///   5. Decode exactly `original_length` symbols using the code table.
///
/// Special cases:
///   - If original_length == 0 (empty input), returns [].
///   - If symbol_count == 0 with original_length > 0, returns [] (malformed).
///
/// - Parameter data: Compressed bytes in CMP04 wire format.
/// - Returns: The original uncompressed bytes.
/// - Throws: `HuffmanCompressionError` variants for malformed input.
public func decompress(_ data: [UInt8]) throws -> [UInt8] {
    // ── Step 1: Check minimum header size ─────────────────────────────────────
    guard data.count >= 8 else {
        throw HuffmanCompressionError.dataTooShort(length: data.count)
    }

    // ── Step 2: Parse header ──────────────────────────────────────────────────
    let originalLength = Int(readUInt32BE(data, offset: 0))
    let symbolCount    = Int(readUInt32BE(data, offset: 4))

    // Trivial cases.
    if originalLength == 0 { return [] }
    if symbolCount == 0    { return [] }

    // ── Step 3: Parse code-lengths table ──────────────────────────────────────
    // Each entry is 2 bytes: [symbol, length]. There are `symbolCount` entries.
    // The table starts at byte 8 and occupies 2 × symbolCount bytes.
    let tableStart = 8
    let tableBytes = symbolCount * 2
    guard data.count >= tableStart + tableBytes else {
        throw HuffmanCompressionError.truncatedCodeTable(
            expected:  symbolCount,
            available: (data.count - tableStart) / 2
        )
    }

    // Read and validate each (symbol, length) pair.
    var lengths = [(symbol: Int, length: Int)]()
    lengths.reserveCapacity(symbolCount)
    for i in 0..<symbolCount {
        let sym = Int(data[tableStart + i * 2])
        let len = Int(data[tableStart + i * 2 + 1])
        guard len >= 1 else {
            throw HuffmanCompressionError.invalidCodeLength(symbol: sym, length: len)
        }
        lengths.append((symbol: sym, length: len))
    }

    // ── Step 4: Reconstruct canonical codes ───────────────────────────────────
    // The table was stored sorted by (length, symbol), which is the exact
    // input format that `canonicalCodesFromLengths` requires.
    let codeToSym = canonicalCodesFromLengths(lengths)

    // ── Step 5: Unpack the bit stream ─────────────────────────────────────────
    // The bit stream starts immediately after the code-lengths table.
    let bitStreamStart = tableStart + tableBytes
    let bitBytes = Array(data[bitStreamStart...])
    let bitString = unpackBitsLsbFirst(bitBytes)

    // ── Step 6: Special case — single symbol ──────────────────────────────────
    // When there is only one distinct symbol, the canonical code is "0" (the
    // HuffmanTree's single-leaf convention). Each occurrence is one "0" bit.
    if symbolCount == 1 {
        let onlySym = lengths[0].symbol
        return [UInt8](repeating: UInt8(onlySym), count: originalLength)
    }

    // ── Step 7: Decode bit stream ─────────────────────────────────────────────
    // We walk the bit string character by character, accumulating a "current
    // code" string. At each step we check whether `currentCode` matches any
    // entry in `codeToSym`. Because canonical codes are prefix-free, there is
    // exactly one match at the right length.
    //
    // This is a greedy prefix search: try the shortest code first (length 1),
    // extending one bit at a time until a match is found.
    //
    // Maximum code length is 16 bits (the Huffman tree depth bound), so the
    // inner search is bounded and never runs away.
    var output   = [UInt8]()
    output.reserveCapacity(originalLength)
    var currentCode = ""
    var idx = bitString.startIndex

    while output.count < originalLength {
        guard idx < bitString.endIndex else {
            throw HuffmanCompressionError.bitStreamExhausted(
                decoded:  output.count,
                expected: originalLength
            )
        }

        currentCode.append(bitString[idx])
        idx = bitString.index(after: idx)

        if let sym = codeToSym[currentCode] {
            output.append(UInt8(sym))
            currentCode = ""
        }
        // If no match yet, currentCode grows by one more bit on the next iteration.
    }

    return output
}
