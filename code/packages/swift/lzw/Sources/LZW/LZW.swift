// LZW.swift
// Part of coding-adventures — an educational computing stack.
//
// ============================================================================
// LZW Lossless Compression Algorithm (1984)
// ============================================================================
//
// LZW (Lempel-Ziv-Welch, 1984) is LZ78 with a pre-seeded dictionary: all 256
// single-byte sequences are loaded before encoding begins (codes 0–255). This
// eliminates LZ78's mandatory next_char byte — every possible byte is already
// in the dictionary, so the encoder emits pure codes.
//
// With only codes to transmit, LZW uses variable-width bit-packing: codes start
// at 9 bits and grow as the dictionary expands. This is exactly how GIF works.
//
// Pre-Seeded Dictionary
// ---------------------
//
// LZ78 starts with an empty dictionary and emits (dict_index, next_char) for
// every literal. This means single-byte sequences always cost 3 bytes until
// the dictionary fills in. LZW initialises the dictionary upfront:
//
//   Code 0–255:  Single-byte entries [0x00] through [0xFF]
//   Code 256:    CLEAR_CODE — reset to initial 256-entry state
//   Code 257:    STOP_CODE  — marks end of code stream
//   Code 258+:   Dynamically added entries
//
// Reserved Codes
// --------------
//
//   CLEAR_CODE = 256  — instructs decoder to reset its dictionary
//   STOP_CODE  = 257  — end of compressed data
//
// The encoder emits CLEAR_CODE at the start of every stream and whenever the
// dictionary is full. The decoder resets whenever it sees CLEAR_CODE.
//
// Variable-Width Codes
// --------------------
//
//   Codes 0–511:    9 bits  (initial code size, covers 258+ reserved codes)
//   Codes 512–1023: 10 bits
//   ...
//   Codes 32768–65535: 16 bits  (maximum code size)
//
// Both encoder and decoder track nextCode and grow codeSize in lockstep, so
// they always agree on the current bit width.
//
// The Tricky Token (SC == NC edge case)
// --------------------------------------
//
// During decoding, the decoder may receive code C where C == nextCode (not yet
// added to the dictionary). This happens when the input has the form xyx…x,
// e.g. "AAAAAAA". The fix is:
//
//   entry = dict[prevCode] + [dict[prevCode][0]]
//
// This works because any self-referential code must encode a sequence that
// starts and ends with the same byte as the previous match (by construction).
//
// Wire Format (CMP03)
// -------------------
//
//   Bytes 0–3:  original_length (big-endian UInt32)
//   Bytes 4+:   bit-packed variable-width codes, LSB-first within each byte
//
// The Series: CMP00 → CMP05
// -------------------------
//
//   CMP00 (LZ77,    1977) — Sliding-window backreferences.
//   CMP01 (LZ78,    1978) — Explicit dictionary (trie), no sliding window.
//   CMP02 (LZSS,    1982) — LZ77 + flag bits; eliminates wasted literals.
//   CMP03 (LZW,     1984) — Pre-initialized dictionary; powers GIF. This module.
//   CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE.
//   CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib standard.

import Foundation

// MARK: - Constants

/// The code that instructs the decoder to reset its dictionary to the initial
/// 256-entry state and restart with nextCode = 258, codeSize = 9.
public let clearCode: UInt = 256

/// The code that marks the end of the compressed code stream.
public let stopCode: UInt = 257

/// The first dynamically assigned dictionary code (after the 256 pre-seeded
/// entries plus CLEAR_CODE and STOP_CODE).
public let initialNextCode: UInt = 258

/// Starting bit-width for codes. With 9 bits we can represent codes 0–511,
/// which covers the initial 258 codes (0–257) with headroom to spare.
public let initialCodeSize: UInt = 9

/// Maximum bit-width. With 16 bits the dictionary caps at 65536 entries.
public let maxCodeSize: UInt = 16

// MARK: - Bit I/O

/// BitWriter accumulates variable-width codes into a byte slice, LSB-first.
///
/// Bits within each byte are filled from the least-significant end. This
/// matches the GIF and Unix compress conventions.
///
/// Example — writing code 0b101 (5) at width 4:
///
///   buf = 0b0101   bitPos = 4
///   → no full byte yet
///
/// Writing code 0b11 (3) at width 4:
///
///   buf = 0b0011_0101   bitPos = 8
///   → flush: emit 0b0011_0101 (0x35)
///   buf = 0   bitPos = 0
///
struct BitWriter {
    var buf: UInt64 = 0
    var bitPos: UInt = 0
    var out: [UInt8] = []

    /// Writes `code` using exactly `codeSize` bits, LSB-first.
    ///
    /// Bits accumulate in `buf` from the low end. Whenever 8 or more bits are
    /// available, the low byte is emitted and shifted out.
    mutating func write(_ code: UInt, codeSize: UInt) {
        // Shift code into the buffer at the current bit position.
        buf |= UInt64(code) << bitPos
        bitPos += codeSize

        // Drain complete bytes from the low end of the buffer.
        while bitPos >= 8 {
            out.append(UInt8(buf & 0xFF))
            buf >>= 8
            bitPos -= 8
        }
    }

    /// Flushes any remaining bits (zero-padded to a byte boundary).
    mutating func flush() {
        if bitPos > 0 {
            out.append(UInt8(buf & 0xFF))
            buf = 0
            bitPos = 0
        }
    }
}

/// BitReader reads variable-width codes from a byte slice, LSB-first.
///
/// Mirrors BitWriter exactly. Bytes are loaded into the low end of `buf`
/// and consumed from the low end.
struct BitReader {
    var data: [UInt8]
    var pos: Int = 0
    var buf: UInt64 = 0
    var bitPos: UInt = 0

    /// Returns the next `codeSize`-bit code.
    ///
    /// If the stream is exhausted before `codeSize` bits are loaded,
    /// the best-effort partial value is returned (remaining bits are 0).
    mutating func read(codeSize: UInt) -> UInt? {
        // Fill the buffer until we have enough bits.
        while bitPos < codeSize {
            guard pos < data.count else {
                // Stream exhausted — return nil to signal end of input.
                return nil
            }
            buf |= UInt64(data[pos]) << bitPos
            pos += 1
            bitPos += 8
        }
        // Extract the low codeSize bits.
        let code = buf & ((1 << codeSize) - 1)
        buf >>= codeSize
        bitPos -= codeSize
        return UInt(code)
    }

    /// Returns true when no more bytes remain and the internal buffer is empty.
    var exhausted: Bool { pos >= data.count && bitPos == 0 }
}

// MARK: - Encoder

/// Encodes data into a slice of LZW codes including CLEAR_CODE and STOP_CODE.
///
/// The encode dictionary maps byte sequences (as `Data` for O(1) hashing) to
/// code numbers. We seed it with all 256 single-byte entries, then extend the
/// current prefix `w` byte by byte. When `w + b` is not in the dictionary:
///
/// 1. Emit the code for `w`.
/// 2. Add `w + b` to the dictionary (if room exists).
/// 3. Reset `w` to just `[b]`.
///
/// When the dictionary is full (nextCode reaches 2^maxCodeSize), emit CLEAR_CODE
/// and reset everything to the initial state.
///
/// - Returns: `(codes, originalLength)` — the code sequence and the byte count of
///   the original data (needed for the wire-format header).
public func encodeCodes(_ data: [UInt8]) -> ([UInt], Int) {
    let originalLength = data.count

    // Encoder dictionary: sequence (as Data) → code number.
    // We use Data as the key because it is Hashable and efficient for
    // byte-sequence equality.
    var encDict: [Data: UInt] = [:]
    encDict.reserveCapacity(512)

    // Seed with all 256 single-byte entries.
    for b in 0..<256 {
        encDict[Data([UInt8(b)])] = UInt(b)
    }

    var nextCode: UInt = initialNextCode
    let maxEntries: UInt = 1 << maxCodeSize

    // Every well-formed LZW stream starts with CLEAR_CODE.
    var codes: [UInt] = [clearCode]

    // `w` is the current working prefix — the longest sequence seen so far
    // that is already in the dictionary.
    var w = Data()

    for b in data {
        // Try to extend the current prefix by one byte.
        var wb = w
        wb.append(b)

        if encDict[wb] != nil {
            // Extended prefix is in the dictionary — keep growing.
            w = wb
        } else {
            // Extended prefix is NOT in the dictionary.
            // Emit the code for the current (known) prefix.
            codes.append(encDict[w]!)

            if nextCode < maxEntries {
                // Add the extended prefix as a new entry.
                encDict[wb] = nextCode
                nextCode += 1
            } else if nextCode == maxEntries {
                // Dictionary full — emit CLEAR and reset.
                codes.append(clearCode)
                encDict.removeAll(keepingCapacity: true)
                for i in 0..<256 {
                    encDict[Data([UInt8(i)])] = UInt(i)
                }
                nextCode = initialNextCode
            }

            // Restart the prefix with just the unmatched byte.
            w = Data([b])
        }
    }

    // Flush any remaining prefix.
    if !w.isEmpty {
        codes.append(encDict[w]!)
    }

    // Mark end of stream.
    codes.append(stopCode)
    return (codes, originalLength)
}

// MARK: - Decoder

/// Decodes a slice of LZW codes back to a byte slice.
///
/// The decode dictionary is a flat `[[UInt8]]` indexed by code. New entries
/// are built as `dict[prevCode] + [entry[0]]`.
///
/// The tricky-token case (code == nextCode) is handled by constructing the
/// missing entry from the previous entry extended by its own first byte.
///
/// Decoder state machine:
///
///   1. Read CLEAR_CODE first (well-formed streams always start with it).
///   2. For each subsequent code:
///      a. Look up entry in dict (or handle tricky token).
///      b. Append entry bytes to output.
///      c. Add new dict entry = dict[prevCode] + [entry[0]].
///      d. Advance nextCode; grow codeSize if needed.
///   3. Stop on STOP_CODE.
public func decodeCodes(_ codes: [UInt]) -> [UInt8] {
    // Initialise the decode dictionary with 256 single-byte entries plus
    // two placeholder slots for CLEAR_CODE (256) and STOP_CODE (257).
    var decDict: [[UInt8]] = (0..<256).map { [UInt8($0)] }
    decDict.append([]) // slot 256 — CLEAR_CODE placeholder
    decDict.append([]) // slot 257 — STOP_CODE placeholder

    var nextCode: UInt = initialNextCode

    var output: [UInt8] = []
    // `prevCode` is nil until the first data code is processed.
    var prevCode: UInt? = nil

    for code in codes {
        // ── Control codes ──────────────────────────────────────────────────
        if code == clearCode {
            // Reset the dictionary to just the 256 pre-seeded entries.
            decDict = (0..<256).map { [UInt8($0)] }
            decDict.append([]) // CLEAR_CODE placeholder
            decDict.append([]) // STOP_CODE placeholder
            nextCode = initialNextCode
            prevCode = nil
            continue
        }

        if code == stopCode {
            break
        }

        // ── Resolve the entry ──────────────────────────────────────────────
        let entry: [UInt8]

        if code < UInt(decDict.count) {
            // Normal case: code is already in the dictionary.
            entry = decDict[Int(code)]
        } else if code == nextCode {
            // Tricky token: code not yet in the dictionary.
            //
            // This happens when the encoded string has the form x·y·x…x where
            // the encoder added the new entry (x…) just before emitting it.
            // The decoder can reconstruct it because any such entry must begin
            // and end with the same byte as the previous entry's first byte.
            //
            // Entry = dict[prevCode] + [dict[prevCode][0]]
            guard let prev = prevCode, prev < UInt(decDict.count) else {
                // Malformed stream: tricky token with no prior code. Skip.
                continue
            }
            let prevEntry = decDict[Int(prev)]
            entry = prevEntry + [prevEntry[0]]
        } else {
            // Invalid code: skip and continue (graceful degradation).
            continue
        }

        // Append the resolved entry to the output.
        output.append(contentsOf: entry)

        // ── Add a new dictionary entry ─────────────────────────────────────
        // New entry = dict[prevCode] + [entry[0]]
        // This mirrors what the encoder did: when it emitted `prevCode` and
        // started a new prefix with `entry[0]`, it added that combination.
        if let prev = prevCode, nextCode < (1 << maxCodeSize) {
            let prevEntry = decDict[Int(prev)]
            decDict.append(prevEntry + [entry[0]])
            nextCode += 1
            // Note: codeSize growth tracking happens in unpackCodes, not here.
        }

        prevCode = code
    }

    return output
}

// MARK: - Serialisation

/// Packs a list of LZW codes into the CMP03 wire format.
///
/// The code size starts at `initialCodeSize` (9) and grows whenever `nextCode`
/// crosses the next power-of-2 boundary. CLEAR_CODE resets the code size back
/// to 9.
///
/// Wire format:
///
///   Bytes 0–3:  original_length (big-endian UInt32)
///   Bytes 4+:   bit-packed variable-width codes, LSB-first
///
/// The code-size tracking rule (applied after writing each data code):
///
///   nextCode += 1
///   if nextCode > (1 << codeSize) && codeSize < maxCodeSize {
///       codeSize += 1
///   }
///
/// This ensures encoder and decoder always use the same bit-width for each code.
public func packCodes(_ codes: [UInt], originalLength: Int) -> [UInt8] {
    var writer = BitWriter()
    var codeSize: UInt = initialCodeSize
    var nextCode: UInt = initialNextCode

    for code in codes {
        writer.write(code, codeSize: codeSize)

        if code == clearCode {
            // Reset code-size tracking after a CLEAR.
            codeSize = initialCodeSize
            nextCode = initialNextCode
        } else if code != stopCode {
            // Data code: advance nextCode and grow codeSize if needed.
            if nextCode < (1 << maxCodeSize) {
                nextCode += 1
                if nextCode > (1 << codeSize) && codeSize < maxCodeSize {
                    codeSize += 1
                }
            }
        }
    }
    writer.flush()

    // Prepend the 4-byte big-endian original_length header.
    let orig = UInt32(originalLength)
    let header: [UInt8] = [
        UInt8((orig >> 24) & 0xFF),
        UInt8((orig >> 16) & 0xFF),
        UInt8((orig >>  8) & 0xFF),
        UInt8( orig        & 0xFF),
    ]
    return header + writer.out
}

/// Reads CMP03 wire-format bytes and returns the list of LZW codes plus the
/// original data length stored in the header.
///
/// Stops at STOP_CODE or stream exhaustion. Returns `([clearCode, stopCode], 0)`
/// for input shorter than 4 bytes (the minimum valid header size).
public func unpackCodes(_ data: [UInt8]) -> ([UInt], Int) {
    guard data.count >= 4 else {
        // Too short to contain even a valid header — treat as empty stream.
        return ([clearCode, stopCode], 0)
    }

    // Read the big-endian original_length from the first 4 bytes.
    let originalLength = Int(
        UInt32(data[0]) << 24 | UInt32(data[1]) << 16
            | UInt32(data[2]) << 8 | UInt32(data[3])
    )

    var reader = BitReader(data: Array(data[4...]))
    var codeSize: UInt = initialCodeSize
    var nextCode: UInt = initialNextCode

    var codes: [UInt] = []

    while !reader.exhausted {
        guard let code = reader.read(codeSize: codeSize) else { break }
        codes.append(code)

        if code == stopCode {
            // Well-formed stream ends with STOP_CODE.
            return (codes, originalLength)
        } else if code == clearCode {
            // Reset code-size tracking after a CLEAR.
            codeSize = initialCodeSize
            nextCode = initialNextCode
        } else {
            // Data code: advance nextCode and grow codeSize if needed.
            if nextCode < (1 << maxCodeSize) {
                nextCode += 1
                if nextCode > (1 << codeSize) && codeSize < maxCodeSize {
                    codeSize += 1
                }
            }
        }
    }

    return (codes, originalLength)
}

// MARK: - Public API

/// Compresses data using LZW and returns the CMP03 wire-format bytes.
///
/// The returned bytes begin with a 4-byte big-endian `original_length` header
/// followed by LSB-first variable-width bit-packed codes.
///
/// ```swift
/// let compressed = compress(Array("hello hello".utf8))
/// let original   = decompress(compressed)
/// // original == Array("hello hello".utf8)
/// ```
///
/// - Parameter data: Input bytes to compress.
/// - Returns: Compressed bytes in CMP03 wire format.
public func compress(_ data: [UInt8]) -> [UInt8] {
    let (codes, originalLength) = encodeCodes(data)
    return packCodes(codes, originalLength: originalLength)
}

/// Decompresses CMP03 wire-format data and returns the original bytes.
///
/// - Parameter data: Compressed bytes in CMP03 wire format.
/// - Returns: Reconstructed original bytes, truncated to the stored `original_length`.
public func decompress(_ data: [UInt8]) -> [UInt8] {
    let (codes, originalLength) = unpackCodes(data)
    var result = decodeCodes(codes)
    if originalLength > 0 && result.count > originalLength {
        result = Array(result.prefix(originalLength))
    }
    return result
}
