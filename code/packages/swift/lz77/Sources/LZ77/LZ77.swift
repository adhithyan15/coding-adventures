// LZ77.swift
// Part of coding-adventures — an educational computing stack.
//
// ============================================================================
// LZ77 Lossless Compression Algorithm (1977)
// ============================================================================
//
// LZ77 (Lempel & Ziv, 1977) replaces repeated byte sequences with compact
// backreferences into a sliding window of recently seen data. It is the
// foundation of DEFLATE, gzip, PNG, and zlib.
//
// The Sliding Window Model
// ------------------------
//
//     ┌─────────────────────────────────┬──────────────────┐
//     │         SEARCH BUFFER           │ LOOKAHEAD BUFFER  │
//     │  (already processed — the       │  (not yet seen —  │
//     │   last windowSize bytes)        │  next maxMatch)   │
//     └─────────────────────────────────┴──────────────────┘
//                                        ↑
//                                    cursor (current position)
//
// At each step the encoder finds the longest match in the search buffer. If
// found and long enough (≥ minMatch), emit a backreference token. Otherwise
// emit a literal token.
//
// Token: (offset, length, nextChar)
// -----------------------------------
//
// - offset:   distance back the match starts (1..windowSize), or 0.
// - length:   number of bytes the match covers (0 = literal).
// - nextChar: literal byte immediately after the match.
//
// Overlapping Matches
// -------------------
//
// When offset < length, the match extends into bytes not yet decoded. The
// decoder must copy byte-by-byte (not bulk copy) to handle this correctly.
//
// Example: output = [A, B], token = (offset=2, length=5, nextChar='Z')
//   Copy byte-by-byte → [A,B,A,B,A,B,A], then append Z → ABABABAZ (8 bytes)
//
// The Series: CMP00 → CMP05
// -------------------------
//
//   CMP00 (LZ77, 1977) — Sliding-window backreferences. This module.
//   CMP01 (LZ78, 1978) — Explicit dictionary (trie), no sliding window.
//   CMP02 (LZSS, 1982) — LZ77 + flag bits; eliminates wasted literals.
//   CMP03 (LZW,  1984) — Pre-initialized dictionary; powers GIF.
//   CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE.
//   CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib standard.

import Foundation

// MARK: - Token

/// A single LZ77 token: `(offset, length, nextChar)`.
///
/// Represents one unit of the compressed stream.
///
/// - `offset`:   distance back the match starts (1..windowSize), or 0.
/// - `length`:   number of bytes the match covers (0 = literal).
/// - `nextChar`: literal byte immediately after the match (0..255).
public struct Token: Equatable {
    public let offset: UInt16
    public let length: UInt8
    public let nextChar: UInt8

    public init(offset: UInt16, length: UInt8, nextChar: UInt8) {
        self.offset = offset
        self.length = length
        self.nextChar = nextChar
    }
}

// MARK: - Encoder

/// Finds the longest match in the search buffer.
///
/// Scans the last `windowSize` bytes before `cursor` for the longest substring
/// matching the start of the lookahead buffer.
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
    // Reserve 1 byte for nextChar.
    let lookaheadEnd = min(cursor + maxMatch, data.count - 1)

    for pos in searchStart..<cursor {
        var length = 0
        // Match byte by byte. Matches may overlap (extend past cursor).
        while cursor + length < lookaheadEnd
            && data[pos + length] == data[cursor + length]
        {
            length += 1
        }
        if length > bestLength {
            bestLength = length
            bestOffset = cursor - pos  // Distance back from cursor.
        }
    }

    return (bestOffset, bestLength)
}

/// Encodes data into an LZ77 token stream.
///
/// Scans the input left-to-right. For each position, finds the longest match
/// in the search buffer. If the match is long enough (≥ minMatch), emits a
/// backreference token; otherwise emits a literal token.
///
/// - Parameters:
///   - data:       Input bytes.
///   - windowSize: Maximum lookback distance (default 4096).
///   - maxMatch:   Maximum match length (default 255).
///   - minMatch:   Minimum length for a backreference (default 3).
/// - Returns: Array of tokens representing the compressed stream.
///
/// ```swift
/// let tokens = encode([65, 66, 65, 66, 65, 66, 65, 66])
/// // tokens.count == 3: two literals + one backreference
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
        // Edge case: last byte has no room for nextChar after a match.
        if cursor == data.count - 1 {
            tokens.append(Token(offset: 0, length: 0, nextChar: data[cursor]))
            cursor += 1
            continue
        }

        let (offset, length) = findLongestMatch(data: data, cursor: cursor, windowSize: windowSize, maxMatch: maxMatch)

        if length >= minMatch {
            // Emit a backreference token.
            let nextChar = data[cursor + length]
            tokens.append(Token(offset: UInt16(offset), length: UInt8(length), nextChar: nextChar))
            cursor += length + 1
        } else {
            // Emit a literal token (no match or too short).
            tokens.append(Token(offset: 0, length: 0, nextChar: data[cursor]))
            cursor += 1
        }
    }

    return tokens
}

// MARK: - Decoder

/// Decodes an LZ77 token stream back into the original bytes.
///
/// Processes each token: if `length > 0`, copies `length` bytes byte-by-byte
/// from the search buffer (handling overlapping matches), then appends
/// `nextChar`.
///
/// - Parameters:
///   - tokens:        The token stream (output of `encode`).
///   - initialBuffer: Optional seed for the search buffer (streaming use).
/// - Returns: Reconstructed bytes.
///
/// ```swift
/// let tokens = [Token(offset: 0, length: 0, nextChar: 65),
///               Token(offset: 1, length: 3, nextChar: 68)]
/// decode(tokens)  // [65, 65, 65, 65, 68] = "AAAAD"
/// ```
public func decode(_ tokens: [Token], initialBuffer: [UInt8] = []) -> [UInt8] {
    var output: [UInt8] = initialBuffer

    for token in tokens {
        if token.length > 0 {
            // Copy length bytes from position (output.count - offset).
            let start = output.count - Int(token.offset)
            // Copy byte-by-byte to handle overlapping matches (offset < length).
            for i in 0..<Int(token.length) {
                output.append(output[start + i])
            }
        }
        // Always append nextChar.
        output.append(token.nextChar)
    }

    return output
}

// MARK: - Serialisation

/// Serialises a token list to bytes using a fixed-width format.
///
/// Format:
/// - 4 bytes: token count (big-endian UInt32)
/// - N × 4 bytes: each token as `(offset: UInt16 BE, length: UInt8, nextChar: UInt8)`
///
/// This is a teaching format. Production compressors use variable-width
/// bit-packing (see DEFLATE, zstd).
public func serialiseTokens(_ tokens: [Token]) -> [UInt8] {
    var buf: [UInt8] = []
    // Write token count as big-endian UInt32.
    let count = UInt32(tokens.count)
    buf.append(UInt8((count >> 24) & 0xFF))
    buf.append(UInt8((count >> 16) & 0xFF))
    buf.append(UInt8((count >> 8)  & 0xFF))
    buf.append(UInt8(count         & 0xFF))

    for token in tokens {
        buf.append(UInt8((token.offset >> 8) & 0xFF))
        buf.append(UInt8(token.offset        & 0xFF))
        buf.append(token.length)
        buf.append(token.nextChar)
    }

    return buf
}

/// Deserialises bytes back into a token list.
///
/// Inverse of `serialiseTokens`.
public func deserialiseTokens(_ data: [UInt8]) -> [Token] {
    guard data.count >= 4 else { return [] }

    let count = Int(UInt32(data[0]) << 24 | UInt32(data[1]) << 16
                    | UInt32(data[2]) << 8 | UInt32(data[3]))
    var tokens: [Token] = []

    for i in 0..<count {
        let base = 4 + i * 4
        guard base + 4 <= data.count else { break }

        let offset = UInt16(data[base]) << 8 | UInt16(data[base + 1])
        let length = data[base + 2]
        let nextChar = data[base + 3]
        tokens.append(Token(offset: offset, length: length, nextChar: nextChar))
    }

    return tokens
}

// MARK: - One-Shot API

/// Compresses data using LZ77.
///
/// One-shot API: `encode` then serialise the token stream to bytes.
///
/// - Parameters:
///   - data:       Input bytes.
///   - windowSize: Maximum lookback distance (default 4096).
///   - maxMatch:   Maximum match length (default 255).
///   - minMatch:   Minimum match length for backreferences (default 3).
/// - Returns: Compressed bytes.
///
/// ```swift
/// let compressed = compress([65, 65, 65, 65, 65, 65, 65])
/// decompress(compressed)  // [65, 65, 65, 65, 65, 65, 65]
/// ```
public func compress(
    _ data: [UInt8],
    windowSize: Int = 4096,
    maxMatch: Int = 255,
    minMatch: Int = 3
) -> [UInt8] {
    let tokens = encode(data, windowSize: windowSize, maxMatch: maxMatch, minMatch: minMatch)
    return serialiseTokens(tokens)
}

/// Decompresses data that was compressed with `compress`.
///
/// Deserialises the byte stream into tokens, then decodes.
public func decompress(_ data: [UInt8]) -> [UInt8] {
    let tokens = deserialiseTokens(data)
    return decode(tokens)
}
