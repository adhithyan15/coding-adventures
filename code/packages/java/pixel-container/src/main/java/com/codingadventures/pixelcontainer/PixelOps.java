package com.codingadventures.pixelcontainer;

/**
 * Static helpers for reading and writing pixels in a {@link PixelContainer}.
 *
 * <p>These utilities deliberately do <em>not</em> live as instance methods on
 * PixelContainer because the container is meant to be a zero-behaviour data
 * carrier. Behaviour (including ops that tempt reflection, subclassing, or
 * mocking) lives in free functions that can be composed and tested in
 * isolation.</p>
 *
 * <h2>Out-of-bounds policy</h2>
 * <p>Reads return transparent black ({@code {0,0,0,0}}) for any
 * {@code (x, y)} outside the image. Writes silently no-op. This "clamp to
 * empty" convention is chosen over throwing exceptions because point and
 * geometric operations frequently sample just outside the canvas, and
 * throwing would force every caller to pre-check.</p>
 */
public final class PixelOps {
    private PixelOps() {}

    /**
     * Factory: create a new PixelContainer filled with transparent
     * black (0, 0, 0, 0).
     *
     * <p>Provided for callers that prefer a factory style over
     * {@code new PixelContainer(w, h)}.</p>
     *
     * @param width  image width in pixels
     * @param height image height in pixels
     * @return a fresh container backed by a zeroed byte array
     */
    public static PixelContainer create(int width, int height) {
        return new PixelContainer(width, height);
    }

    /**
     * Read the RGBA components at {@code (x, y)}.
     *
     * <p>Returns a freshly allocated 4-int array {@code {R, G, B, A}} with
     * each channel in {@code 0..255}. Out-of-bounds reads return
     * {@code {0, 0, 0, 0}} — transparent black — rather than throwing.</p>
     *
     * <p>Why int[] and not a Pixel record? Hotspot inlines the array access
     * extremely well, and a 4-int array lets the caller mutate/reuse values
     * without boxing.</p>
     *
     * @return RGBA quad; {@code {0,0,0,0}} if {@code (x,y)} is outside
     */
    public static int[] pixelAt(PixelContainer c, int x, int y) {
        if (x < 0 || x >= c.width || y < 0 || y >= c.height) {
            return new int[]{0, 0, 0, 0};
        }
        int i = (y * c.width + x) * 4;
        return new int[]{
            c.data[i]     & 0xFF,
            c.data[i + 1] & 0xFF,
            c.data[i + 2] & 0xFF,
            c.data[i + 3] & 0xFF
        };
    }

    /**
     * Write an RGBA pixel at {@code (x, y)}. No-op for out-of-bounds
     * coordinates, matching the "clamp to empty" read policy.
     *
     * <p>Each channel value is masked to 8 bits before storing, so callers
     * may pass values up to 32 bits without corrupting neighbouring
     * channels.</p>
     */
    public static void setPixel(PixelContainer c, int x, int y, int r, int g, int b, int a) {
        if (x < 0 || x >= c.width || y < 0 || y >= c.height) return;
        int i = (y * c.width + x) * 4;
        c.data[i]     = (byte) (r & 0xFF);
        c.data[i + 1] = (byte) (g & 0xFF);
        c.data[i + 2] = (byte) (b & 0xFF);
        c.data[i + 3] = (byte) (a & 0xFF);
    }

    /**
     * Fill every pixel of the container with the given RGBA colour.
     *
     * <p>Implemented as a tight loop over interleaved bytes rather than
     * delegating to {@link #setPixel} per pixel — avoids per-call bounds
     * checks and {@code y*width+x} arithmetic that the fill case doesn't
     * need.</p>
     */
    public static void fillPixels(PixelContainer c, int r, int g, int b, int a) {
        byte rb = (byte) (r & 0xFF);
        byte gb = (byte) (g & 0xFF);
        byte bb = (byte) (b & 0xFF);
        byte ab = (byte) (a & 0xFF);
        for (int i = 0; i < c.data.length; i += 4) {
            c.data[i]     = rb;
            c.data[i + 1] = gb;
            c.data[i + 2] = bb;
            c.data[i + 3] = ab;
        }
    }
}
