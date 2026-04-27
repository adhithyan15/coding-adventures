package com.codingadventures.imagegeometrictransforms

import com.codingadventures.pixelcontainer.PixelContainer
import com.codingadventures.pixelcontainer.PixelOps
import kotlin.math.abs
import kotlin.math.ceil
import kotlin.math.cos
import kotlin.math.floor
import kotlin.math.pow
import kotlin.math.roundToInt
import kotlin.math.sin

/**
 * IMG04 — Geometric transforms on [PixelContainer].
 *
 * Point ops (IMG03) change what a pixel *is*; geometric transforms change
 * *where* a pixel is. Every transform here is a spatial warp: flips,
 * rotations, crops, scaling, arbitrary-angle rotation, affine, perspective.
 *
 * ## Inverse-warp convention
 *
 * Every continuous transform is an **inverse warp** (pull mapping):
 * ```
 * for each output pixel (x', y'):
 *     compute corresponding source coordinate (u, v)
 *     sample the source at (u, v)
 *     write the sample to output (x', y')
 * ```
 * Forward warps leave holes and need atomic writes for many-to-one mappings.
 * Inverse warps visit every output pixel exactly once and sampling is always
 * well-defined (with a chosen OOB policy for `(u, v)` outside the source).
 *
 * ## Pixel-centre model (+0.5 / -0.5)
 *
 * A pixel at integer `(x, y)` occupies a unit square centred at
 * `(x + 0.5, y + 0.5)` in continuous space. When we scale by `sx`, the
 * centre of output pixel `x'` maps to source coordinate
 * `u = (x' + 0.5) / sx - 0.5`. Without this offset every resampled image
 * would be shifted half a pixel — the source of more "slight blur" bugs
 * than any other off-by-one in imaging.
 *
 * ## Linear-light resampling
 *
 * sRGB bytes are gamma-compressed. Averaging byte 0 and byte 255 naively
 * gives 127, which looks noticeably darker than the mid-grey a camera would
 * record. Bilinear and bicubic therefore decode to linear light before
 * blending and re-encode afterwards. Nearest-neighbour doesn't blend, so it
 * works directly on raw bytes.
 *
 * ## Catmull-Rom
 *
 * The bicubic kernel is Catmull-Rom: `(B=0, C=0.5)` in the Mitchell family.
 * It interpolates (passes through the sample values), has compact `[-2, 2]`
 * support, and C1 continuity. Negative lobes mean the result can overshoot
 * `[0, 1]`, so [encode] clamps before converting back to bytes.
 */
object ImageGeometricTransforms {

    // ----------------------------------------------------------------------
    // Public enums
    // ----------------------------------------------------------------------

    /** Interpolation kernel for continuous sampling. */
    enum class Interpolation { NEAREST, BILINEAR, BICUBIC }

    /**
     * Output size policy for [rotate].
     *
     * - [FIT]  — enlarge output to contain the entire rotated source.
     * - [CROP] — keep the original dimensions, clipping rotated corners.
     */
    enum class RotateBounds { FIT, CROP }

    /**
     * Policy for sampling outside `[0, width) × [0, height)`:
     *
     * - [ZERO]      — return transparent black.
     * - [REPLICATE] — clamp to the nearest border pixel.
     * - [REFLECT]   — mirror the image at each border (period `2 * max`).
     * - [WRAP]      — tile the image periodically.
     */
    enum class OutOfBounds { ZERO, REPLICATE, REFLECT, WRAP }

    // ----------------------------------------------------------------------
    // sRGB ↔ linear-light
    //
    // Identical to IMG03. We reproduce it here rather than depend on
    // image-point-ops because geometric transforms should not inherit colour
    // operations — the two packages are siblings, not a stack.
    // ----------------------------------------------------------------------

    private val SRGB_TO_LINEAR: DoubleArray = DoubleArray(256) { i ->
        val c = i / 255.0
        if (c <= 0.04045) c / 12.92 else ((c + 0.055) / 1.055).pow(2.4)
    }

    private fun decode(b: Int): Double = SRGB_TO_LINEAR[b and 0xFF]

    private fun encode(linear: Double): Int {
        val c = linear.coerceIn(0.0, 1.0)
        val s = if (c <= 0.0031308) c * 12.92 else 1.055 * c.pow(1.0 / 2.4) - 0.055
        return (s * 255.0).roundToInt()
    }

    // ----------------------------------------------------------------------
    // OOB coordinate resolution
    //
    // `resolveCoord` returns -1 when the policy demands "return zero"; the
    // caller then treats that as transparent black. Returning Int (not
    // nullable) keeps the inner loops free of Int? box allocations.
    // ----------------------------------------------------------------------

    private fun resolveCoord(x: Int, maxVal: Int, oob: OutOfBounds): Int {
        if (x in 0 until maxVal) return x
        return when (oob) {
            OutOfBounds.ZERO -> -1
            OutOfBounds.REPLICATE -> x.coerceIn(0, maxVal - 1)
            OutOfBounds.REFLECT -> {
                // Mirror the image so it tiles the line with period 2*max;
                // the fold at the edges creates the "bathroom tile" effect.
                val period = 2 * maxVal
                var xm = x % period
                if (xm < 0) xm += period
                if (xm >= maxVal) period - xm - 1 else xm
            }
            OutOfBounds.WRAP -> {
                var xm = x % maxVal
                if (xm < 0) xm += maxVal
                xm
            }
        }
    }

    private fun getPixelOob(src: PixelContainer, x: Int, y: Int, oob: OutOfBounds): IntArray {
        val xi = resolveCoord(x, src.width, oob)
        val yi = resolveCoord(y, src.height, oob)
        if (xi < 0 || yi < 0) return intArrayOf(0, 0, 0, 0)
        return PixelOps.pixelAt(src, xi, yi)
    }

    // ----------------------------------------------------------------------
    // Catmull-Rom kernel
    //
    // The (B=0, C=0.5) member of the Mitchell-Netravali family. Interpolates
    // (w(0)=1, w(±1)=0, w(±2)=0), has support in [-2, 2], is C1-continuous,
    // and has slight negative lobes — hence the clamp in encode().
    // ----------------------------------------------------------------------

    private fun catmullRom(d: Double): Double {
        val ad = abs(d)
        return when {
            ad < 1.0 -> 1.5 * ad * ad * ad - 2.5 * ad * ad + 1.0
            ad < 2.0 -> -0.5 * ad * ad * ad + 2.5 * ad * ad - 4.0 * ad + 2.0
            else -> 0.0
        }
    }

    // ----------------------------------------------------------------------
    // Sampling
    // ----------------------------------------------------------------------

    private fun sampleNearest(src: PixelContainer, u: Double, v: Double, oob: OutOfBounds): IntArray {
        val xi = u.roundToInt()
        val yi = v.roundToInt()
        return getPixelOob(src, xi, yi, oob)
    }

    private fun sampleBilinear(src: PixelContainer, u: Double, v: Double, oob: OutOfBounds): IntArray {
        val x0 = floor(u).toInt()
        val y0 = floor(v).toInt()
        val wx1 = u - x0
        val wx0 = 1.0 - wx1
        val wy1 = v - y0
        val wy0 = 1.0 - wy1

        val p00 = getPixelOob(src, x0,     y0,     oob)
        val p10 = getPixelOob(src, x0 + 1, y0,     oob)
        val p01 = getPixelOob(src, x0,     y0 + 1, oob)
        val p11 = getPixelOob(src, x0 + 1, y0 + 1, oob)

        val out = IntArray(4)
        for (c in 0..2) {
            val lin = decode(p00[c]) * wx0 * wy0 +
                      decode(p10[c]) * wx1 * wy0 +
                      decode(p01[c]) * wx0 * wy1 +
                      decode(p11[c]) * wx1 * wy1
            out[c] = encode(lin)
        }
        // Alpha is linear coverage by definition; blend without sRGB decode.
        out[3] = (p00[3] * wx0 * wy0 +
                  p10[3] * wx1 * wy0 +
                  p01[3] * wx0 * wy1 +
                  p11[3] * wx1 * wy1).roundToInt().coerceIn(0, 255)
        return out
    }

    private fun sampleBicubic(src: PixelContainer, u: Double, v: Double, oob: OutOfBounds): IntArray {
        val x0 = floor(u).toInt()
        val y0 = floor(v).toInt()

        // Precompute 4 horizontal and 4 vertical Catmull-Rom weights.
        val wu = DoubleArray(4) { i -> catmullRom(u - (x0 + (i - 1))) }
        val wv = DoubleArray(4) { j -> catmullRom(v - (y0 + (j - 1))) }

        val acc = DoubleArray(4)
        for (dy in -1..2) {
            val yrow = dy + 1
            for (dx in -1..2) {
                val p = getPixelOob(src, x0 + dx, y0 + dy, oob)
                val w = wu[dx + 1] * wv[yrow]
                acc[0] += decode(p[0]) * w
                acc[1] += decode(p[1]) * w
                acc[2] += decode(p[2]) * w
                acc[3] += (p[3] / 255.0) * w
            }
        }
        return intArrayOf(
            encode(acc[0]),
            encode(acc[1]),
            encode(acc[2]),
            (acc[3].coerceIn(0.0, 1.0) * 255.0).roundToInt()
        )
    }

    private fun samplePixel(src: PixelContainer, u: Double, v: Double, interp: Interpolation, oob: OutOfBounds): IntArray =
        when (interp) {
            Interpolation.NEAREST  -> sampleNearest(src, u, v, oob)
            Interpolation.BILINEAR -> sampleBilinear(src, u, v, oob)
            Interpolation.BICUBIC  -> sampleBicubic(src, u, v, oob)
        }

    // ======================================================================
    // LOSSLESS TRANSFORMS — byte-level, no interpolation, no colour conv.
    // ======================================================================

    /** Mirror the image left↔right. Double application is the identity. */
    fun flipHorizontal(src: PixelContainer): PixelContainer {
        val out = PixelContainer(src.width, src.height)
        for (y in 0 until src.height)
            for (x in 0 until src.width) {
                val p = PixelOps.pixelAt(src, x, y)
                PixelOps.setPixel(out, src.width - 1 - x, y, p[0], p[1], p[2], p[3])
            }
        return out
    }

    /** Mirror the image top↔bottom. Double application is the identity. */
    fun flipVertical(src: PixelContainer): PixelContainer {
        val out = PixelContainer(src.width, src.height)
        for (y in 0 until src.height)
            for (x in 0 until src.width) {
                val p = PixelOps.pixelAt(src, x, y)
                PixelOps.setPixel(out, x, src.height - 1 - y, p[0], p[1], p[2], p[3])
            }
        return out
    }

    /**
     * Rotate 90° clockwise. Dimensions swap: `outW = srcH`, `outH = srcW`.
     *
     * Inverse warp: the output pixel `(x', y')` is supplied by source
     * `(y', H - 1 - x')` — here `H = srcHeight`.
     */
    fun rotate90CW(src: PixelContainer): PixelContainer {
        val out = PixelContainer(src.height, src.width)
        for (yp in 0 until out.height)
            for (xp in 0 until out.width) {
                val p = PixelOps.pixelAt(src, yp, src.height - 1 - xp)
                PixelOps.setPixel(out, xp, yp, p[0], p[1], p[2], p[3])
            }
        return out
    }

    /**
     * Rotate 90° counter-clockwise. Dimensions swap.
     *
     * Inverse warp: output `(x', y')` comes from source `(W - 1 - y', x')`.
     */
    fun rotate90CCW(src: PixelContainer): PixelContainer {
        val out = PixelContainer(src.height, src.width)
        for (yp in 0 until out.height)
            for (xp in 0 until out.width) {
                val p = PixelOps.pixelAt(src, src.width - 1 - yp, xp)
                PixelOps.setPixel(out, xp, yp, p[0], p[1], p[2], p[3])
            }
        return out
    }

    /** Rotate 180°. Equivalent to two 90° CW rotations, done in one pass. */
    fun rotate180(src: PixelContainer): PixelContainer {
        val out = PixelContainer(src.width, src.height)
        for (y in 0 until src.height)
            for (x in 0 until src.width) {
                val p = PixelOps.pixelAt(src, src.width - 1 - x, src.height - 1 - y)
                PixelOps.setPixel(out, x, y, p[0], p[1], p[2], p[3])
            }
        return out
    }

    /**
     * Extract a `w × h` rectangle with top-left at `(x0, y0)`. Coordinates
     * falling outside the source are filled with transparent black (the
     * default OOB behaviour of [PixelOps.pixelAt]).
     */
    fun crop(src: PixelContainer, x0: Int, y0: Int, w: Int, h: Int): PixelContainer {
        val out = PixelContainer(w, h)
        for (y in 0 until h)
            for (x in 0 until w) {
                val p = PixelOps.pixelAt(src, x0 + x, y0 + y)
                PixelOps.setPixel(out, x, y, p[0], p[1], p[2], p[3])
            }
        return out
    }

    // ======================================================================
    // CONTINUOUS TRANSFORMS — interpolation in linear light
    // ======================================================================

    /**
     * Resample the image to `outW × outH` using [interp].
     *
     * Uses the inverse-warp pixel-centre model: for each output pixel `x'`
     * with scale factor `sx = outW / srcW`, the source coordinate is
     * `u = (x' + 0.5) / sx - 0.5`. REPLICATE OOB keeps borders clean.
     */
    fun scale(
        src: PixelContainer,
        outW: Int,
        outH: Int,
        interp: Interpolation = Interpolation.BILINEAR
    ): PixelContainer {
        val out = PixelContainer(outW, outH)
        val sx = outW.toDouble() / src.width
        val sy = outH.toDouble() / src.height
        for (yp in 0 until outH)
            for (xp in 0 until outW) {
                val u = (xp + 0.5) / sx - 0.5
                val v = (yp + 0.5) / sy - 0.5
                val p = samplePixel(src, u, v, interp, OutOfBounds.REPLICATE)
                PixelOps.setPixel(out, xp, yp, p[0], p[1], p[2], p[3])
            }
        return out
    }

    /**
     * Rotate by `radians` (positive = CCW in standard mathematical
     * convention — here screen-y points down, so visually it looks CW).
     *
     * In [RotateBounds.FIT] the output is the smallest axis-aligned
     * bounding box of the rotated source; in [RotateBounds.CROP] the
     * output keeps the input dimensions and the rotated image may be
     * clipped at the corners.
     */
    fun rotate(
        src: PixelContainer,
        radians: Double,
        interp: Interpolation = Interpolation.BILINEAR,
        bounds: RotateBounds = RotateBounds.FIT
    ): PixelContainer {
        val W = src.width; val H = src.height
        val cosA = cos(radians); val sinA = sin(radians)

        val outW: Int; val outH: Int
        if (bounds == RotateBounds.FIT) {
            outW = ceil(W * abs(cosA) + H * abs(sinA)).toInt()
            outH = ceil(W * abs(sinA) + H * abs(cosA)).toInt()
        } else {
            outW = W; outH = H
        }
        require(outW.toLong() * outH.toLong() <= Int.MAX_VALUE / 4) {
            "Rotated canvas too large: ${outW}×${outH}"
        }

        val cxIn = W / 2.0; val cyIn = H / 2.0
        val cxOut = outW / 2.0; val cyOut = outH / 2.0
        val out = PixelContainer(outW, outH)

        for (yp in 0 until outH)
            for (xp in 0 until outW) {
                val dx = xp - cxOut
                val dy = yp - cyOut
                // Inverse rotation matrix: [[cos, sin], [-sin, cos]].
                val u = cxIn + cosA * dx + sinA * dy
                val v = cyIn - sinA * dx + cosA * dy
                val p = samplePixel(src, u, v, interp, OutOfBounds.ZERO)
                PixelOps.setPixel(out, xp, yp, p[0], p[1], p[2], p[3])
            }
        return out
    }

    /** Shift the image by `(tx, ty)` pixels. Background fills with ZERO. */
    fun translate(
        src: PixelContainer,
        tx: Double,
        ty: Double,
        interp: Interpolation = Interpolation.BILINEAR
    ): PixelContainer {
        val out = PixelContainer(src.width, src.height)
        for (yp in 0 until src.height)
            for (xp in 0 until src.width) {
                val p = samplePixel(src, xp - tx, yp - ty, interp, OutOfBounds.ZERO)
                PixelOps.setPixel(out, xp, yp, p[0], p[1], p[2], p[3])
            }
        return out
    }

    /**
     * Apply a 2x3 affine transform given in forward form: for each source
     * point `(x, y)` the destination is `m * [x, y, 1]^T`. This function
     * inverts `m` internally and performs an inverse warp into an output of
     * the same size as the input.
     *
     * ```
     * [ a b c ] [x]   [x']
     * [ d e f ] [y] = [y']
     *           [1]
     * ```
     * Inverse (using `det = a*e - b*d`):
     * ```
     * [  e/det  -b/det  (b*f - c*e)/det ]
     * [ -d/det   a/det  (c*d - a*f)/det ]
     * ```
     */
    fun affine(
        src: PixelContainer,
        m: Array<DoubleArray>,
        interp: Interpolation = Interpolation.BILINEAR,
        oob: OutOfBounds = OutOfBounds.ZERO
    ): PixelContainer {
        require(m.size == 2 && m.all { it.size == 3 }) { "affine matrix must be 2x3" }
        val a = m[0][0]; val b = m[0][1]; val c = m[0][2]
        val d = m[1][0]; val e = m[1][1]; val f = m[1][2]
        val det = a * e - b * d
        require(abs(det) > 1e-12) { "affine matrix is singular" }
        val ia =  e / det
        val ib = -b / det
        val ic = -d / det
        val id =  a / det

        val out = PixelContainer(src.width, src.height)
        for (y in 0 until src.height)
            for (x in 0 until src.width) {
                val dx = x - c
                val dy = y - f
                val u = ia * dx + ib * dy
                val v = ic * dx + id * dy
                val p = samplePixel(src, u, v, interp, oob)
                PixelOps.setPixel(out, x, y, p[0], p[1], p[2], p[3])
            }
        return out
    }

    /**
     * Apply a 3x3 projective (homography) warp. Unlike [affine], the
     * caller supplies `h` in *inverse* form — i.e. already the matrix that
     * takes output coordinates to source coordinates. This is the more
     * convenient convention when computing a homography from four point
     * correspondences ("warp this quadrilateral back into a rectangle").
     *
     * The output size is the same as the source. For each output pixel
     * `(x', y')`, the homogeneous source point is
     * `[uh, vh, w]^T = h * [x', y', 1]^T`, and the Euclidean source is
     * `(uh / w, vh / w)`. When `w == 0` the point is at infinity and we
     * write transparent black.
     */
    fun perspectiveWarp(
        src: PixelContainer,
        h: Array<DoubleArray>,
        interp: Interpolation = Interpolation.BILINEAR,
        oob: OutOfBounds = OutOfBounds.ZERO
    ): PixelContainer {
        require(h.size == 3 && h.all { it.size == 3 }) { "homography must be 3x3" }
        val out = PixelContainer(src.width, src.height)
        for (y in 0 until src.height)
            for (x in 0 until src.width) {
                val uh = h[0][0] * x + h[0][1] * y + h[0][2]
                val vh = h[1][0] * x + h[1][1] * y + h[1][2]
                val w  = h[2][0] * x + h[2][1] * y + h[2][2]
                if (abs(w) < 1e-12) {
                    PixelOps.setPixel(out, x, y, 0, 0, 0, 0)
                    continue
                }
                val u = uh / w
                val v = vh / w
                val p = samplePixel(src, u, v, interp, oob)
                PixelOps.setPixel(out, x, y, p[0], p[1], p[2], p[3])
            }
        return out
    }

    // ----------------------------------------------------------------------
    // 3x3 matrix inverse (helper, public — useful for callers composing
    // homographies by hand).
    // ----------------------------------------------------------------------

    /**
     * Invert a 3x3 matrix by the adjugate method. Throws if the matrix is
     * singular (|det| below `1e-12`).
     */
    fun invert3x3(m: Array<DoubleArray>): Array<DoubleArray> {
        require(m.size == 3 && m.all { it.size == 3 }) { "matrix must be 3x3" }
        val a = m[0][0]; val b = m[0][1]; val c = m[0][2]
        val d = m[1][0]; val e = m[1][1]; val f = m[1][2]
        val g = m[2][0]; val h = m[2][1]; val i = m[2][2]
        val det = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g)
        require(abs(det) > 1e-12) { "matrix is singular" }
        return arrayOf(
            doubleArrayOf( (e * i - f * h) / det, -(b * i - c * h) / det,  (b * f - c * e) / det),
            doubleArrayOf(-(d * i - f * g) / det,  (a * i - c * g) / det, -(a * f - c * d) / det),
            doubleArrayOf( (d * h - e * g) / det, -(a * h - b * g) / det,  (a * e - b * d) / det)
        )
    }
}
