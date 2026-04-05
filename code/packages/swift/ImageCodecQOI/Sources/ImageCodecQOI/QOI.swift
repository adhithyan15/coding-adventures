// QOI.swift
// Part of coding-adventures — IC03: QOI image encoder/decoder.
//
// ============================================================================
// MARK: - IC03: QOI (Quite OK Image) Codec
// ============================================================================
//
// QOI is a lossless image format designed by Dominic Szablewski (2021).
// Its spec is ~1 page. Its reference encoder and decoder are ~300 lines of C.
// It achieves 2-4x compression on typical images and near-PNG compression
// on pixel art, while encoding faster than PNG and decoding comparably.
//
// Reference spec: https://qoiformat.org/qoi-specification.pdf
//
// ============================================================================
// QOI File Structure
// ============================================================================
//
//   Bytes   Field
//   ──────  ──────────────────────────────────────────────────────────────
//   0..3    Magic: "qoif" (0x71, 0x6F, 0x69, 0x66)
//   4..7    Width  (big-endian uint32)
//   8..11   Height (big-endian uint32)
//   12      Channels: 3 = RGB, 4 = RGBA
//   13      Colorspace: 0 = sRGB with linear alpha, 1 = all linear
//   14..N   Compressed chunk stream
//   N+1..   End marker: 8 bytes [0,0,0,0,0,0,0,1]
//
// ============================================================================
// QOI Compression: The 6 Chunk Types
// ============================================================================
//
// QOI is a simple run-based codec. Each pixel is encoded as one of 6 chunk
// types, chosen by comparing the current pixel to the previous pixel and to
// a 64-entry hash table (the "seen pixels" array). The encoder tries each
// type in priority order and picks the first that applies.
//
// ─────────────────────────────────────────────────────────────────────────
// 1. QOI_OP_RGB   (3 bytes)
//    Tag: 0b_1111_1110 = 0xFE
//    Payload: R, G, B bytes (unchanged alpha from previous pixel)
//    Used when: alpha has not changed but none of the cheaper encodings apply.
//
//    Byte layout:
//      [0xFE] [R] [G] [B]
//
// ─────────────────────────────────────────────────────────────────────────
// 2. QOI_OP_RGBA  (4 bytes)
//    Tag: 0b_1111_1111 = 0xFF
//    Payload: R, G, B, A bytes.
//    Used when: no cheaper encoding applies AND alpha has changed.
//
//    Byte layout:
//      [0xFF] [R] [G] [B] [A]
//
// ─────────────────────────────────────────────────────────────────────────
// 3. QOI_OP_INDEX (1 byte)
//    Tag: 0b_00xx_xxxx = top 2 bits 00
//    Payload: 6-bit index into the 64-entry seen-pixels hash table.
//    Used when: the current pixel is already in the hash table.
//
//    Byte layout:
//      [00 | index(6)]
//
//    Hash function: index = (R×3 + G×5 + B×7 + A×11) % 64
//    The hash table stores complete RGBA values.
//    On encode: if seen[hash(px)] == px, emit index.
//    On decode: load px from seen[index].
//
// ─────────────────────────────────────────────────────────────────────────
// 4. QOI_OP_DIFF  (1 byte)
//    Tag: 0b_01xx_xxxx = top 2 bits 01
//    Payload: three 2-bit signed differences: dr, dg, db ∈ {-2, -1, 0, 1}.
//    Bias: each difference is stored as (delta + 2) so it fits in 2 bits (0..3).
//    Used when: dr, dg, db are all in [-2, +1] AND alpha has not changed.
//
//    Byte layout:
//      [01 | (dr+2)(2) | (dg+2)(2) | (db+2)(2)]
//
//    Why bias +2? A signed 2-bit field holds {0,1,2,3}. We want {-2,-1,0,1}.
//    Adding 2 maps -2→0, -1→1, 0→2, 1→3. Subtracting 2 on decode restores
//    the original difference.
//
// ─────────────────────────────────────────────────────────────────────────
// 5. QOI_OP_LUMA  (2 bytes)
//    Tag: 0b_10xx_xxxx = top 2 bits 10
//    Payload:
//      Byte 1: [10 | (dg+32)(6)]          — green difference, bias +32
//      Byte 2: [(dr-dg+8)(4) | (db-dg+8)(4)] — red/blue relative to green
//    Used when: dg ∈ [-32, +31] AND (dr-dg), (db-dg) ∈ [-8, +7] AND alpha unchanged.
//
//    Byte layout:
//      [10 | (dg+32)(6)] [(dr-dg+8)(4) | (db-dg+8)(4)]
//
//    Why encode relative to green? Green carries most of the luminance in
//    natural images. Red and blue channels tend to change in proportion to
//    green. Encoding dr and db relative to dg gives a tighter range for
//    most images, allowing 2-byte encoding where QOI_OP_DIFF would need
//    3-byte QOI_OP_RGB.
//
// ─────────────────────────────────────────────────────────────────────────
// 6. QOI_OP_RUN   (1 byte)
//    Tag: 0b_11xx_xxxx = top 2 bits 11
//    Payload: 6-bit run length - 1 (bias: stored as run-1 so 0 means run=1).
//    Used when: the current pixel equals the previous pixel.
//    Maximum run: 62 (stored as 61). Runs of 63 and 64 are reserved for the
//    RGB and RGBA op tags (0b_1111_1110 and 0b_1111_1111).
//
//    Byte layout:
//      [11 | (run-1)(6)]
//
// ─────────────────────────────────────────────────────────────────────────
//
// ============================================================================
// Delta Wrapping
// ============================================================================
//
// Channel differences are computed modulo 256 (wrapping arithmetic).
// For example, if prev.R = 2 and curr.R = 0, the difference is -2, which
// wraps to 254 in UInt8. We need to detect this as -2 (fits in DIFF) rather
// than 254 (doesn't fit).
//
// The wrap helper converts an unsigned byte difference to a signed value in
// the range [-128, 127]:
//
//   wrap(delta: UInt8) -> Int:
//     ((Int(delta) + 128) & 0xFF) - 128
//
// Wait — actually the spec requires wrapping differences. The correct way to
// compute the signed difference between two UInt8 values a and b is:
//
//   let d = Int(a) - Int(b)   (range: -255..255 before wrapping)
//   wrapping to [-128..127]:
//   let wrapped = ((d & 0xFF) + 128) & 0xFF - 128
//
// But the spec says to interpret the channel values modulo 256 when adding
// differences back. So for encoding we just do:
//
//   dr = Int(curr.r) - Int(prev.r)   (range: -255..255)
//
// And check if dr is in [-2..1] for DIFF, or [-32..31] for LUMA.
// We don't need wrapping for the range check — if dr is outside [-128..127]
// (e.g., prev.R = 250, curr.R = 5: dr = 5 - 250 = -245), we fall through to
// QOI_OP_RGB or QOI_OP_RGBA.
//
// On decode, we add the difference back and let UInt8 wrap naturally:
//   newR = UInt8((Int(prev.r) + dr) & 0xFF)
//
// ============================================================================
// Seen-Pixels Hash Table
// ============================================================================
//
// The hash table is a 64-entry array of RGBA tuples, initialised to all zeros.
// Before encoding/decoding each pixel, we update the table:
//
//   seen[hash(px)] = px
//
// where hash(r, g, b, a) = (r×3 + g×5 + b×7 + a×11) % 64.
//
// On encode: if the current pixel is in seen[hash(px)], emit QOI_OP_INDEX.
// On decode: when we see QOI_OP_INDEX, load seen[index] as the current pixel.
//
// The table is not a cache of recently seen pixels — it's a hash map. Two
// different pixels with the same hash overwrite each other, which is fine
// because the decoder uses the same hash to look them up.
//
// ============================================================================

import PixelContainer

// ============================================================================
// MARK: - Error Type
// ============================================================================

/// Errors produced by the QOI codec.
public enum ImageCodecQOIError: Error, Equatable {
    /// The byte array does not start with the "qoif" magic bytes.
    case invalidMagic
    /// The header is shorter than 14 bytes.
    case truncatedHeader
    /// Width or height is zero.
    case invalidDimensions
    /// The channels field is not 3 or 4.
    case unsupportedChannels
    /// The compressed chunk stream ended before all pixels were decoded.
    case truncatedData
    /// The end marker (8 bytes [0,0,0,0,0,0,0,1]) was not found after all pixels.
    case missingEndMarker
}

// ============================================================================
// MARK: - Internal Pixel Representation
// ============================================================================

/// A four-channel RGBA pixel used internally during encode/decode.
///
/// We use a struct rather than a tuple so we can store it in arrays and
/// compare two pixels for equality easily.
struct Pixel: Equatable {
    var r: UInt8
    var g: UInt8
    var b: UInt8
    var a: UInt8

    static let zero = Pixel(r: 0, g: 0, b: 0, a: 0)
}

// ============================================================================
// MARK: - Hash Function
// ============================================================================

/// Compute the 64-entry hash-table index for a pixel.
///
/// The hash function from the QOI spec:
///
///   index = (R × 3 + G × 5 + B × 7 + A × 11) mod 64
///
/// This is a simple linear combination with prime coefficients. Prime
/// multipliers help distribute pixels uniformly across the 64 slots and
/// reduce collisions for common colour patterns (gradients, solid areas).
///
/// The modulo 64 restricts the result to 6 bits (0..63), matching the
/// 6-bit index field in QOI_OP_INDEX.
///
/// - Parameter px: The RGBA pixel to hash.
/// - Returns: An index in 0..63.
func qoiHash(_ px: Pixel) -> Int {
    // Using Int to avoid overflow before taking modulo.
    return (Int(px.r) * 3 + Int(px.g) * 5 + Int(px.b) * 7 + Int(px.a) * 11) % 64
}

// ============================================================================
// MARK: - Big-Endian Helpers
// ============================================================================
//
// QOI uses BIG-endian byte order for the header fields (unlike BMP/QOI which
// use little-endian). Big-endian means the most significant byte comes first.
//
//   val = 0x12345678
//   bytes: [0x12, 0x34, 0x56, 0x78]
//          ↑ MSB first

/// Write a `UInt32` in big-endian byte order into `buf` starting at `offset`.
func writeBE32(_ val: UInt32, into buf: inout [UInt8], at offset: Int) {
    buf[offset]     = UInt8((val >> 24) & 0xFF)  // MSB first
    buf[offset + 1] = UInt8((val >> 16) & 0xFF)
    buf[offset + 2] = UInt8((val >> 8)  & 0xFF)
    buf[offset + 3] = UInt8(val & 0xFF)           // LSB last
}

/// Read a `UInt32` in big-endian byte order from `buf` starting at `offset`.
func readBE32(_ buf: [UInt8], at offset: Int) -> UInt32 {
    let b0 = UInt32(buf[offset])     // MSB
    let b1 = UInt32(buf[offset + 1])
    let b2 = UInt32(buf[offset + 2])
    let b3 = UInt32(buf[offset + 3]) // LSB
    return (b0 << 24) | (b1 << 16) | (b2 << 8) | b3
}

// ============================================================================
// MARK: - End Marker
// ============================================================================

/// QOI end marker: 7 zero bytes followed by a 0x01 byte.
///
/// After the last pixel chunk, QOI appends this 8-byte sequence to signal
/// the end of the compressed stream. It is chosen so that it cannot appear
/// as a valid chunk (no valid chunk starts with 7 zero bytes in sequence).
let qoiEndMarker: [UInt8] = [0, 0, 0, 0, 0, 0, 0, 1]

// ============================================================================
// MARK: - Encode
// ============================================================================

/// Encode a `PixelContainer` as a QOI byte array.
///
/// Encoding algorithm:
///   1. Write the 14-byte header.
///   2. Maintain a 64-entry seen-pixels table (all zeroes initially).
///   3. Track the previous pixel (initially RGBA = 0, 0, 0, 255).
///   4. Track a run count (consecutive identical pixels).
///   5. For each pixel in row-major order:
///      a. If pixel == prev, increment run. Flush run at 62 or end.
///      b. Otherwise, flush any pending run, then try encodings in order:
///         INDEX → DIFF → LUMA → RGB → RGBA.
///      c. Update seen table: seen[hash(px)] = px.
///      d. Update prev = px.
///   6. Flush any remaining run.
///   7. Append 8-byte end marker.
///
/// - Parameter pixels: The source RGBA8 pixel buffer.
/// - Returns: A complete, valid QOI byte array.
public func encodeQoi(_ pixels: PixelContainer) -> [UInt8] {
    let w = Int(pixels.width)
    let h = Int(pixels.height)
    let totalPixels = w * h

    // ── Header ────────────────────────────────────────────────────────────

    // Pre-allocate with a rough upper bound; we'll append as we go.
    var out = [UInt8]()
    out.reserveCapacity(14 + totalPixels * 5 + 8)

    // Magic: "qoif"
    out.append(0x71)  // 'q'
    out.append(0x6F)  // 'o'
    out.append(0x69)  // 'i'
    out.append(0x66)  // 'f'

    // Width (big-endian uint32, bytes 4..7)
    out.append(UInt8((UInt32(w) >> 24) & 0xFF))
    out.append(UInt8((UInt32(w) >> 16) & 0xFF))
    out.append(UInt8((UInt32(w) >> 8)  & 0xFF))
    out.append(UInt8(UInt32(w) & 0xFF))

    // Height (big-endian uint32, bytes 8..11)
    out.append(UInt8((UInt32(h) >> 24) & 0xFF))
    out.append(UInt8((UInt32(h) >> 16) & 0xFF))
    out.append(UInt8((UInt32(h) >> 8)  & 0xFF))
    out.append(UInt8(UInt32(h) & 0xFF))

    // Channels: 4 = RGBA (we always output RGBA even though BMP/PPM don't have alpha).
    out.append(4)

    // Colorspace: 0 = sRGB with linear alpha.
    out.append(0)

    // ── Encoder state ─────────────────────────────────────────────────────

    // Previous pixel, initialised to opaque black per the QOI spec.
    var prev = Pixel(r: 0, g: 0, b: 0, a: 255)

    // 64-entry seen-pixels hash table, all transparent black initially.
    var seen = [Pixel](repeating: Pixel.zero, count: 64)

    // Current run length (pixels equal to prev).
    var run = 0

    // ── Helper: flush a run as one or more QOI_OP_RUN chunks ──────────────
    //
    // QOI_OP_RUN encodes a run of identical pixels.
    // The run length is biased by -1 and stored in 6 bits (0..61 → runs 1..62).
    // The tag is 0b_1100_0000 = 0xC0.
    //
    // We split runs longer than 62 into multiple chunks, because 6 bits can
    // hold at most 62 (not 63 or 64, which are reserved for RGB/RGBA tags).

    func flushRun() {
        while run > 0 {
            let chunk = min(run, 62)     // up to 62 identical pixels per chunk
            out.append(0xC0 | UInt8(chunk - 1))  // tag 11 | (run-1)
            run -= chunk
        }
    }

    // ── Main encoding loop ────────────────────────────────────────────────

    for i in 0..<totalPixels {
        let srcOff = i * 4
        let px = Pixel(
            r: pixels.data[srcOff],
            g: pixels.data[srcOff + 1],
            b: pixels.data[srcOff + 2],
            a: pixels.data[srcOff + 3]
        )

        if px == prev {
            // ── QOI_OP_RUN ────────────────────────────────────────────────
            // Current pixel is identical to the previous pixel — extend the run.
            run += 1
            if run == 62 {
                // Max run length reached; flush immediately to avoid overflow.
                flushRun()
            }
        } else {
            // Different from previous: flush any pending run first.
            flushRun()

            let idx = qoiHash(px)

            if seen[idx] == px {
                // ── QOI_OP_INDEX ──────────────────────────────────────────
                // Pixel is already in the seen table. Emit its 6-bit index.
                // Tag: 0b_00xx_xxxx → top 2 bits 00, bottom 6 bits = index.
                out.append(UInt8(idx))  // 0b00_xxxxxx (top bits already 0)

            } else {
                // Compute channel differences from previous pixel.
                // Use Int to preserve sign; range is -255..255.
                let dr = Int(px.r) - Int(prev.r)
                let dg = Int(px.g) - Int(prev.g)
                let db = Int(px.b) - Int(prev.b)
                let da = Int(px.a) - Int(prev.a)

                if da == 0 {
                    // Alpha unchanged — candidate for DIFF or LUMA.

                    if dr >= -2 && dr <= 1 && dg >= -2 && dg <= 1 && db >= -2 && db <= 1 {
                        // ── QOI_OP_DIFF ───────────────────────────────────
                        // Small signed differences (-2..1) fit in 2 bits each
                        // when biased by +2 (maps -2→0, -1→1, 0→2, 1→3).
                        // Tag: 0b_01xx_xxxx; 6-bit payload: dr+2, dg+2, db+2.
                        //
                        // Bit layout:
                        //   [01 | dr+2(2) | dg+2(2) | db+2(2)]
                        let byte = 0x40
                            | ((dr + 2) << 4)   // dr+2 in bits 5..4
                            | ((dg + 2) << 2)   // dg+2 in bits 3..2
                            |  (db + 2)         // db+2 in bits 1..0
                        out.append(UInt8(byte))

                    } else {
                        // Luma candidates: encode dg in 6 bits, dr and db relative to dg.
                        let drDg = dr - dg   // red   relative to green
                        let dbDg = db - dg   // blue  relative to green

                        if dg >= -32 && dg <= 31 && drDg >= -8 && drDg <= 7 && dbDg >= -8 && dbDg <= 7 {
                            // ── QOI_OP_LUMA ───────────────────────────────
                            // Encodes a larger signed difference using green as
                            // the luminance reference. Red and blue are encoded
                            // relative to green, exploiting inter-channel correlation.
                            //
                            // Byte 1: [10 | (dg+32)(6)]
                            //   Top 2 bits tag (10), bottom 6 bits = dg biased by +32.
                            //   Bias +32 maps dg ∈ [-32..31] → [0..63] (fits in 6 bits).
                            //
                            // Byte 2: [(drDg+8)(4) | (dbDg+8)(4)]
                            //   High nibble = dr-dg biased by +8 → [0..15] (fits in 4 bits).
                            //   Low  nibble = db-dg biased by +8 → [0..15].
                            //   Bias +8 maps [-8..7] → [0..15].
                            out.append(UInt8(0x80 | (dg + 32)))
                            out.append(UInt8(((drDg + 8) << 4) | (dbDg + 8)))

                        } else {
                            // ── QOI_OP_RGB ────────────────────────────────
                            // None of the compact encodings fit; emit all three
                            // channels verbatim. Alpha is unchanged so we don't emit it.
                            out.append(0xFE)  // RGB tag
                            out.append(px.r)
                            out.append(px.g)
                            out.append(px.b)
                        }
                    }
                } else {
                    // ── QOI_OP_RGBA ───────────────────────────────────────
                    // Alpha has changed; emit all four channels verbatim.
                    out.append(0xFF)  // RGBA tag
                    out.append(px.r)
                    out.append(px.g)
                    out.append(px.b)
                    out.append(px.a)
                }

            }
        }

        // Update the seen table for every pixel (including RUN pixels).
        // The spec requires: seen[hash(r,g,b,a)] = current_pixel after each pixel.
        seen[qoiHash(px)] = px

        prev = px
    }

    // Flush any remaining run at the end of the pixel stream.
    flushRun()

    // ── End marker ────────────────────────────────────────────────────────

    out.append(contentsOf: qoiEndMarker)  // [0,0,0,0,0,0,0,1]

    return out
}

// ============================================================================
// MARK: - Decode
// ============================================================================

/// Decode a QOI byte array into a `PixelContainer`.
///
/// Decoding algorithm:
///   1. Validate magic bytes ("qoif") and header size.
///   2. Read width, height, channels from the header.
///   3. Initialise decoder state: prev pixel, seen table, run counter.
///   4. Read chunks until all width × height pixels are decoded.
///   5. Verify the 8-byte end marker.
///
/// - Parameter bytes: Raw bytes from a QOI file.
/// - Returns: The decoded RGBA8 pixel buffer.
/// - Throws: `ImageCodecQOIError` if the data is malformed.
public func decodeQoi(_ bytes: [UInt8]) throws -> PixelContainer {

    // ── Validate and read header ──────────────────────────────────────────

    guard bytes.count >= 14 else {
        throw ImageCodecQOIError.truncatedHeader
    }

    // Magic: "qoif" = [0x71, 0x6F, 0x69, 0x66]
    guard bytes[0] == 0x71, bytes[1] == 0x6F,
          bytes[2] == 0x69, bytes[3] == 0x66 else {
        throw ImageCodecQOIError.invalidMagic
    }

    let w = Int(readBE32(bytes, at: 4))
    let h = Int(readBE32(bytes, at: 8))

    guard w > 0, h > 0 else {
        throw ImageCodecQOIError.invalidDimensions
    }

    let channels = bytes[12]
    guard channels == 3 || channels == 4 else {
        throw ImageCodecQOIError.unsupportedChannels
    }

    // ── Decoder state ─────────────────────────────────────────────────────

    var prev = Pixel(r: 0, g: 0, b: 0, a: 255)  // initial previous pixel
    var seen = [Pixel](repeating: Pixel.zero, count: 64)

    var container = PixelContainer(width: UInt32(w), height: UInt32(h))
    let totalPixels = w * h

    var pos = 14       // current read cursor (skip 14-byte header)
    var pixelIndex = 0 // how many pixels we have decoded so far
    var run = 0        // remaining pixels in current run

    while pixelIndex < totalPixels {
        var px = prev  // default: if we're in a run, pixel = prev

        if run > 0 {
            // ── RUN continuation ─────────────────────────────────────────
            // We're in the middle of a run: use prev and decrement counter.
            run -= 1
        } else {
            // ── Read next chunk ───────────────────────────────────────────
            guard pos < bytes.count else {
                throw ImageCodecQOIError.truncatedData
            }
            let b0 = bytes[pos]
            pos += 1

            if b0 == 0xFE {
                // ── QOI_OP_RGB ────────────────────────────────────────────
                // Tag 0xFE followed by 3 bytes R, G, B. Alpha = prev.a.
                guard pos + 2 < bytes.count else { throw ImageCodecQOIError.truncatedData }
                px.r = bytes[pos];     pos += 1
                px.g = bytes[pos];     pos += 1
                px.b = bytes[pos];     pos += 1
                // px.a stays as prev.a (alpha unchanged)

            } else if b0 == 0xFF {
                // ── QOI_OP_RGBA ───────────────────────────────────────────
                // Tag 0xFF followed by 4 bytes R, G, B, A.
                guard pos + 3 < bytes.count else { throw ImageCodecQOIError.truncatedData }
                px.r = bytes[pos];     pos += 1
                px.g = bytes[pos];     pos += 1
                px.b = bytes[pos];     pos += 1
                px.a = bytes[pos];     pos += 1

            } else {
                // Discriminate on the top 2 bits of b0.
                let tag2 = b0 >> 6  // top 2 bits

                switch tag2 {

                case 0b00:
                    // ── QOI_OP_INDEX ──────────────────────────────────────
                    // Tag 0b00 followed by 6-bit index into the seen table.
                    // The full byte is [00 | index(6)].
                    let index = Int(b0 & 0x3F)  // mask off top 2 bits
                    px = seen[index]

                case 0b01:
                    // ── QOI_OP_DIFF ───────────────────────────────────────
                    // Tag 0b01, then 6-bit payload: (dr+2)(2)(dg+2)(2)(db+2)(2).
                    // Decode by extracting 2-bit fields and subtracting bias.
                    //
                    //   Bit layout of b0:
                    //     7  6  5  4  3  2  1  0
                    //     0  1  dr dr dg dg db db
                    //
                    //   Extract:
                    //     dr = ((b0 >> 4) & 0x3) - 2   → {-2, -1, 0, +1}
                    //     dg = ((b0 >> 2) & 0x3) - 2
                    //     db = ( b0       & 0x3) - 2
                    let dr = Int((b0 >> 4) & 0x3) - 2
                    let dg = Int((b0 >> 2) & 0x3) - 2
                    let db = Int( b0       & 0x3) - 2
                    // Add differences back, wrapping mod 256 via & 0xFF.
                    px.r = UInt8((Int(prev.r) + dr) & 0xFF)
                    px.g = UInt8((Int(prev.g) + dg) & 0xFF)
                    px.b = UInt8((Int(prev.b) + db) & 0xFF)
                    // Alpha unchanged.

                case 0b10:
                    // ── QOI_OP_LUMA ───────────────────────────────────────
                    // Two-byte encoding. Byte 1 (b0) encodes dg; byte 2 encodes
                    // dr-dg and db-dg.
                    //
                    //   Byte 1: [10 | (dg+32)(6)]
                    //     dg = (b0 & 0x3F) - 32   → {-32..+31}
                    //
                    //   Byte 2: [(drDg+8)(4) | (dbDg+8)(4)]
                    //     drDg = (b1 >> 4) - 8    → {-8..+7}
                    //     dbDg = (b1 & 0x0F) - 8
                    //
                    //   Reconstruct:
                    //     dr = dg + drDg
                    //     db = dg + dbDg
                    guard pos < bytes.count else { throw ImageCodecQOIError.truncatedData }
                    let b1 = bytes[pos]; pos += 1

                    let dg  = Int(b0 & 0x3F) - 32
                    let drDg = Int(b1 >> 4)  - 8
                    let dbDg = Int(b1 & 0x0F) - 8
                    let dr = dg + drDg
                    let db = dg + dbDg

                    px.r = UInt8((Int(prev.r) + dr) & 0xFF)
                    px.g = UInt8((Int(prev.g) + dg) & 0xFF)
                    px.b = UInt8((Int(prev.b) + db) & 0xFF)
                    // Alpha unchanged.

                default:  // 0b11
                    // ── QOI_OP_RUN ────────────────────────────────────────
                    // Tag 0b11, 6-bit payload = run length - 1.
                    //   run = (b0 & 0x3F) + 1
                    //
                    // The run does NOT include the current pixel being processed —
                    // we emit `prev` for the current pixel and then `run-1` more times.
                    // So we set `run` to (run_length - 1) for the remaining pixels.
                    //
                    // px stays as prev (current pixel = start of run).
                    run = Int(b0 & 0x3F)  // biased by -1; we use prev as this pixel
                    // and `run` more pixels to follow
                }
            }

        }

        // Update the seen table for every pixel (including run continuations).
        // The QOI spec says: "After a pixel has been decoded, it is stored in
        // the seen-pixels array at seen[hash(r, g, b, a)]."
        seen[qoiHash(px)] = px

        // Write the decoded pixel to the container.
        let dstOff = pixelIndex * 4
        container.data[dstOff]     = px.r
        container.data[dstOff + 1] = px.g
        container.data[dstOff + 2] = px.b
        container.data[dstOff + 3] = px.a

        prev = px
        pixelIndex += 1
    }

    // ── Verify end marker ─────────────────────────────────────────────────

    // The spec requires 8 bytes [0,0,0,0,0,0,0,1] after the last chunk.
    guard pos + 8 <= bytes.count else {
        throw ImageCodecQOIError.missingEndMarker
    }
    let endSlice = Array(bytes[pos..<(pos + 8)])
    guard endSlice == qoiEndMarker else {
        throw ImageCodecQOIError.missingEndMarker
    }

    return container
}

// ============================================================================
// MARK: - QoiCodec (ImageCodec conformance)
// ============================================================================

/// A QOI image encoder/decoder that conforms to `ImageCodec`.
///
/// Wraps `encodeQoi` and `decodeQoi` in the standard codec interface.
///
/// ## Usage
///
/// ```swift
/// let codec = QoiCodec()
/// let bytes  = codec.encode(myPixels)          // PixelContainer → [UInt8]
/// let pixels = try codec.decode(someBytes)     // [UInt8] → PixelContainer
/// print(codec.mimeType)                        // "image/qoi"
/// ```
public struct QoiCodec: ImageCodec {
    public init() {}

    /// IANA MIME type for QOI files.
    public var mimeType: String { "image/qoi" }

    /// Encode a pixel container as a QOI byte array.
    public func encode(_ pixels: PixelContainer) -> [UInt8] {
        encodeQoi(pixels)
    }

    /// Decode a QOI byte array into a pixel container.
    ///
    /// - Throws: `ImageCodecQOIError` if the input is invalid.
    public func decode(_ bytes: [UInt8]) throws -> PixelContainer {
        try decodeQoi(bytes)
    }
}
