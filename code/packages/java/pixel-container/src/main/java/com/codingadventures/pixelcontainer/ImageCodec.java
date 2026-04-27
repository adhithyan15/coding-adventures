package com.codingadventures.pixelcontainer;

/**
 * Abstract codec interface for encoding and decoding {@link PixelContainer}s.
 *
 * <p>Every image format (PNG, JPEG, BMP, WebP, …) that wants to interoperate
 * with the coding-adventures image stack implements this three-method
 * contract. Nothing in the core pixel container depends on any concrete
 * codec, so adding a new format never forces a recompile of consumers.</p>
 *
 * <h2>Contract</h2>
 * <ul>
 *   <li>{@link #encode} serializes a PixelContainer to the format's byte
 *       layout (e.g. a full PNG file).</li>
 *   <li>{@link #decode} parses those bytes back to RGBA8.</li>
 *   <li>{@link #mimeType} returns the canonical MIME string, used by
 *       HTTP layers and registries that route by type.</li>
 * </ul>
 *
 * <p>Implementations should be stateless and thread-safe where possible,
 * so callers can share a single instance across threads.</p>
 */
public interface ImageCodec {
    /** Canonical MIME type for this format, e.g. {@code "image/png"}. */
    String mimeType();

    /** Serialize the container into the format's on-disk byte layout. */
    byte[] encode(PixelContainer container);

    /** Parse bytes back into a fresh RGBA8 PixelContainer. */
    PixelContainer decode(byte[] data);
}
