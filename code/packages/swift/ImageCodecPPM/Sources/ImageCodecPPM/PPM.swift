// PPM.swift
// Part of coding-adventures — IC02: PPM image encoder/decoder.
//
// ============================================================================
// MARK: - IC02: PPM (Portable Pixmap) Image Codec
// ============================================================================
//
// PPM is part of the Netpbm family of plain-text image formats. Its design
// goal is simplicity: a human-readable header followed by binary or ASCII
// pixel data. This makes PPM trivial to generate, easy to inspect with a
// text editor, and widely supported by Unix image tools (convert, display, etc.)
//
// ============================================================================
// PPM File Format (P6 Binary Variant)
// ============================================================================
//
// We implement the P6 (binary) variant of PPM, which is by far the most
// common in practice. A P6 file looks like:
//
//   P6\n
//   # optional comment lines starting with #\n
//   <width> <height>\n
//   <maxval>\n
//   <binary pixel data>
//
// Where <binary pixel data> is width × height × 3 bytes: R, G, B for each
// pixel in row-major, left-to-right, top-to-bottom order. maxval is the
// maximum channel value; we always write and accept 255.
//
// ============================================================================
// Header Parsing Strategy
// ============================================================================
//
// We use a cursor-based approach: a single integer index `pos` advances
// through the byte array as we consume header tokens. This avoids allocation
// of intermediate strings and is easy to follow step-by-step.
//
//   Tokens we consume:
//     1. "P6" — magic bytes
//     2. whitespace / comment lines (lines starting with '#')
//     3. width integer
//     4. whitespace
//     5. height integer
//     6. whitespace
//     7. maxval integer
//     8. exactly one whitespace byte (separates header from binary data)
//     9. binary pixel data (width × height × 3 bytes)
//
// ============================================================================
// PPM vs BMP vs QOI
// ============================================================================
//
//   Format  Header type   Pixel order  Alpha?  Compression?  Size
//   ──────  ────────────  ───────────  ──────  ────────────  ──────────────
//   PPM     ASCII text    RGB          No      No (raw)      Large (raw RGB)
//   BMP     Binary fixed  BGR          No      No (raw)      Large (raw BGR)
//   QOI     Binary fixed  RGBA         Yes     Yes (lossless)Compact
//
// ============================================================================

import PixelContainer

// ============================================================================
// MARK: - Error Type
// ============================================================================

/// Errors produced by the PPM codec.
public enum ImageCodecPPMError: Error, Equatable {
    /// The file does not start with "P6".
    case invalidMagic
    /// The header is missing a required field or is malformed.
    case malformedHeader
    /// Width or height is zero.
    case invalidDimensions
    /// The maxval field is not 255 (we only support 8-bit channels).
    case unsupportedMaxval
    /// The pixel data section is shorter than width × height × 3 bytes.
    case truncatedPixelData
}

// ============================================================================
// MARK: - Encode
// ============================================================================

/// Encode a `PixelContainer` as a P6 binary PPM byte array.
///
/// The output is a complete, valid PPM file. Alpha is dropped (PPM P6
/// stores only R, G, B per pixel).
///
/// Encoding steps:
///   1. Build the ASCII header: "P6\n<width> <height>\n255\n".
///   2. Convert the header string to UTF-8 bytes.
///   3. Append raw RGB pixel data (no padding, no compression).
///
/// - Parameter pixels: The source RGBA8 pixel buffer.
/// - Returns: A byte array representing a complete, valid P6 PPM file.
public func encodePpm(_ pixels: PixelContainer) -> [UInt8] {
    let w = Int(pixels.width)
    let h = Int(pixels.height)

    // ── ASCII header ──────────────────────────────────────────────────────
    //
    // The header is human-readable ASCII:
    //   "P6\n"           — magic number identifying binary PPM
    //   "<w> <h>\n"      — dimensions separated by a space
    //   "255\n"          — maximum channel value (8 bits per channel)
    //
    // The newline after "255" is the required single-whitespace separator
    // between the header and the binary pixel data.
    let header = "P6\n\(w) \(h)\n255\n"

    // Convert the header to raw bytes (PPM uses ASCII/UTF-8 for the header).
    var out = [UInt8](header.utf8)

    // ── Binary pixel data ─────────────────────────────────────────────────
    //
    // width × height pixels, each 3 bytes: R, G, B.
    // Row-major order, top-to-bottom, left-to-right.
    // PPM has NO row padding (unlike BMP) and NO alpha.
    out.reserveCapacity(out.count + w * h * 3)

    for i in 0..<(w * h) {
        let srcOff = i * 4  // RGBA offset in PixelContainer
        out.append(pixels.data[srcOff])     // R
        out.append(pixels.data[srcOff + 1]) // G
        out.append(pixels.data[srcOff + 2]) // B
        // Alpha (srcOff + 3) is dropped — PPM has no alpha channel.
    }

    return out
}

// ============================================================================
// MARK: - Decode
// ============================================================================

/// Decode a P6 binary PPM byte array into a `PixelContainer`.
///
/// Decoding steps:
///   1. Check magic bytes "P6".
///   2. Skip whitespace and comment lines (lines starting with '#').
///   3. Parse width, height, and maxval integers from the ASCII header.
///   4. Consume exactly one whitespace byte after maxval.
///   5. Read width × height × 3 bytes of binary pixel data.
///   6. Convert RGB → RGBA (synthesise alpha = 255).
///
/// - Parameter bytes: Raw bytes from a PPM file.
/// - Returns: The decoded RGBA8 pixel buffer.
/// - Throws: `ImageCodecPPMError` if the data is malformed.
public func decodePpm(_ bytes: [UInt8]) throws -> PixelContainer {

    // ── Cursor-based parser ───────────────────────────────────────────────
    //
    // `pos` is our cursor: an index into `bytes` that moves forward as we
    // consume each part of the header. All helper closures below capture
    // `pos` by reference and advance it.

    var pos = 0

    // ── Helper: skip whitespace and comment lines ─────────────────────────
    //
    // PPM allows comment lines (starting with '#') to appear anywhere in the
    // header. A comment extends to the end of its line. We also skip all
    // non-comment whitespace (space, tab, \r, \n) between tokens.

    func skipWhitespaceAndComments() {
        while pos < bytes.count {
            let b = bytes[pos]
            if b == 0x23 {
                // '#' — comment: skip everything up to and including the newline.
                while pos < bytes.count && bytes[pos] != 0x0A {
                    pos += 1
                }
                // Skip the newline itself.
                if pos < bytes.count { pos += 1 }
            } else if b == 0x20 || b == 0x09 || b == 0x0A || b == 0x0D {
                // Space, tab, LF, CR — skip.
                pos += 1
            } else {
                break  // Non-whitespace, non-comment byte: stop skipping.
            }
        }
    }

    // ── Helper: parse an ASCII decimal integer ────────────────────────────
    //
    // Reads digit characters until a non-digit is encountered.
    // Returns nil if no digits are found at the current position.

    func parseInt() -> Int? {
        var result = 0
        var found = false
        while pos < bytes.count {
            let b = bytes[pos]
            guard b >= 0x30 && b <= 0x39 else { break }  // '0'..'9'
            result = result * 10 + Int(b - 0x30)
            found = true
            pos += 1
        }
        return found ? result : nil
    }

    // ── 1. Parse magic number ─────────────────────────────────────────────

    guard bytes.count >= 2, bytes[0] == 0x50, bytes[1] == 0x36 else {
        // Expected "P6" (0x50 = 'P', 0x36 = '6')
        throw ImageCodecPPMError.invalidMagic
    }
    pos = 2  // Consumed "P6"

    // ── 2. Parse width ────────────────────────────────────────────────────

    skipWhitespaceAndComments()
    guard let width = parseInt() else {
        throw ImageCodecPPMError.malformedHeader
    }

    // ── 3. Parse height ───────────────────────────────────────────────────

    skipWhitespaceAndComments()
    guard let height = parseInt() else {
        throw ImageCodecPPMError.malformedHeader
    }

    guard width > 0, height > 0 else {
        throw ImageCodecPPMError.invalidDimensions
    }

    // ── 4. Parse maxval ───────────────────────────────────────────────────

    skipWhitespaceAndComments()
    guard let maxval = parseInt() else {
        throw ImageCodecPPMError.malformedHeader
    }

    // We only support maxval = 255 (8-bit channels).
    // maxval = 65535 would indicate 16-bit PPM (P6 with 2 bytes/channel).
    guard maxval == 255 else {
        throw ImageCodecPPMError.unsupportedMaxval
    }

    // ── 5. Consume exactly one whitespace byte ────────────────────────────
    //
    // The PPM spec requires exactly one whitespace byte between the end of
    // the maxval and the start of the binary pixel data.

    guard pos < bytes.count,
          (bytes[pos] == 0x20 || bytes[pos] == 0x09 ||
           bytes[pos] == 0x0A || bytes[pos] == 0x0D) else {
        throw ImageCodecPPMError.malformedHeader
    }
    pos += 1  // Consume the separator byte.

    // ── 6. Validate that enough pixel bytes remain ────────────────────────

    let pixelBytes = width * height * 3  // 3 bytes per pixel (RGB, no alpha)
    guard bytes.count - pos >= pixelBytes else {
        throw ImageCodecPPMError.truncatedPixelData
    }

    // ── 7. Read pixel data ────────────────────────────────────────────────

    var container = PixelContainer(width: UInt32(width), height: UInt32(height))

    for i in 0..<(width * height) {
        let r = bytes[pos + i * 3]       // Red
        let g = bytes[pos + i * 3 + 1]  // Green
        let b = bytes[pos + i * 3 + 2]  // Blue
        // Synthesise alpha = 255 (fully opaque): PPM has no alpha channel.
        let dstOff = i * 4
        container.data[dstOff]     = r
        container.data[dstOff + 1] = g
        container.data[dstOff + 2] = b
        container.data[dstOff + 3] = 255
    }

    return container
}

// ============================================================================
// MARK: - PpmCodec (ImageCodec conformance)
// ============================================================================

/// A PPM image encoder/decoder that conforms to `ImageCodec`.
///
/// Wraps `encodePpm` and `decodePpm` in the standard codec interface.
///
/// ## Usage
///
/// ```swift
/// let codec = PpmCodec()
/// let bytes  = codec.encode(myPixels)          // PixelContainer → [UInt8]
/// let pixels = try codec.decode(someBytes)     // [UInt8] → PixelContainer
/// print(codec.mimeType)                        // "image/x-portable-pixmap"
/// ```
public struct PpmCodec: ImageCodec {
    public init() {}

    /// IANA MIME type for PPM files.
    public var mimeType: String { "image/x-portable-pixmap" }

    /// Encode a pixel container as a PPM byte array.
    public func encode(_ pixels: PixelContainer) -> [UInt8] {
        encodePpm(pixels)
    }

    /// Decode a PPM byte array into a pixel container.
    ///
    /// - Throws: `ImageCodecPPMError` if the input is invalid.
    public func decode(_ bytes: [UInt8]) throws -> PixelContainer {
        try decodePpm(bytes)
    }
}
