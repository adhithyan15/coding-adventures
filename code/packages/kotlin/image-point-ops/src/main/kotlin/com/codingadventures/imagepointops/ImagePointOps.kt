package com.codingadventures.imagepointops

import com.codingadventures.pixelcontainer.PixelContainer
import com.codingadventures.pixelcontainer.PixelOps
import kotlin.math.abs
import kotlin.math.pow
import kotlin.math.roundToInt

/**
 * IMG03 — Per-pixel point operations on [PixelContainer].
 *
 * Every method in this object transforms each pixel independently, using only
 * that pixel's own value — no neighbourhood, no frequency domain, no geometric
 * resampling. That is exactly what distinguishes "point ops" from the rest of
 * the imaging pipeline, and it's what makes them trivially parallelisable in
 * principle: any pixel can be computed before, after, or simultaneously with
 * any other pixel.
 *
 * ## Two domains
 *
 * Point ops fall cleanly into two groups, and this split is load-bearing for
 * correctness — not an academic nicety:
 *
 * - **u8 domain** — invert, threshold, posterize, channel shuffling,
 *   brightness. Operate directly on the 0..255 byte values. Fast, exact,
 *   and appropriate because the operation has no photometric meaning: you
 *   are drawing on the encoded pixel values as a data structure.
 *
 * - **Linear-light domain** — gamma, exposure, greyscale, sepia, colour
 *   matrices, saturation, hue rotation. Operate on the actual light
 *   intensity that the display emitted. Pixels as they live in a PNG or
 *   JPEG are sRGB-encoded (roughly gamma 2.2), so you first *decode* to
 *   linear light, do the maths, then *re-encode* on the way out. Skipping
 *   this step produces subtly wrong greyscale, muddy blurs, and weirdly
 *   dim tinting — all the classic "web graphics look bad" bugs.
 *
 * ## sRGB ↔ linear
 *
 * ```
 * decode: c8  -> c = c8 / 255
 *         c <= 0.04045  ?  c / 12.92  :  ((c + 0.055) / 1.055)^2.4
 *
 * encode: l   -> l <= 0.0031308  ?  l * 12.92  :  1.055 * l^(1/2.4) - 0.055
 *         out = round(encoded * 255)
 * ```
 *
 * The sRGB-to-linear direction is evaluated once at class init into a
 * 256-entry lookup table, because there are only 256 possible 8-bit inputs;
 * computing [pow] per pixel per channel is a wasted billion floating-point
 * ops for a 1024x1024 image.
 */
object ImagePointOps {

    // ----------------------------------------------------------------------
    // sRGB ↔ linear helpers
    //
    // SRGB_TO_LINEAR is a 256-entry LUT — all possible inputs to `decode` are
    // bytes, so we precompute every answer once at class-load time.
    // `encode` stays a per-call computation because its input is a continuous
    // [0, 1] double and would need quantisation to be table-driven; we
    // accept a pow() per encode rather than approximate.
    // ----------------------------------------------------------------------

    private val SRGB_TO_LINEAR: DoubleArray = DoubleArray(256) { i ->
        val c = i / 255.0
        if (c <= 0.04045) c / 12.92 else ((c + 0.055) / 1.055).pow(2.4)
    }

    private fun decode(b: Int): Double = SRGB_TO_LINEAR[b and 0xFF]

    private fun encode(linear: Double): Int {
        val c = linear.coerceIn(0.0, 1.0)
        val srgb = if (c <= 0.0031308) c * 12.92 else 1.055 * c.pow(1.0 / 2.4) - 0.055
        return (srgb * 255.0).roundToInt()
    }

    // ----------------------------------------------------------------------
    // mapPixels — shared core. Every point op is "read pixel, compute new
    // pixel, write pixel" — so we factor that walk into a single helper and
    // let each op supply only the per-pixel lambda. Output is always a new
    // PixelContainer; we never mutate the input.
    // ----------------------------------------------------------------------

    private inline fun mapPixels(
        src: PixelContainer,
        f: (Int, Int, Int, Int) -> IntArray
    ): PixelContainer {
        val out = PixelContainer(src.width, src.height)
        for (y in 0 until src.height)
            for (x in 0 until src.width) {
                val p = PixelOps.pixelAt(src, x, y)
                val q = f(p[0], p[1], p[2], p[3])
                PixelOps.setPixel(out, x, y, q[0], q[1], q[2], q[3])
            }
        return out
    }

    // ======================================================================
    // u8-DOMAIN OPERATIONS
    // ======================================================================

    /**
     * Photographic negative — flip R, G, B about 127.5. Alpha passes
     * through unchanged: inverting alpha would turn transparent pixels
     * opaque, which is rarely what the user means.
     */
    fun invert(src: PixelContainer): PixelContainer =
        mapPixels(src) { r, g, b, a -> intArrayOf(255 - r, 255 - g, 255 - b, a) }

    /**
     * Binary threshold on the unweighted mean of R, G, B. Pixels with mean
     * strictly greater than [t] become white; the rest become black. Alpha
     * is preserved. The threshold is compared with `>`, so `t = 255` always
     * produces all-black output and `t = -1` always produces all-white.
     */
    fun threshold(src: PixelContainer, t: Int): PixelContainer =
        mapPixels(src) { r, g, b, a ->
            val avg = (r + g + b) / 3
            val v = if (avg > t) 255 else 0
            intArrayOf(v, v, v, a)
        }

    /**
     * Binary threshold on Rec.709 luminance (Y = 0.2126R + 0.7152G + 0.0722B),
     * computed in the u8 domain for speed. This is a conscious approximation:
     * a photometrically rigorous luminance threshold would decode sRGB first,
     * but threshold output is already a two-value stair-step — the
     * difference is visually indistinguishable.
     */
    fun thresholdLuminance(src: PixelContainer, t: Int): PixelContainer =
        mapPixels(src) { r, g, b, a ->
            val y = 0.2126 * r + 0.7152 * g + 0.0722 * b
            val v = if (y > t) 255 else 0
            intArrayOf(v, v, v, a)
        }

    /**
     * Posterize to [levels] discrete values per channel. Maps each 0..255
     * input to the nearest of `levels` evenly spaced output values in
     * 0..255. `levels = 2` gives pure black/white per channel (eight
     * possible colours); `levels = 256` is a no-op.
     *
     * ```
     * step = 255 / (levels - 1)
     * out  = round(round(in / step) * step)
     * ```
     */
    fun posterize(src: PixelContainer, levels: Int): PixelContainer {
        require(levels >= 2) { "levels must be >= 2" }
        val step = 255.0 / (levels - 1)
        fun q(v: Int): Int = ((v / step).roundToInt() * step).roundToInt().coerceIn(0, 255)
        return mapPixels(src) { r, g, b, a -> intArrayOf(q(r), q(g), q(b), a) }
    }

    /** Channel shuffle: swap R and B. Useful when bridging to BGR pipelines. */
    fun swapRgbBgr(src: PixelContainer): PixelContainer =
        mapPixels(src) { r, g, b, a -> intArrayOf(b, g, r, a) }

    /**
     * Replicate a single channel into R, G, B. The chosen channel index is
     * `0 = R`, `1 = G`, `2 = B`, `3 = A`. Alpha in the output is always
     * fully opaque (255), because a greyed-out visualisation of alpha only
     * makes sense when the RGB carries the same info.
     */
    fun extractChannel(src: PixelContainer, ch: Int): PixelContainer {
        require(ch in 0..3) { "channel must be 0..3" }
        return mapPixels(src) { r, g, b, a ->
            val v = intArrayOf(r, g, b, a)[ch]
            intArrayOf(v, v, v, 255)
        }
    }

    /** Additive brightness in u8 domain — clamped per channel. */
    fun brightness(src: PixelContainer, delta: Int): PixelContainer =
        mapPixels(src) { r, g, b, a ->
            intArrayOf(
                (r + delta).coerceIn(0, 255),
                (g + delta).coerceIn(0, 255),
                (b + delta).coerceIn(0, 255),
                a
            )
        }

    /**
     * Photoshop-style contrast: pivots each channel about 128. `factor = 1.0`
     * is a no-op, larger factors increase contrast, smaller decreases, and
     * `0.0` flattens the whole image to mid-grey.
     *
     * ```
     * out = clamp( (in - 128) * factor + 128 )
     * ```
     */
    fun contrast(src: PixelContainer, factor: Double): PixelContainer =
        mapPixels(src) { r, g, b, a ->
            fun c(v: Int) = ((v - 128) * factor + 128).roundToInt().coerceIn(0, 255)
            intArrayOf(c(r), c(g), c(b), a)
        }

    // ======================================================================
    // LINEAR-LIGHT DOMAIN OPERATIONS
    // ======================================================================

    /**
     * Gamma correction in linear light: decode sRGB → raise to the power
     * `g` → re-encode. `g = 1.0` is a no-op, `g < 1` brightens midtones,
     * `g > 1` darkens them.
     */
    fun gamma(src: PixelContainer, g: Double): PixelContainer =
        mapPixels(src) { r, gg, b, a ->
            intArrayOf(
                encode(decode(r).pow(g)),
                encode(decode(gg).pow(g)),
                encode(decode(b).pow(g)),
                a
            )
        }

    /**
     * Exposure in f-stops. Each stop doubles linear light, so the scale
     * factor is `2^stops`. Negative stops darken. Saturates at 1.0 before
     * re-encoding.
     */
    fun exposure(src: PixelContainer, stops: Double): PixelContainer {
        val k = 2.0.pow(stops)
        return mapPixels(src) { r, g, b, a ->
            intArrayOf(
                encode(decode(r) * k),
                encode(decode(g) * k),
                encode(decode(b) * k),
                a
            )
        }
    }

    /** Luminance weightings for [greyscale]. */
    enum class GreyscaleMethod { REC709, BT601, AVERAGE }

    /**
     * Desaturate to grey in linear light. The three methods differ only in
     * their R, G, B weights:
     *
     * | method  | R       | G       | B       | notes                       |
     * |---------|---------|---------|---------|-----------------------------|
     * | REC709  | 0.2126  | 0.7152  | 0.0722  | modern HDTV / sRGB          |
     * | BT601   | 0.299   | 0.587   | 0.114   | legacy NTSC / JPEG Y'CbCr   |
     * | AVERAGE | 1/3     | 1/3     | 1/3     | naive, perceptually wrong   |
     */
    fun greyscale(src: PixelContainer, method: GreyscaleMethod): PixelContainer {
        val (wr, wg, wb) = when (method) {
            GreyscaleMethod.REC709  -> Triple(0.2126, 0.7152, 0.0722)
            GreyscaleMethod.BT601   -> Triple(0.299,  0.587,  0.114)
            GreyscaleMethod.AVERAGE -> Triple(1.0/3, 1.0/3, 1.0/3)
        }
        return mapPixels(src) { r, g, b, a ->
            val y = decode(r) * wr + decode(g) * wg + decode(b) * wb
            val v = encode(y)
            intArrayOf(v, v, v, a)
        }
    }

    /**
     * Classic sepia matrix, applied in linear light:
     * ```
     * R' = 0.393 R + 0.769 G + 0.189 B
     * G' = 0.349 R + 0.686 G + 0.168 B
     * B' = 0.272 R + 0.534 G + 0.131 B
     * ```
     */
    fun sepia(src: PixelContainer): PixelContainer =
        colourMatrix(
            src,
            arrayOf(
                doubleArrayOf(0.393, 0.769, 0.189),
                doubleArrayOf(0.349, 0.686, 0.168),
                doubleArrayOf(0.272, 0.534, 0.131)
            )
        )

    /**
     * Apply a 3×3 colour matrix to each pixel in linear light. Rows are
     * "coefficients that produce R', G', B'" respectively; columns are
     * "R, G, B contributions". Alpha is preserved.
     */
    fun colourMatrix(src: PixelContainer, m: Array<DoubleArray>): PixelContainer {
        require(m.size == 3 && m.all { it.size == 3 }) { "matrix must be 3x3" }
        return mapPixels(src) { r, g, b, a ->
            val lr = decode(r); val lg = decode(g); val lb = decode(b)
            val nr = m[0][0]*lr + m[0][1]*lg + m[0][2]*lb
            val ng = m[1][0]*lr + m[1][1]*lg + m[1][2]*lb
            val nb = m[2][0]*lr + m[2][1]*lg + m[2][2]*lb
            intArrayOf(encode(nr), encode(ng), encode(nb), a)
        }
    }

    /**
     * Saturation adjustment. `factor = 0` collapses every pixel to its own
     * luminance (perfect greyscale), `1` is identity, and values > 1 push
     * each channel farther from grey.
     *
     * Implementation: compute Y in linear light, then lerp each channel
     * between Y and itself by [factor].
     */
    fun saturate(src: PixelContainer, factor: Double): PixelContainer =
        mapPixels(src) { r, g, b, a ->
            val lr = decode(r); val lg = decode(g); val lb = decode(b)
            val y = 0.2126 * lr + 0.7152 * lg + 0.0722 * lb
            intArrayOf(
                encode(y + (lr - y) * factor),
                encode(y + (lg - y) * factor),
                encode(y + (lb - y) * factor),
                a
            )
        }

    /**
     * Rotate the hue of every pixel by [degrees] while preserving saturation
     * and value. Done via round-trip RGB → HSV → shift H → HSV → RGB, all
     * in linear light.
     */
    fun hueRotate(src: PixelContainer, degrees: Double): PixelContainer =
        mapPixels(src) { r, g, b, a ->
            val (h, s, v) = rgbToHsv(decode(r), decode(g), decode(b))
            var nh = h + degrees
            nh = ((nh % 360.0) + 360.0) % 360.0
            val (nr, ng, nb) = hsvToRgb(nh, s, v)
            intArrayOf(encode(nr), encode(ng), encode(nb), a)
        }

    /** Decode every pixel's RGB from sRGB to linear, stored as 0..255 bytes. */
    fun srgbToLinearImage(src: PixelContainer): PixelContainer =
        mapPixels(src) { r, g, b, a ->
            intArrayOf(
                (decode(r) * 255.0).roundToInt().coerceIn(0, 255),
                (decode(g) * 255.0).roundToInt().coerceIn(0, 255),
                (decode(b) * 255.0).roundToInt().coerceIn(0, 255),
                a
            )
        }

    /** Inverse of [srgbToLinearImage]: re-encode linear 0..255 back to sRGB. */
    fun linearToSrgbImage(src: PixelContainer): PixelContainer =
        mapPixels(src) { r, g, b, a ->
            intArrayOf(
                encode(r / 255.0),
                encode(g / 255.0),
                encode(b / 255.0),
                a
            )
        }

    // ======================================================================
    // LUT helpers
    // ======================================================================

    /**
     * Apply independent 1-D u8 look-up tables to R, G, B. Alpha untouched.
     * Each LUT must have exactly 256 entries.
     */
    fun applyLut1dU8(
        src: PixelContainer,
        lutR: ByteArray,
        lutG: ByteArray,
        lutB: ByteArray
    ): PixelContainer {
        require(lutR.size == 256 && lutG.size == 256 && lutB.size == 256) {
            "LUTs must have 256 entries"
        }
        return mapPixels(src) { r, g, b, a ->
            intArrayOf(
                lutR[r].toInt() and 0xFF,
                lutG[g].toInt() and 0xFF,
                lutB[b].toInt() and 0xFF,
                a
            )
        }
    }

    /**
     * Build a 256-entry u8 LUT from a function `f: [0, 1] -> [0, 1]`. The
     * input index is mapped to `[0, 1]` by dividing by 255; the output is
     * clamped and rounded back to u8.
     */
    fun buildLut1dU8(f: (Double) -> Double): ByteArray =
        ByteArray(256) { i ->
            val y = f(i / 255.0).coerceIn(0.0, 1.0)
            (y * 255.0).roundToInt().toByte()
        }

    /** Gamma LUT: `x^g` over [0, 1]. */
    fun buildGammaLut(g: Double): ByteArray = buildLut1dU8 { x -> x.pow(g) }

    // ======================================================================
    // HSV helpers
    // ======================================================================

    /**
     * Convert linear-light RGB in `[0, 1]` to HSV with H in degrees.
     *
     * Edge cases follow the standard formulation: greys return `H = 0`,
     * zero-value pixels return `S = 0`.
     */
    private fun rgbToHsv(r: Double, g: Double, b: Double): Triple<Double, Double, Double> {
        val cmax = maxOf(r, g, b)
        val cmin = minOf(r, g, b)
        val delta = cmax - cmin
        var h = when {
            delta < 1e-6 -> 0.0
            cmax == r -> 60.0 * (((g - b) / delta) % 6.0)
            cmax == g -> 60.0 * ((b - r) / delta + 2.0)
            else      -> 60.0 * ((r - g) / delta + 4.0)
        }
        if (h < 0) h += 360.0
        val s = if (cmax < 1e-6) 0.0 else delta / cmax
        return Triple(h, s, cmax)
    }

    /** Inverse of [rgbToHsv]. */
    private fun hsvToRgb(h: Double, s: Double, v: Double): Triple<Double, Double, Double> {
        val c = v * s
        val x = c * (1.0 - abs((h / 60.0) % 2.0 - 1.0))
        val m = v - c
        val sector = ((h / 60.0).toInt() % 6 + 6) % 6
        val (r1, g1, b1) = when (sector) {
            0 -> Triple(c, x, 0.0)
            1 -> Triple(x, c, 0.0)
            2 -> Triple(0.0, c, x)
            3 -> Triple(0.0, x, c)
            4 -> Triple(x, 0.0, c)
            else -> Triple(c, 0.0, x)
        }
        return Triple(
            (r1 + m).coerceIn(0.0, 1.0),
            (g1 + m).coerceIn(0.0, 1.0),
            (b1 + m).coerceIn(0.0, 1.0)
        )
    }
}
