// BMP.swift
// Part of coding-adventures — IC01: BMP image encoder/decoder.
//
// ============================================================================
// MARK: - IC01: BMP Image Codec
// ============================================================================
//
// The BMP (Bitmap) format is one of the oldest and simplest image formats on
// Windows. It stores uncompressed pixel data with a small fixed-size header.
// This makes it an excellent first format to learn binary file I/O.
//
// ============================================================================
// BMP File Structure (the subset we implement)
// ============================================================================
//
// We implement the 24-bit BGR BMP format (BITMAPINFOHEADER, no palette):
//
//   Offset  Size  Field
//   ──────  ────  ─────────────────────────────────────────────────────────
//    0      2     Signature: "BM" (0x42, 0x4D)
//    2      4     File size in bytes (little-endian uint32)
//    6      2     Reserved 1 (0x0000)
//    8      2     Reserved 2 (0x0000)
//   10      4     Pixel data offset (= 54 = 14 + 40)
//   ─────────────── DIB header (BITMAPINFOHEADER) ──────────────────────────
//   14      4     DIB header size (= 40)
//   18      4     Width in pixels (little-endian int32, positive)
//   22      4     Height in pixels (little-endian int32, NEGATIVE for top-down)
//   26      2     Color planes (= 1)
//   28      2     Bits per pixel (= 24)
//   30      4     Compression (= 0 = BI_RGB, no compression)
//   34      4     Raw pixel data size (can be 0 for BI_RGB)
//   38      4     X pixels per metre (= 2835 ≈ 72 DPI)
//   42      4     Y pixels per metre (= 2835 ≈ 72 DPI)
//   46      4     Colors in table (= 0)
//   50      4     Important colors (= 0)
//   ─────────────── Pixel data ─────────────────────────────────────────────
//   54     ...    Row-by-row, bottom-to-top (unless height is negative)
//
// ============================================================================
// Row Stride and Padding
// ============================================================================
//
// Each row in a BMP file must be padded to a multiple of 4 bytes.
//
//   stride = ceil(width × 3 / 4) × 4
//          = ((width × 3 + 3) / 4) × 4  (integer arithmetic)
//
// For a 10-pixel-wide image: 10 × 3 = 30 bytes, padded to 32 bytes (2 bytes
// padding per row).
//
// ============================================================================
// BGR vs RGBA
// ============================================================================
//
// BMP stores pixels in BGR order (Blue, Green, Red), NOT RGB. This is a
// historical artefact of Windows storing colours in DWORD little-endian order.
//
// When encoding:  PixelContainer[R, G, B, A] → BMP[B, G, R]  (drop alpha)
// When decoding:  BMP[B, G, R]              → PixelContainer[R, G, B, 255]
//
// We discard the alpha channel on encode (BMP 24-bit has no alpha).
// We synthesize alpha = 255 (fully opaque) on decode.
//
// ============================================================================
// Endianness
// ============================================================================
//
// All multi-byte integers in BMP are little-endian. On Apple Silicon and
// x86-64 (also little-endian), we could use unsafe casts, but we use manual
// byte assembly for clarity and portability.
//
//   Little-endian layout of 0x12345678:
//     byte 0 = 0x78 (least significant byte first)
//     byte 1 = 0x56
//     byte 2 = 0x34
//     byte 3 = 0x12 (most significant byte last)
//
// ============================================================================

import PixelContainer

// ============================================================================
// MARK: - Error Type
// ============================================================================

/// Errors produced by the BMP codec.
public enum ImageCodecBMPError: Error, Equatable {
    /// The byte array is too short to contain a complete BMP header.
    case truncatedHeader
    /// The first two bytes are not "BM" (0x42, 0x4D).
    case invalidSignature
    /// The DIB header size field is not 40 (we only support BITMAPINFOHEADER).
    case unsupportedDibHeader
    /// The bits-per-pixel field is not 24 (we only support 24-bit BMP).
    case unsupportedBitDepth
    /// The compression field is not 0 (we only support BI_RGB, uncompressed).
    case unsupportedCompression
    /// Width or height is zero or would cause an arithmetic overflow.
    case invalidDimensions
    /// The file is shorter than the header claims it should be.
    case truncatedPixelData
}

// ============================================================================
// MARK: - Little-Endian Helpers
// ============================================================================
//
// BMP uses little-endian byte order for all multi-byte integers.
// These helpers assemble or disassemble bytes at specific positions in a
// buffer, making the header read/write code easy to follow.

/// Write a `UInt16` in little-endian byte order into `buf` starting at `offset`.
///
/// Little-endian means the least significant byte comes first:
///   val = 0xABCD  →  buf[offset] = 0xCD, buf[offset+1] = 0xAB
///
/// - Parameters:
///   - val:    The 16-bit value to write.
///   - buf:    The byte array to write into (passed by reference).
///   - offset: The position in `buf` to write the first (LSB) byte.
func writeLE16(_ val: UInt16, into buf: inout [UInt8], at offset: Int) {
    buf[offset]     = UInt8(val & 0xFF)          // low byte
    buf[offset + 1] = UInt8((val >> 8) & 0xFF)   // high byte
}

/// Write a `UInt32` in little-endian byte order into `buf` starting at `offset`.
///
/// val = 0x12345678  →  buf[offset+0] = 0x78, [+1] = 0x56, [+2] = 0x34, [+3] = 0x12
func writeLE32(_ val: UInt32, into buf: inout [UInt8], at offset: Int) {
    buf[offset]     = UInt8(val & 0xFF)           // byte 0 (LSB)
    buf[offset + 1] = UInt8((val >> 8)  & 0xFF)  // byte 1
    buf[offset + 2] = UInt8((val >> 16) & 0xFF)  // byte 2
    buf[offset + 3] = UInt8((val >> 24) & 0xFF)  // byte 3 (MSB)
}

/// Write an `Int32` in little-endian byte order into `buf` starting at `offset`.
///
/// We use this for the height field, which is negative in top-down BMPs.
/// Negative numbers are represented in two's complement, so we reinterpret
/// the bit pattern as UInt32 before writing the bytes.
func writeLE32Signed(_ val: Int32, into buf: inout [UInt8], at offset: Int) {
    // Reinterpret the bit pattern as unsigned — same bytes, just different sign.
    writeLE32(UInt32(bitPattern: val), into: &buf, at: offset)
}

/// Read a `UInt16` in little-endian byte order from `buf` starting at `offset`.
///
/// Reconstructs the value by OR-ing the shifted bytes:
///   val = UInt16(buf[offset]) | (UInt16(buf[offset+1]) << 8)
func readLE16(_ buf: [UInt8], at offset: Int) -> UInt16 {
    let lo = UInt16(buf[offset])
    let hi = UInt16(buf[offset + 1])
    return lo | (hi << 8)
}

/// Read a `UInt32` in little-endian byte order from `buf` starting at `offset`.
func readLE32(_ buf: [UInt8], at offset: Int) -> UInt32 {
    let b0 = UInt32(buf[offset])
    let b1 = UInt32(buf[offset + 1])
    let b2 = UInt32(buf[offset + 2])
    let b3 = UInt32(buf[offset + 3])
    return b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
}

/// Read an `Int32` in little-endian byte order from `buf` starting at `offset`.
///
/// Reads as UInt32 then reinterprets the bit pattern as a signed integer.
/// Used for the height field, which is negative for top-down BMPs.
func readLE32Signed(_ buf: [UInt8], at offset: Int) -> Int32 {
    return Int32(bitPattern: readLE32(buf, at: offset))
}

// ============================================================================
// MARK: - Encode
// ============================================================================

/// Encode a `PixelContainer` into a 24-bit BGR BMP byte array.
///
/// The output is a complete, valid BMP file that can be written directly to
/// disk and opened in any image viewer.
///
/// We use a **negative height** in the DIB header (`-Int32(height)`) so that
/// rows are stored top-to-bottom in the file. Standard BMP is bottom-to-top,
/// but using a negative height avoids reversing rows and keeps the code simple.
///
/// Encoding steps:
///   1. Compute stride (each row padded to 4-byte boundary).
///   2. Compute total file size = 54 + stride × height.
///   3. Write the 14-byte file header.
///   4. Write the 40-byte DIB (BITMAPINFOHEADER).
///   5. Write pixel data row by row, converting RGBA → BGR and padding rows.
///
/// - Parameter pixels: The source RGBA8 pixel buffer.
/// - Returns: A byte array representing a complete, valid BMP file.
public func encodeBmp(_ pixels: PixelContainer) -> [UInt8] {
    let w = Int(pixels.width)
    let h = Int(pixels.height)

    // Row stride: each row of 24-bit pixels, padded up to a multiple of 4 bytes.
    //   width × 3 bytes per pixel, then round up to next multiple of 4.
    //   Formula: ((w * 3 + 3) / 4) * 4  using integer division.
    let stride = ((w * 3 + 3) / 4) * 4

    // Total file size: 54-byte header + pixel data.
    let pixelDataSize = stride * h
    let fileSize = 54 + pixelDataSize

    // Pre-allocate the output buffer, all zeros.
    var buf = [UInt8](repeating: 0, count: fileSize)

    // ── File Header (14 bytes, offsets 0..13) ─────────────────────────────

    // Signature: "BM" marks this as a BMP file.
    buf[0] = 0x42  // 'B'
    buf[1] = 0x4D  // 'M'

    // File size (uint32 LE at offset 2).
    writeLE32(UInt32(fileSize), into: &buf, at: 2)

    // Reserved fields (offsets 6 and 8) stay zero.

    // Pixel data offset: always 54 = 14 (file header) + 40 (DIB header).
    writeLE32(54, into: &buf, at: 10)

    // ── DIB Header / BITMAPINFOHEADER (40 bytes, offsets 14..53) ─────────

    // DIB header size = 40 (identifies BITMAPINFOHEADER format).
    writeLE32(40, into: &buf, at: 14)

    // Width in pixels (int32 LE, positive).
    writeLE32Signed(Int32(w), into: &buf, at: 18)

    // Height in pixels (int32 LE, NEGATIVE = top-down storage order).
    // Standard BMP stores rows bottom-to-top; negative height reverses this
    // so row 0 of our PixelContainer maps to the first row in the file.
    writeLE32Signed(-Int32(h), into: &buf, at: 22)

    // Color planes: always 1.
    writeLE16(1, into: &buf, at: 26)

    // Bits per pixel: 24 (three 8-bit channels: B, G, R).
    writeLE16(24, into: &buf, at: 28)

    // Compression: 0 = BI_RGB (no compression).
    writeLE32(0, into: &buf, at: 30)

    // Raw pixel data size: can be 0 for BI_RGB, but we fill it in anyway.
    writeLE32(UInt32(pixelDataSize), into: &buf, at: 34)

    // Horizontal resolution: 2835 pixels/metre ≈ 72 DPI.
    writeLE32(2835, into: &buf, at: 38)
    // Vertical resolution: same.
    writeLE32(2835, into: &buf, at: 42)

    // Colors in palette: 0 (no palette for 24-bit images).
    writeLE32(0, into: &buf, at: 46)
    // Important colors: 0.
    writeLE32(0, into: &buf, at: 50)

    // ── Pixel Data (offsets 54 .. fileSize-1) ────────────────────────────
    //
    // Write one row at a time. Each row is `stride` bytes:
    //   - width × 3 bytes of pixel data (BGR order)
    //   - (stride - width*3) bytes of padding (zeros, already in place)

    var rowStart = 54  // current write position in buf
    for y in 0..<h {
        var col = rowStart
        for x in 0..<w {
            // Read RGBA from the PixelContainer.
            let srcOffset = (y * w + x) * 4
            let r = pixels.data[srcOffset]
            let g = pixels.data[srcOffset + 1]
            let b = pixels.data[srcOffset + 2]
            // Alpha is discarded — 24-bit BMP has no alpha channel.

            // Write in BMP byte order: Blue, Green, Red.
            buf[col]     = b
            buf[col + 1] = g
            buf[col + 2] = r
            col += 3
        }
        // Padding bytes (already zero from pre-allocation) follow at col..rowStart+stride-1.
        rowStart += stride
    }

    return buf
}

/// Decode a 24-bit BGR BMP byte array into a `PixelContainer`.
///
/// Decoding steps:
///   1. Validate the file header (signature, minimum size).
///   2. Validate the DIB header (size=40, bpp=24, compression=0).
///   3. Read width and height (handle negative height for top-down BMPs).
///   4. Read pixel rows, converting BGR → RGBA and skipping padding bytes.
///
/// - Parameter bytes: Raw bytes from a BMP file.
/// - Returns: The decoded RGBA8 pixel buffer.
/// - Throws: `ImageCodecBMPError` if the data is malformed or unsupported.
public func decodeBmp(_ bytes: [UInt8]) throws -> PixelContainer {

    // ── Validate file header ──────────────────────────────────────────────

    // The minimum valid BMP file is 54 bytes (14 + 40 header, 0 pixels).
    guard bytes.count >= 54 else {
        throw ImageCodecBMPError.truncatedHeader
    }

    // Magic bytes: BMP files always start with "BM".
    guard bytes[0] == 0x42, bytes[1] == 0x4D else {
        throw ImageCodecBMPError.invalidSignature
    }

    // Pixel data offset (where pixel rows begin in the file).
    let pixelDataOffset = Int(readLE32(bytes, at: 10))

    // ── Validate DIB header ───────────────────────────────────────────────

    // DIB header size field at offset 14; we only support size=40 (BITMAPINFOHEADER).
    let dibSize = readLE32(bytes, at: 14)
    guard dibSize == 40 else {
        throw ImageCodecBMPError.unsupportedDibHeader
    }

    // Bits per pixel at offset 28; we only support 24-bit.
    let bpp = readLE16(bytes, at: 28)
    guard bpp == 24 else {
        throw ImageCodecBMPError.unsupportedBitDepth
    }

    // Compression at offset 30; we only support BI_RGB (= 0).
    let compression = readLE32(bytes, at: 30)
    guard compression == 0 else {
        throw ImageCodecBMPError.unsupportedCompression
    }

    // ── Read dimensions ───────────────────────────────────────────────────

    // Width is an int32 at offset 18 — always positive.
    let widthSigned = readLE32Signed(bytes, at: 18)
    // Height is an int32 at offset 22.
    //   Positive = bottom-to-top storage (standard BMP).
    //   Negative = top-to-bottom storage (less common, but valid).
    let heightSigned = readLE32Signed(bytes, at: 22)

    guard widthSigned > 0 && heightSigned != 0 else {
        throw ImageCodecBMPError.invalidDimensions
    }

    let maxDimension = 16384
    guard widthSigned <= maxDimension, abs(heightSigned) <= maxDimension else {
        throw ImageCodecBMPError.invalidDimensions
    }

    let w = Int(widthSigned)
    // Use the absolute value of height for storage; track direction separately.
    let h = Int(abs(heightSigned))
    let topDown = heightSigned < 0  // negative height → rows are top-to-bottom

    // Row stride: each row padded to a multiple of 4 bytes.
    let stride = ((w * 3 + 3) / 4) * 4

    // Validate that the file contains enough bytes for the pixel data.
    let requiredBytes = pixelDataOffset + stride * h
    guard bytes.count >= requiredBytes else {
        throw ImageCodecBMPError.truncatedPixelData
    }

    // ── Read pixel data ───────────────────────────────────────────────────

    var container = PixelContainer(width: UInt32(w), height: UInt32(h))

    for row in 0..<h {
        // Map the file row index to the PixelContainer row index.
        // Standard BMP (positive height): file row 0 = bottom row of image.
        // Top-down BMP (negative height): file row 0 = top row of image.
        let destY = topDown ? row : (h - 1 - row)

        // Start of this row's bytes in the file.
        let rowOffset = pixelDataOffset + row * stride

        for x in 0..<w {
            let srcOff = rowOffset + x * 3
            let b = bytes[srcOff]      // Blue  (first byte in BMP)
            let g = bytes[srcOff + 1]  // Green
            let r = bytes[srcOff + 2]  // Red   (last byte in BMP)
            // Write to PixelContainer as RGBA, synthesising A = 255 (fully opaque).
            let dstOff = (destY * w + x) * 4
            container.data[dstOff]     = r
            container.data[dstOff + 1] = g
            container.data[dstOff + 2] = b
            container.data[dstOff + 3] = 255  // 24-bit BMP has no alpha; assume opaque
        }
    }

    return container
}

// ============================================================================
// MARK: - BmpCodec (ImageCodec conformance)
// ============================================================================

/// A BMP image encoder/decoder that conforms to `ImageCodec`.
///
/// `BmpCodec` wraps `encodeBmp` and `decodeBmp` in the standard codec
/// interface. Use it when you need to handle BMP images polymorphically
/// alongside other codecs (PPM, QOI, …).
///
/// ## Usage
///
/// ```swift
/// let codec = BmpCodec()
/// let bytes  = codec.encode(myPixels)          // PixelContainer → [UInt8]
/// let pixels = try codec.decode(someBytes)     // [UInt8] → PixelContainer
/// print(codec.mimeType)                        // "image/bmp"
/// ```
public struct BmpCodec: ImageCodec {
    public init() {}

    /// IANA MIME type for BMP files.
    public var mimeType: String { "image/bmp" }

    /// Encode a pixel container as a BMP byte array.
    public func encode(_ pixels: PixelContainer) -> [UInt8] {
        encodeBmp(pixels)
    }

    /// Decode a BMP byte array into a pixel container.
    ///
    /// - Throws: `ImageCodecBMPError` if the input is invalid.
    public func decode(_ bytes: [UInt8]) throws -> PixelContainer {
        try decodeBmp(bytes)
    }
}
