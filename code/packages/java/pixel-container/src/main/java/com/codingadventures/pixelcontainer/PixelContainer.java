package com.codingadventures.pixelcontainer;

/**
 * IC00 — Universal RGBA8 Pixel Buffer.
 *
 * <p>PixelContainer is the zero-dependency foundation for the
 * coding-adventures image processing stack. Every image codec and
 * processing stage depends only on this class, never on a specific file
 * format or library.</p>
 *
 * <h2>Why RGBA8, always?</h2>
 * <p>The image stack deals with many formats — PNG, JPEG, BMP, GIF, WebP —
 * each with its own quirks (palettes, grayscale, 16-bit channels, pre- vs
 * post-multiplied alpha). Forcing every producer to decode down to a single
 * canonical format means consumers never care where the pixels came from.
 * RGBA8 was chosen because:</p>
 * <ul>
 *   <li>It's the overwhelmingly common screen representation, so rendering
 *       is trivial.</li>
 *   <li>It handles transparency, which the simpler RGB8 does not.</li>
 *   <li>8 bits per channel is universally supported by every codec.</li>
 * </ul>
 *
 * <h2>Layout</h2>
 * <p>Pixels are stored row-major, top-left origin, RGBA interleaved:</p>
 * <pre>
 *   offset = (y * width + x) * 4
 *   data[offset + 0] = R
 *   data[offset + 1] = G
 *   data[offset + 2] = B
 *   data[offset + 3] = A
 * </pre>
 *
 * <h2>Why public fields?</h2>
 * <p>The container is a dumb data carrier — tight inner loops in point
 * operations and geometric transforms benefit from direct field access
 * (e.g. Hotspot can hoist {@code c.width} out of loop bodies easily). The
 * trade-off is immutability is not enforced at the JVM level; we rely on
 * discipline: codecs create containers once, processing stages produce
 * new containers rather than mutating inputs.</p>
 */
public final class PixelContainer {
    /** Image width in pixels. Never negative. */
    public final int width;

    /** Image height in pixels. Never negative. */
    public final int height;

    /**
     * Flat RGBA8 byte buffer, length {@code width * height * 4}.
     *
     * <p>Bytes are <em>unsigned</em> in meaning, even though Java's
     * {@code byte} is signed. Readers must mask with {@code & 0xFF} to get
     * the intended 0..255 value.</p>
     */
    public final byte[] data;

    /**
     * Create a new container filled with transparent black (0, 0, 0, 0).
     *
     * <p>Java initialises byte arrays to zero, which happens to be our
     * desired "empty" value — no extra work needed.</p>
     *
     * @param width  image width in pixels
     * @param height image height in pixels
     */
    public PixelContainer(int width, int height) {
        if (width < 0 || height < 0)
            throw new IllegalArgumentException("Dimensions must be non-negative");
        long size = (long) width * height * 4;
        if (size > Integer.MAX_VALUE)
            throw new IllegalArgumentException(
                "Image too large: " + width + "×" + height + " exceeds 32-bit index range");
        this.width = width;
        this.height = height;
        this.data = new byte[(int) size];
    }

    /**
     * Create a container wrapping an existing byte buffer. Used by codecs
     * that decode directly into a caller-supplied array, avoiding a copy.
     *
     * <p>The caller is responsible for ensuring {@code data.length ==
     * width * height * 4}.</p>
     *
     * @param width  image width in pixels
     * @param height image height in pixels
     * @param data   backing RGBA8 buffer (not copied, not validated)
     */
    public PixelContainer(int width, int height, byte[] data) {
        this.width = width;
        this.height = height;
        this.data = data;
    }
}
