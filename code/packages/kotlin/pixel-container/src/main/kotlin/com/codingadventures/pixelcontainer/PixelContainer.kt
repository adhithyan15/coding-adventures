package com.codingadventures.pixelcontainer

/**
 * IC00 — Universal RGBA8 Pixel Buffer.
 *
 * `PixelContainer` is the zero-dependency foundation for the coding-adventures
 * image-processing stack. Every image codec (PNG, JPEG, BMP, ...) and every
 * processing stage (point-ops, geometric transforms, filters, ...) depends
 * only on this class. The deliberate narrowness of the interface is the whole
 * point: once a codec decodes into a `PixelContainer`, anything downstream can
 * consume it, and once a processing stage produces one, any encoder can write
 * it out. This is the "narrow waist" of the stack.
 *
 * ## Memory layout
 *
 * Pixels are stored **row-major**, top-left origin, with channels interleaved
 * in RGBA order:
 *
 * ```
 * offset(x, y) = (y * width + x) * 4
 *
 * data[offset + 0] = R   // red,   0..255
 * data[offset + 1] = G   // green, 0..255
 * data[offset + 2] = B   // blue,  0..255
 * data[offset + 3] = A   // alpha, 0..255 (0 = transparent, 255 = opaque)
 * ```
 *
 * ## Fixed format
 *
 * The container is deliberately restricted:
 *
 * - exactly 4 channels (R, G, B, A)
 * - exactly 8 bits per channel (unsigned, stored as signed `Byte` with the
 *   reader using `.toInt() and 0xFF` to widen to unsigned)
 * - exactly top-left origin, row-major order
 * - no padding / stride — rows are packed
 *
 * Restricting the format means callers never need to dispatch on channel
 * order, byte order, or depth. More exotic formats (HDR, grey-only, planar)
 * are a job for a different container at a different layer.
 *
 * ## Size invariant
 *
 * The backing [data] array must always have length `width * height * 4`. When
 * an explicit array is supplied, it is the caller's responsibility to satisfy
 * that invariant. The default constructor allocates a correctly sized, fully
 * transparent (all zero) buffer.
 *
 * @property width  image width in pixels (must be non-negative)
 * @property height image height in pixels (must be non-negative)
 * @property data   packed RGBA bytes, length `width * height * 4`
 */
class PixelContainer(
    val width: Int,
    val height: Int,
    val data: ByteArray = run {
        require(width >= 0 && height >= 0) { "Dimensions must be non-negative" }
        val size = width.toLong() * height.toLong() * 4L
        require(size <= Int.MAX_VALUE) {
            "Image too large: ${width}×${height} exceeds 32-bit index range"
        }
        ByteArray(size.toInt())
    }
)
