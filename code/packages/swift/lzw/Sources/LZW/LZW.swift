// =============================================================================
// LZW — CMP03
// =============================================================================
//
// LZW (Lempel-Ziv-Welch, 1984) lossless compression algorithm.
// Part of the CMP compression series in the coding-adventures monorepo.
//
// What Is LZW?
// ------------
//
// LZW is LZ78 with a pre-seeded dictionary: all 256 single-byte sequences are
// added before encoding begins (codes 0–255). This eliminates LZ78's mandatory
// next_char byte — every symbol is already in the dictionary, so the encoder
// can emit pure codes.
//
// With only codes to transmit, LZW uses variable-width bit-packing: codes start
// at 9 bits and grow as the dictionary expands. This is exactly how GIF works.
//
// Reserved Codes
// --------------
//
//   0–255:  Pre-seeded single-byte entries.
//   256:    clearCode — reset to initial 256-entry state.
//   257:    stopCode  — end of code stream.
//   258+:   Dynamically added entries.
//
// Wire Format (CMP03)
// -------------------
//
//   Bytes 0–3:  original_length (big-endian UInt32)
//   Bytes 4+:   bit-packed variable-width codes, LSB-first
//
// The Tricky Token
// ----------------
//
// During decoding the decoder may receive code C == nextCode (not yet added).
// This happens when the input has the form xyx...x. The fix:
//
//   entry = dict[prevCode] + [dict[prevCode][0]]
//
// The Series
// ----------
//
//   CMP00 (LZ77,    1977) — Sliding-window backreferences.
//   CMP01 (LZ78,    1978) — Explicit dictionary (trie).
//   CMP02 (LZSS,    1982) — LZ77 + flag bits; no wasted literals.
//   CMP03 (LZW,     1984) — LZ78 + pre-initialized dict; GIF. (this module)
//   CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE.
//   CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
// =============================================================================

import Foundation

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

public let clearCode: UInt32        = 256
public let stopCode: UInt32         = 257
public let initialNextCode: UInt32  = 258
public let initialCodeSize: UInt32  = 9
public let maxCodeSize: UInt32      = 16

// ---------------------------------------------------------------------------
// Bit I/O
// ---------------------------------------------------------------------------

/// Accumulates variable-width codes into a byte array, LSB-first.
private struct BitWriter {
    var buf: UInt64 = 0
    var bitPos: UInt32 = 0
    var output: [UInt8] = []

    mutating func write(_ code: UInt32, size: UInt32) {
        buf |= UInt64(code) << bitPos
        bitPos += size
        while bitPos >= 8 {
            output.append(UInt8(buf & 0xFF))
            buf >>= 8
            bitPos -= 8
        }
    }

    mutating func flush() {
        if bitPos > 0 {
            output.append(UInt8(buf & 0xFF))
            buf = 0
            bitPos = 0
        }
    }
}

/// Reads variable-width codes from a byte array, LSB-first.
private struct BitReader {
    let data: [UInt8]
    var pos: Int = 0
    var buf: UInt64 = 0
    var bitPos: UInt32 = 0

    init(_ data: [UInt8]) { self.data = data }

    var exhausted: Bool { pos >= data.count && bitPos == 0 }

    mutating func read(size: UInt32) -> UInt32? {
        while bitPos < size {
            guard pos < data.count else { return nil }
            buf |= UInt64(data[pos]) << bitPos
            pos += 1
            bitPos += 8
        }
        let mask: UInt64 = (1 << size) - 1
        let code = UInt32(buf & mask)
        buf >>= size
        bitPos -= size
        return code
    }
}

// ---------------------------------------------------------------------------
// Encoder
// ---------------------------------------------------------------------------

/// Encode `data` into an array of LZW codes including clearCode and stopCode.
///
/// Returns (codes, originalLength). The encode dictionary maps byte sequences
/// (stored as [UInt8]) to codes. The encoder walks the input byte-by-byte,
/// extending the current prefix; when prefix+new byte is not in the dict, the
/// prefix's code is emitted, the new sequence added (if room), and prefix resets.
func encodeCodes(_ data: [UInt8]) -> ([UInt32], Int) {
    let originalLength = data.count
    var encDict: [ArraySlice<UInt8>: UInt32] = [:]
    // Seed with all 256 single-byte sequences.
    for b in 0..<256 {
        let key = ArraySlice([UInt8(b)])
        encDict[key] = UInt32(b)
    }

    var nextCode = initialNextCode
    let maxEntries = UInt32(1) << maxCodeSize
    var codes: [UInt32] = [clearCode]
    var w: [UInt8] = []

    for byte in data {
        var wb = w
        wb.append(byte)
        let wbSlice = ArraySlice(wb)
        if encDict[wbSlice] != nil {
            w = wb
        } else {
            codes.append(encDict[ArraySlice(w)]!)

            if nextCode < maxEntries {
                encDict[wbSlice] = nextCode
                nextCode += 1
            } else if nextCode == maxEntries {
                codes.append(clearCode)
                encDict.removeAll()
                for b in 0..<256 { encDict[ArraySlice([UInt8(b)])] = UInt32(b) }
                nextCode = initialNextCode
            }

            w = [byte]
        }
    }

    if !w.isEmpty {
        codes.append(encDict[ArraySlice(w)]!)
    }
    codes.append(stopCode)
    return (codes, originalLength)
}

// ---------------------------------------------------------------------------
// Decoder
// ---------------------------------------------------------------------------

/// Decode an array of LZW codes back to a byte array.
///
/// Handles clearCode (reset), stopCode (done), and the tricky-token edge case
/// (code == nextCode).
func decodeCodes(_ codes: [UInt32]) -> [UInt8] {
    var decDict: [[UInt8]] = (0..<256).map { [UInt8($0)] }
    decDict.append([]) // 256 = clearCode placeholder
    decDict.append([]) // 257 = stopCode  placeholder

    var nextCode = initialNextCode
    let maxEntries = UInt32(1) << maxCodeSize
    var output: [UInt8] = []
    var prevCode: UInt32? = nil

    for code in codes {
        if code == clearCode {
            decDict = (0..<256).map { [UInt8($0)] }
            decDict.append([])
            decDict.append([])
            nextCode = initialNextCode
            prevCode = nil
            continue
        }
        if code == stopCode { break }

        let entry: [UInt8]
        if Int(code) < decDict.count {
            entry = decDict[Int(code)]
        } else if code == nextCode, let prev = prevCode {
            // Tricky token.
            let prevEntry = decDict[Int(prev)]
            guard !prevEntry.isEmpty else { continue }
            entry = prevEntry + [prevEntry[0]]
        } else {
            continue // invalid
        }

        output.append(contentsOf: entry)

        if let prev = prevCode, nextCode < maxEntries {
            let prevEntry = decDict[Int(prev)]
            decDict.append(prevEntry + [entry[0]])
            nextCode += 1
        }

        prevCode = code
    }

    return output
}

// ---------------------------------------------------------------------------
// Serialisation
// ---------------------------------------------------------------------------

/// Pack an array of LZW codes into the CMP03 wire format.
func packCodes(_ codes: [UInt32], originalLength: Int) -> [UInt8] {
    var writer = BitWriter()
    var codeSize = initialCodeSize
    var nextCode = initialNextCode
    let maxEntries = UInt32(1) << maxCodeSize

    for code in codes {
        writer.write(code, size: codeSize)
        if code == clearCode {
            codeSize = initialCodeSize
            nextCode = initialNextCode
        } else if code != stopCode {
            if nextCode < maxEntries {
                nextCode += 1
                if nextCode > (1 << codeSize) && codeSize < maxCodeSize {
                    codeSize += 1
                }
            }
        }
    }
    writer.flush()

    var result = [UInt8](repeating: 0, count: 4 + writer.output.count)
    // Write original_length as big-endian UInt32: shift the native value directly.
    // (Do NOT call .bigEndian first — that reorders bytes for memory layout, and
    // then shifting the reordered value produces wrong results.)
    let len = UInt32(originalLength)
    result[0] = UInt8((len >> 24) & 0xFF)
    result[1] = UInt8((len >> 16) & 0xFF)
    result[2] = UInt8((len >> 8)  & 0xFF)
    result[3] = UInt8( len        & 0xFF)
    result.replaceSubrange(4..., with: writer.output)
    return result
}

/// Unpack CMP03 wire-format bytes into an array of LZW codes.
/// Returns (codes, originalLength).
func unpackCodes(_ data: [UInt8]) -> ([UInt32], Int) {
    guard data.count >= 4 else { return ([clearCode, stopCode], 0) }

    let originalLength = Int(
        UInt32(data[0]) << 24 | UInt32(data[1]) << 16 |
        UInt32(data[2]) << 8  | UInt32(data[3])
    )
    var reader = BitReader(Array(data[4...]))
    var codes: [UInt32] = []
    var codeSize = initialCodeSize
    var nextCode = initialNextCode
    let maxEntries = UInt32(1) << maxCodeSize

    while !reader.exhausted {
        guard let code = reader.read(size: codeSize) else { break }
        codes.append(code)
        if code == stopCode { break }
        else if code == clearCode {
            codeSize = initialCodeSize
            nextCode = initialNextCode
        } else if nextCode < maxEntries {
            nextCode += 1
            if nextCode > (1 << codeSize) && codeSize < maxCodeSize {
                codeSize += 1
            }
        }
    }

    return (codes, originalLength)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compress `data` using LZW and return CMP03 wire-format bytes.
public func compress(_ data: [UInt8]) -> [UInt8] {
    let (codes, originalLength) = encodeCodes(data)
    return packCodes(codes, originalLength: originalLength)
}

/// Decompress CMP03 wire-format `data` and return the original bytes.
public func decompress(_ data: [UInt8]) -> [UInt8] {
    let (codes, originalLength) = unpackCodes(data)
    var result = decodeCodes(codes)
    if result.count > originalLength { result = Array(result.prefix(originalLength)) }
    return result
}
