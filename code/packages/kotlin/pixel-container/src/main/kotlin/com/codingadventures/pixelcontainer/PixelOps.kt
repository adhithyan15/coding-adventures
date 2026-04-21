package com.codingadventures.pixelcontainer

/**
 * Static helpers for reading and writing individual pixels inside a
 * [PixelContainer].
 *
 * These primitives are kept on a singleton rather than as methods on
 * [PixelContainer] so the data class stays a pure, dumb memory holder —
 * exactly the sort of container many different processing libraries can
 * share without dragging behaviour in with it.
 *
 * ## Out-of-bounds policy
 *
 * - [pixelAt]    — returns `(0, 0, 0, 0)` (transparent black) for OOB reads.
 * - [setPixel]   — silently ignores OOB writes.
 * - [fillPixels] — never goes OOB (operates over the entire buffer).
 *
 * Returning zeros instead of throwing makes the read path safe to use inside
 * geometric transforms where the sample coordinate can legitimately fall off
 * the image and the caller then chooses a border policy explicitly.
 */
object PixelOps {

    /**
     * Read the RGBA quadruple at `(x, y)`.
     *
     * Each returned component is already widened to an unsigned 0..255 `Int`
     * using `byte.toInt() and 0xFF`, so callers do not need to remember the
     * sign-extension dance.
     *
     * Out-of-bounds coordinates return `intArrayOf(0, 0, 0, 0)`.
     */
    fun pixelAt(c: PixelContainer, x: Int, y: Int): IntArray {
        if (x < 0 || x >= c.width || y < 0 || y >= c.height)
            return intArrayOf(0, 0, 0, 0)
        val i = (y * c.width + x) * 4
        return intArrayOf(
            c.data[i].toInt() and 0xFF,
            c.data[i + 1].toInt() and 0xFF,
            c.data[i + 2].toInt() and 0xFF,
            c.data[i + 3].toInt() and 0xFF
        )
    }

    /**
     * Write the RGBA quadruple at `(x, y)`. Each component is truncated to
     * its low 8 bits (`and 0xFF`) before storing, so callers may pass raw
     * un-clamped values from arithmetic pipelines. Out-of-bounds coordinates
     * are silently ignored (a no-op).
     */
    fun setPixel(c: PixelContainer, x: Int, y: Int, r: Int, g: Int, b: Int, a: Int) {
        if (x < 0 || x >= c.width || y < 0 || y >= c.height) return
        val i = (y * c.width + x) * 4
        c.data[i]     = (r and 0xFF).toByte()
        c.data[i + 1] = (g and 0xFF).toByte()
        c.data[i + 2] = (b and 0xFF).toByte()
        c.data[i + 3] = (a and 0xFF).toByte()
    }

    /**
     * Overwrite every pixel with the given RGBA colour. The loop walks
     * packed bytes rather than `(x, y)` pairs for cache-friendliness: a
     * single forward sweep over `data` hits each cache line exactly once.
     */
    fun fillPixels(c: PixelContainer, r: Int, g: Int, b: Int, a: Int) {
        val rb = (r and 0xFF).toByte()
        val gb = (g and 0xFF).toByte()
        val bb = (b and 0xFF).toByte()
        val ab = (a and 0xFF).toByte()
        var i = 0
        while (i < c.data.size) {
            c.data[i]     = rb
            c.data[i + 1] = gb
            c.data[i + 2] = bb
            c.data[i + 3] = ab
            i += 4
        }
    }
}
