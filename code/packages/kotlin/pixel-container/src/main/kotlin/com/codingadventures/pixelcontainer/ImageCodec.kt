package com.codingadventures.pixelcontainer

/**
 * Abstract codec interface for encoding and decoding [PixelContainer]s.
 *
 * Every concrete codec in the stack (PNG, JPEG, BMP, GIF, ...) implements
 * this interface. Because the universe of raster formats is vast, this
 * interface is deliberately tiny — just "turn bytes into pixels" and back —
 * and every format-specific parameter (quality, filter choice, palette, ...)
 * lives on the concrete implementation rather than here.
 *
 * ## Conventions
 *
 * - [mimeType] uses the canonical IANA media type (e.g. `"image/png"`).
 * - [encode] must be the inverse of [decode] for any buffer the codec
 *   produced, modulo lossy compression (JPEG etc.).
 * - Implementations should raise a format-specific exception on malformed
 *   input rather than silently returning a partial image.
 */
interface ImageCodec {
    /** Canonical IANA media type, e.g. `"image/png"`. */
    val mimeType: String

    /** Encode a [PixelContainer] into the codec's wire format. */
    fun encode(container: PixelContainer): ByteArray

    /** Decode a wire-format byte array into a [PixelContainer]. */
    fun decode(data: ByteArray): PixelContainer
}
