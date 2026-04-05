// PixelContainer.swift
// Part of coding-adventures — an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// MARK: - IC00: Fixed RGBA8 Pixel Buffer
// ============================================================================
//
// PixelContainer is the shared data type for the image codec stack. Every
// image format (BMP, PPM, QOI, PNG, …) encodes FROM or decodes TO a
// PixelContainer. This design decouples the pixel data from the file format,
// so a pipeline like:
//
//   decode BMP → PixelContainer → encode QOI
//
// requires no format-specific conversion code.
//
// ============================================================================
// Memory Layout
// ============================================================================
//
// Pixels are stored in row-major order in a flat [UInt8] array.
// Each pixel occupies exactly 4 bytes: Red, Green, Blue, Alpha.
//
//   Byte offset for pixel at column x, row y:
//     offset = (y * width + x) * 4
//
//   Layout of one pixel at offset `o`:
//     data[o + 0]  = Red   channel (0..255)
//     data[o + 1]  = Green channel (0..255)
//     data[o + 2]  = Blue  channel (0..255)
//     data[o + 3]  = Alpha channel (0..255; 0 = transparent, 255 = opaque)
//
// Row-major means that pixels on the same row are adjacent in memory.
// This is the natural order for most image formats and matches C/C++ arrays.
//
// ============================================================================
// Why RGBA8?
// ============================================================================
//
// RGBA8 means: 4 channels (Red, Green, Blue, Alpha) each stored as 8-bit
// unsigned integer (UInt8, range 0..255). This gives:
//   - 256 intensity levels per channel (human perception needs ~200)
//   - 4 bytes per pixel — a predictable, cache-friendly stride
//   - Native alpha compositing support (transparency/opacity)
//   - The format used internally by most GPUs and image processing libraries
//
// Total buffer size = width × height × 4 bytes.
//
// ============================================================================
// ImageCodec Protocol
// ============================================================================
//
// Any type that can encode/decode a specific image file format should conform
// to ImageCodec. This gives codec implementors a consistent interface and
// lets callers handle codecs polymorphically.
//
// Example:
//
//   let codecs: [any ImageCodec] = [BmpCodec(), PpmCodec(), QoiCodec()]
//   for codec in codecs {
//       let bytes = codec.encode(myPixels)
//       print("\(codec.mimeType): \(bytes.count) bytes")
//   }
//
// ============================================================================

// ============================================================================
// MARK: - PixelContainer
// ============================================================================

/// A fixed-size RGBA8 pixel buffer.
///
/// Stores `width × height` pixels in row-major order, 4 bytes per pixel
/// (R, G, B, A). The buffer is allocated once on initialization and stays the
/// same size for the container's lifetime. Mutation happens through the
/// `setPixel` and `fillPixels` free functions.
///
/// ## Memory Formula
///
///   totalBytes  = width × height × 4
///   offset(x,y) = (y × width + x) × 4
///
/// ## Example
///
/// ```swift
/// var img = PixelContainer(width: 8, height: 8)
/// setPixel(&img, x: 3, y: 3, r: 255, g: 0, b: 0, a: 255)  // red dot
/// let (r, g, b, a) = pixelAt(img, x: 3, y: 3)              // (255, 0, 0, 255)
/// ```
public struct PixelContainer {
    /// The width of the image in pixels.
    public let width: UInt32
    /// The height of the image in pixels.
    public let height: UInt32
    /// The raw pixel bytes. Length = `width × height × 4`.
    /// Indexed as `data[(y * width + x) * 4 + channel]` where channel is
    /// 0=R, 1=G, 2=B, 3=A.
    public var data: [UInt8]

    /// Create a new pixel container filled with transparent black (all zeros).
    ///
    /// All bytes are initialised to 0:
    ///   - R = 0, G = 0, B = 0  → black
    ///   - A = 0                → fully transparent
    ///
    /// - Parameters:
    ///   - width:  Image width in pixels.
    ///   - height: Image height in pixels.
    public init(width: UInt32, height: UInt32) {
        self.width = width
        self.height = height
        // Allocate exactly width × height × 4 bytes, all zero.
        self.data = [UInt8](repeating: 0, count: Int(width) * Int(height) * 4)
    }
}

// ============================================================================
// MARK: - ImageCodec Protocol
// ============================================================================

/// Interface every image format encoder/decoder must implement.
///
/// Conforming types convert between a binary byte stream (the on-disk/wire
/// representation of a specific format) and a `PixelContainer` (the in-memory
/// RGBA8 representation).
///
/// ## Encoding
///
///   `encode(_:) → [UInt8]`
///
/// Takes a `PixelContainer` and produces a byte array in the target format.
/// This operation always succeeds (every valid pixel buffer can be encoded).
///
/// ## Decoding
///
///   `decode(_:) throws → PixelContainer`
///
/// Takes a raw byte array and produces a `PixelContainer`. Throws if the
/// bytes are malformed (wrong magic bytes, truncated header, unsupported
/// sub-format, etc.).
///
/// ## MIME Type
///
/// Each codec advertises its MIME type so callers can set `Content-Type`
/// headers or choose the right codec dynamically.
public protocol ImageCodec {
    /// The IANA MIME type for this image format.
    /// Examples: `"image/bmp"`, `"image/x-portable-pixmap"`, `"image/qoi"`.
    var mimeType: String { get }

    /// Encode a pixel container into the format's byte representation.
    ///
    /// - Parameter pixels: The source image.
    /// - Returns: A byte array in the format's binary layout.
    func encode(_ pixels: PixelContainer) -> [UInt8]

    /// Decode a byte array into a pixel container.
    ///
    /// - Parameter bytes: Raw bytes from a file or network stream.
    /// - Returns: The decoded RGBA8 pixel buffer.
    /// - Throws: `PixelContainerError` or a format-specific error type if the
    ///   input is malformed.
    func decode(_ bytes: [UInt8]) throws -> PixelContainer
}

// ============================================================================
// MARK: - Error Types
// ============================================================================

/// Errors produced by PixelContainer operations.
///
/// Individual codecs (BMP, PPM, QOI) define their own error enums for
/// format-specific failure modes. This enum covers only generic pixel-buffer
/// errors that might arise in codec-agnostic code.
public enum PixelContainerError: Error, Equatable {
    /// The requested dimensions are invalid (e.g., zero width or height, or
    /// width × height × 4 would overflow Int).
    case invalidDimensions
    /// The pixel data array has the wrong length (not width × height × 4).
    case invalidData
}

// ============================================================================
// MARK: - Pixel Access Helpers
// ============================================================================
//
// Free functions rather than methods keep PixelContainer a pure value type
// with no hidden methods — consistent with the C-style educational theme.
// The `inout` variants signal mutation clearly at every call site.

/// Return the pixel at column `x`, row `y` as `(r, g, b, a)`.
///
/// Computes byte offset = `(y × width + x) × 4`, then reads 4 consecutive
/// bytes. Returns `(0, 0, 0, 0)` (transparent black) if `(x, y)` is out of
/// bounds instead of trapping — this makes codec loops that read slightly past
/// the edge safe.
///
/// - Parameters:
///   - c: The pixel container to read from.
///   - x: Column index (0-based, left-to-right).
///   - y: Row index (0-based, top-to-bottom).
/// - Returns: `(red, green, blue, alpha)` tuple, each component 0..255.
public func pixelAt(_ c: PixelContainer, x: UInt32, y: UInt32) -> (UInt8, UInt8, UInt8, UInt8) {
    // Bounds check: return transparent black for out-of-bounds access.
    guard x < c.width, y < c.height else {
        return (0, 0, 0, 0)
    }
    // Compute the byte offset. Each pixel = 4 bytes; rows are width pixels wide.
    let offset = Int(y * c.width + x) * 4
    return (c.data[offset], c.data[offset + 1], c.data[offset + 2], c.data[offset + 3])
}

/// Set the pixel at column `x`, row `y` to the given RGBA components.
///
/// This is a no-op if `(x, y)` is out of bounds — the container is unchanged.
/// This matches the bounds behaviour of `pixelAt` and prevents traps in loops
/// that might compute coordinates from user-supplied dimensions.
///
/// - Parameters:
///   - c: The pixel container to mutate (passed by reference).
///   - x: Column index (0-based).
///   - y: Row index (0-based).
///   - r: Red component (0..255).
///   - g: Green component (0..255).
///   - b: Blue component (0..255).
///   - a: Alpha component (0=transparent, 255=opaque).
public func setPixel(_ c: inout PixelContainer, x: UInt32, y: UInt32,
                     r: UInt8, g: UInt8, b: UInt8, a: UInt8) {
    guard x < c.width, y < c.height else { return }
    let offset = Int(y * c.width + x) * 4
    c.data[offset]     = r
    c.data[offset + 1] = g
    c.data[offset + 2] = b
    c.data[offset + 3] = a
}

/// Fill every pixel in the container with a single RGBA value.
///
/// Overwrites all `width × height` pixels with `(r, g, b, a)`. Useful for
/// creating solid-colour backgrounds before drawing content, or for clearing
/// a buffer for reuse.
///
/// Example — opaque red canvas:
/// ```swift
/// var img = PixelContainer(width: 64, height: 64)
/// fillPixels(&img, r: 255, g: 0, b: 0, a: 255)
/// ```
///
/// - Parameters:
///   - c: The pixel container to mutate.
///   - r: Red component (0..255).
///   - g: Green component (0..255).
///   - b: Blue component (0..255).
///   - a: Alpha component (0..255).
public func fillPixels(_ c: inout PixelContainer, r: UInt8, g: UInt8, b: UInt8, a: UInt8) {
    // Write every 4-byte pixel group in one linear pass over the buffer.
    // This is more cache-friendly than calling setPixel in a nested loop
    // because it accesses memory sequentially.
    let count = c.data.count  // width * height * 4
    var i = 0
    while i < count {
        c.data[i]     = r
        c.data[i + 1] = g
        c.data[i + 2] = b
        c.data[i + 3] = a
        i += 4  // advance to the next pixel (4 bytes)
    }
}
