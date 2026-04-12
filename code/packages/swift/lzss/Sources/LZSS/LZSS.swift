// LZSS.swift
// Part of coding-adventures — an educational computing stack.
//
// ============================================================================
// LZSS Lossless Compression Algorithm (1982)
// ============================================================================
//
// LZSS (Storer & Szymanski, 1982) refines LZ77 by eliminating the mandatory
// `nextChar` byte appended after every token. Instead, a flag-bit scheme
// distinguishes the two token kinds:
//
//   Literal(byte)         — 1 byte  (flag bit = 0)
//   Match(offset, length) — 3 bytes (flag bit = 1)
//
// Tokens are grouped in blocks of 8. Each block starts with a 1-byte flag
// (LSB = first token, bit 7 = eighth token).
//
// Break-Even Point
// ----------------
//
// A match token costs 3 bytes; three literals also cost 3 bytes. So
// minMatch = 3 is the minimum that yields any saving; length ≥ 4 yields net
// gain. Compared to LZ77 (4 bytes per match), LZSS typically achieves
// 25–50% better compression on repetitive data.
//
// Wire Format (CMP02)
// -------------------
//
//     Bytes 0–3:  originalLength  (big-endian UInt32)
//     Bytes 4–7:  blockCount      (big-endian UInt32)
//     Bytes 8+:   blocks
//       Each block:
//         [1 byte]  flag — bit i (LSB-first): 0 = literal, 1 = match
//         [variable] up to 8 items:
//                      flag=0: 1 byte  (literal value)
//                      flag=1: 3 bytes (offset BE UInt16 + length UInt8)
//
// The `originalLength` field lets the decoder trim any block-alignment
// padding from the final block.
//
// The Series: CMP00 → CMP05
// -------------------------
//
//   CMP00 (LZ77, 1977)     — Sliding-window backreferences.
//   CMP01 (LZ78, 1978)     — Explicit dictionary (trie), no sliding window.
//   CMP02 (LZSS, 1982)     — LZ77 + flag bits; eliminates wasted literals. This module.
//   CMP03 (LZW,  1984)     — Pre-initialized dictionary; powers GIF.
//   CMP04 (Huffman, 1952)  — Entropy coding; prerequisite for DEFLATE.
//   CMP05 (DEFLATE, 1996)  — LZ77 + Huffman; ZIP/gzip/PNG/zlib standard.

import Foundation

// MARK: - Token

/// An LZSS token: either a raw byte literal or a back-reference into the window.
///
/// Unlike LZ77 which always emits `(offset, length, nextChar)`, LZSS emits
/// either:
///
/// - `.literal(byte)` — a single byte, 1 byte in the wire format.
/// - `.match(offset, length)` — a back-reference, 3 bytes in the wire format.
public enum Token: Equatable {
    case literal(UInt8)
    case match(offset: UInt16, length: UInt8)
}

// MARK: - Encoder

/// Finds the longest match in the search buffer for data starting at `cursor`.
///
/// Unlike LZ77, LZSS does NOT reserve 1 byte for `nextChar`. The lookahead
/// extends all the way to `data.count`, allowing matches that cover the last
/// byte of the input.
///
/// - Returns: `(bestOffset, bestLength)`, both 0 if no match found.
private func findLongestMatch(
    data: [UInt8],
    cursor: Int,
    windowSize: Int,
    maxMatch: Int
) -> (Int, Int) {
    var bestOffset = 0
    var bestLength = 0

    let searchStart = max(0, cursor - windowSize)
    // LZSS: no nextChar reservation — lookahead goes all the way to data.count.
    let lookaheadEnd = min(cursor + maxMatch, data.count)

    for pos in searchStart..<cursor {
        var length = 0
        while cursor + length < lookaheadEnd
            && data[pos + length] == data[cursor + length]
        {
            length += 1
        }
        if length > bestLength {
            bestLength = length
            bestOffset = cursor - pos
        }
    }

    return (bestOffset, bestLength)
}

/// Encodes data into an LZSS token stream.
///
/// Scans the input left-to-right. For each position, finds the longest match
/// in the search buffer. If the match is long enough (≥ minMatch), emits a
/// `.match` token; otherwise emits a `.literal` token.
///
/// Unlike LZ77, the cursor advances by exactly `bestLength` positions on a
/// match (not `bestLength + 1`), because there is no trailing `nextChar`.
///
/// - Parameters:
///   - data:       Input bytes.
///   - windowSize: Maximum lookback distance (default 4096).
///   - maxMatch:   Maximum match length (default 255).
///   - minMatch:   Minimum length for a Match token (default 3).
/// - Returns: Array of tokens.
///
/// ```swift
/// let tokens = encode(Array("ABABAB".utf8))
/// // [.literal(65), .literal(66), .match(offset: 2, length: 4)]
/// ```
public func encode(
    _ data: [UInt8],
    windowSize: Int = 4096,
    maxMatch: Int = 255,
    minMatch: Int = 3
) -> [Token] {
    var tokens: [Token] = []
    var cursor = 0

    while cursor < data.count {
        let (offset, length) = findLongestMatch(data: data, cursor: cursor, windowSize: windowSize, maxMatch: maxMatch)

        if length >= minMatch {
            tokens.append(.match(offset: UInt16(offset), length: UInt8(length)))
            cursor += length
        } else {
            tokens.append(.literal(data[cursor]))
            cursor += 1
        }
    }

    return tokens
}

// MARK: - Decoder

/// Decodes an LZSS token stream back into the original bytes.
///
/// Processes each token:
/// - `.literal(b)` — append `b` to output.
/// - `.match(offset, length)` — copy `length` bytes from `offset` positions
///   back in the output, byte-by-byte to handle overlapping matches.
///
/// Overlapping match example: output = [65], Match(offset=1, length=6)
///   → copies `output[0]` six times → [65, 65, 65, 65, 65, 65, 65] = "AAAAAAA".
///
/// - Parameters:
///   - tokens:         The token stream (output of `encode`).
///   - originalLength: If provided, truncates the output to this length.
/// - Returns: Reconstructed bytes.
public func decode(_ tokens: [Token], originalLength: Int? = nil) -> [UInt8] {
    var output: [UInt8] = []

    for token in tokens {
        switch token {
        case .literal(let b):
            output.append(b)
        case .match(let offset, let length):
            let start = output.count - Int(offset)
            for i in 0..<Int(length) {
                output.append(output[start + i])
            }
        }

        if let limit = originalLength, output.count >= limit {
            break
        }
    }

    if let limit = originalLength, output.count > limit {
        output = Array(output.prefix(limit))
    }

    return output
}

// MARK: - Serialisation

/// Serialises a token list to the CMP02 wire format.
///
/// Groups up to 8 tokens per block. Each block starts with a 1-byte flag
/// (bit i = 0 for Literal, 1 for Match). Literals use 1 byte; Matches use
/// 3 bytes (offset BE UInt16 + length UInt8).
///
/// The header stores the original data length so the decoder can trim
/// block-alignment padding.
///
/// - Parameters:
///   - tokens:         Token list from `encode`.
///   - originalLength: Byte count of the original input.
/// - Returns: CMP02 binary bytes.
public func serialiseTokens(_ tokens: [Token], originalLength: Int) -> [UInt8] {
    var blocks: [[UInt8]] = []
    var i = 0

    while i < tokens.count {
        let chunkEnd = min(i + 8, tokens.count)
        var flag: UInt8 = 0
        var symbols: [UInt8] = []

        for bit in 0..<(chunkEnd - i) {
            let tok = tokens[i + bit]
            switch tok {
            case .literal(let b):
                symbols.append(b)
            case .match(let offset, let length):
                flag |= UInt8(1 << bit)
                symbols.append(UInt8((offset >> 8) & 0xFF))
                symbols.append(UInt8(offset & 0xFF))
                symbols.append(length)
            }
        }

        var block: [UInt8] = [flag]
        block.append(contentsOf: symbols)
        blocks.append(block)
        i = chunkEnd
    }

    // Header: originalLength (BE UInt32) + blockCount (BE UInt32)
    let origLen = UInt32(originalLength)
    let blockCount = UInt32(blocks.count)
    var buf: [UInt8] = [
        UInt8((origLen >> 24) & 0xFF),
        UInt8((origLen >> 16) & 0xFF),
        UInt8((origLen >> 8)  & 0xFF),
        UInt8(origLen         & 0xFF),
        UInt8((blockCount >> 24) & 0xFF),
        UInt8((blockCount >> 16) & 0xFF),
        UInt8((blockCount >> 8)  & 0xFF),
        UInt8(blockCount         & 0xFF),
    ]
    for block in blocks {
        buf.append(contentsOf: block)
    }
    return buf
}

/// Deserialises CMP02 bytes to a token list.
///
/// Security: caps `blockCount` against the actual payload size to prevent a
/// crafted header from causing unbounded iteration on minimal input.
///
/// - Parameter data: CMP02 binary bytes.
/// - Returns: `(tokens, originalLength)`.
public func deserialiseTokens(_ data: [UInt8]) -> ([Token], Int) {
    guard data.count >= 8 else { return ([], 0) }

    let originalLength = Int(
        UInt32(data[0]) << 24 | UInt32(data[1]) << 16
            | UInt32(data[2]) << 8 | UInt32(data[3])
    )
    var blockCount = Int(
        UInt32(data[4]) << 24 | UInt32(data[5]) << 16
            | UInt32(data[6]) << 8 | UInt32(data[7])
    )

    // Cap blockCount to prevent DoS from crafted headers.
    let maxPossible = data.count - 8
    blockCount = min(blockCount, maxPossible)

    var tokens: [Token] = []
    var pos = 8

    for _ in 0..<blockCount {
        guard pos < data.count else { break }
        let flag = data[pos]
        pos += 1

        for bit in 0..<8 {
            guard pos < data.count else { break }

            if (flag >> bit) & 1 == 1 {
                // Match: 3 bytes
                guard pos + 2 < data.count else { break }
                let offset = UInt16(data[pos]) << 8 | UInt16(data[pos + 1])
                let length = data[pos + 2]
                tokens.append(.match(offset: offset, length: length))
                pos += 3
            } else {
                // Literal: 1 byte
                tokens.append(.literal(data[pos]))
                pos += 1
            }
        }
    }

    return (tokens, originalLength)
}

// MARK: - One-Shot API

/// Compresses data using LZSS (CMP02 wire format).
///
/// One-shot API: `encode` then serialise the token stream to bytes.
///
/// - Parameters:
///   - data:       Input bytes.
///   - windowSize: Maximum lookback distance (default 4096).
///   - maxMatch:   Maximum match length (default 255).
///   - minMatch:   Minimum match length for back-references (default 3).
/// - Returns: Compressed bytes in CMP02 wire format.
///
/// ```swift
/// let compressed = compress(Array("hello hello".utf8))
/// decompress(compressed)  // Array("hello hello".utf8)
/// ```
public func compress(
    _ data: [UInt8],
    windowSize: Int = 4096,
    maxMatch: Int = 255,
    minMatch: Int = 3
) -> [UInt8] {
    let tokens = encode(data, windowSize: windowSize, maxMatch: maxMatch, minMatch: minMatch)
    return serialiseTokens(tokens, originalLength: data.count)
}

/// Decompresses data that was compressed with `compress`.
///
/// Deserialises the byte stream into tokens, then decodes them.
public func decompress(_ data: [UInt8]) -> [UInt8] {
    let (tokens, originalLength) = deserialiseTokens(data)
    return decode(tokens, originalLength: originalLength == 0 ? nil : originalLength)
}
