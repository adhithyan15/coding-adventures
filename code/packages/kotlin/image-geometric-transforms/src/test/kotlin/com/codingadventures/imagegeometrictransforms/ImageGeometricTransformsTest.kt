package com.codingadventures.imagegeometrictransforms

import com.codingadventures.pixelcontainer.PixelContainer
import com.codingadventures.pixelcontainer.PixelOps
import com.codingadventures.imagegeometrictransforms.ImageGeometricTransforms as IGT
import kotlin.math.PI
import kotlin.math.abs
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFails
import kotlin.test.assertTrue

class ImageGeometricTransformsTest {

    // ---- helpers --------------------------------------------------------

    /**
     * Build a 3x3 image with a distinct colour at each pixel so we can
     * spot-check geometric operations by inspecting specific locations.
     *
     * Values are chosen so each channel is `10 * x + y + offset` — low
     * enough to stay inside the sRGB "dark" (linear) region where decode is
     * exactly `c / 12.92`, which lets us reason about linear-light
     * operations without dragging pow() in.
     */
    private fun grid3x3(): PixelContainer {
        val c = PixelContainer(3, 3)
        for (y in 0 until 3)
            for (x in 0 until 3)
                PixelOps.setPixel(c, x, y, 10 * x + y, 10 * x + y + 1, 10 * x + y + 2, 255)
        return c
    }

    private fun solid(w: Int, h: Int, r: Int, g: Int, b: Int, a: Int = 255): PixelContainer {
        val c = PixelContainer(w, h)
        PixelOps.fillPixels(c, r, g, b, a)
        return c
    }

    private fun assertNear(actual: Int, expected: Int, tol: Int = 2) {
        assertTrue(abs(actual - expected) <= tol, "expected ~$expected got $actual")
    }

    // ---- flips ---------------------------------------------------------

    @Test fun flipHorizontalReversesRows() {
        val src = grid3x3()
        val out = IGT.flipHorizontal(src)
        assertContentEquals(PixelOps.pixelAt(src, 0, 1), PixelOps.pixelAt(out, 2, 1))
        assertContentEquals(PixelOps.pixelAt(src, 2, 1), PixelOps.pixelAt(out, 0, 1))
    }

    @Test fun flipHorizontalTwiceIsIdentity() {
        val src = grid3x3()
        val out = IGT.flipHorizontal(IGT.flipHorizontal(src))
        for (y in 0 until 3) for (x in 0 until 3)
            assertContentEquals(PixelOps.pixelAt(src, x, y), PixelOps.pixelAt(out, x, y))
    }

    @Test fun flipVerticalReversesColumns() {
        val src = grid3x3()
        val out = IGT.flipVertical(src)
        assertContentEquals(PixelOps.pixelAt(src, 1, 0), PixelOps.pixelAt(out, 1, 2))
    }

    @Test fun flipVerticalTwiceIsIdentity() {
        val src = grid3x3()
        val out = IGT.flipVertical(IGT.flipVertical(src))
        for (y in 0 until 3) for (x in 0 until 3)
            assertContentEquals(PixelOps.pixelAt(src, x, y), PixelOps.pixelAt(out, x, y))
    }

    // ---- 90° rotations -------------------------------------------------

    @Test fun rotate90CWSwapsDimensions() {
        val src = PixelContainer(4, 3)
        val out = IGT.rotate90CW(src)
        assertEquals(3, out.width)
        assertEquals(4, out.height)
    }

    @Test fun rotate90CWMovesTopLeftToTopRight() {
        val src = PixelContainer(3, 3)
        PixelOps.setPixel(src, 0, 0, 255, 0, 0, 255)
        val out = IGT.rotate90CW(src)
        // top-left of source goes to top-right of output
        assertContentEquals(intArrayOf(255, 0, 0, 255), PixelOps.pixelAt(out, 2, 0))
    }

    @Test fun fourRotate90CWIsIdentity() {
        val src = grid3x3()
        var out = src
        repeat(4) { out = IGT.rotate90CW(out) }
        for (y in 0 until 3) for (x in 0 until 3)
            assertContentEquals(PixelOps.pixelAt(src, x, y), PixelOps.pixelAt(out, x, y))
    }

    @Test fun rotate90CCWMovesTopLeftToBottomLeft() {
        val src = PixelContainer(3, 3)
        PixelOps.setPixel(src, 0, 0, 255, 0, 0, 255)
        val out = IGT.rotate90CCW(src)
        assertContentEquals(intArrayOf(255, 0, 0, 255), PixelOps.pixelAt(out, 0, 2))
    }

    @Test fun fourRotate90CCWIsIdentity() {
        val src = grid3x3()
        var out = src
        repeat(4) { out = IGT.rotate90CCW(out) }
        for (y in 0 until 3) for (x in 0 until 3)
            assertContentEquals(PixelOps.pixelAt(src, x, y), PixelOps.pixelAt(out, x, y))
    }

    @Test fun rotate90CWAndCCWAreInverses() {
        val src = grid3x3()
        val out = IGT.rotate90CCW(IGT.rotate90CW(src))
        for (y in 0 until 3) for (x in 0 until 3)
            assertContentEquals(PixelOps.pixelAt(src, x, y), PixelOps.pixelAt(out, x, y))
    }

    @Test fun rotate180Twice() {
        val src = grid3x3()
        val out = IGT.rotate180(IGT.rotate180(src))
        for (y in 0 until 3) for (x in 0 until 3)
            assertContentEquals(PixelOps.pixelAt(src, x, y), PixelOps.pixelAt(out, x, y))
    }

    @Test fun rotate180MovesTopLeftToBottomRight() {
        val src = PixelContainer(3, 3)
        PixelOps.setPixel(src, 0, 0, 200, 100, 50, 255)
        val out = IGT.rotate180(src)
        assertContentEquals(intArrayOf(200, 100, 50, 255), PixelOps.pixelAt(out, 2, 2))
    }

    // ---- crop ----------------------------------------------------------

    @Test fun cropProducesRequestedDimensions() {
        val src = PixelContainer(10, 10)
        val out = IGT.crop(src, 2, 3, 4, 5)
        assertEquals(4, out.width)
        assertEquals(5, out.height)
    }

    @Test fun cropPreservesContent() {
        val src = grid3x3()
        val out = IGT.crop(src, 1, 1, 2, 2)
        assertContentEquals(PixelOps.pixelAt(src, 1, 1), PixelOps.pixelAt(out, 0, 0))
        assertContentEquals(PixelOps.pixelAt(src, 2, 2), PixelOps.pixelAt(out, 1, 1))
    }

    @Test fun cropOutsideSourceFillsZero() {
        val src = solid(2, 2, 255, 255, 255)
        val out = IGT.crop(src, 0, 0, 4, 4)
        assertContentEquals(intArrayOf(255, 255, 255, 255), PixelOps.pixelAt(out, 0, 0))
        assertContentEquals(intArrayOf(0, 0, 0, 0), PixelOps.pixelAt(out, 3, 3))
    }

    // ---- scale ---------------------------------------------------------

    @Test fun scaleIdentityPreservesImage() {
        val src = grid3x3()
        val out = IGT.scale(src, 3, 3, IGT.Interpolation.NEAREST)
        for (y in 0 until 3) for (x in 0 until 3)
            assertContentEquals(PixelOps.pixelAt(src, x, y), PixelOps.pixelAt(out, x, y))
    }

    @Test fun scaleDoublesSize() {
        val src = solid(2, 2, 100, 150, 200)
        val out = IGT.scale(src, 4, 4, IGT.Interpolation.NEAREST)
        assertEquals(4, out.width)
        assertEquals(4, out.height)
        assertContentEquals(intArrayOf(100, 150, 200, 255), PixelOps.pixelAt(out, 2, 2))
    }

    @Test fun scaleHalvesSize() {
        val src = solid(4, 4, 80, 120, 160)
        val out = IGT.scale(src, 2, 2, IGT.Interpolation.BILINEAR)
        assertEquals(2, out.width)
        val p = PixelOps.pixelAt(out, 0, 0)
        assertNear(p[0], 80, 3)
        assertNear(p[1], 120, 3)
        assertNear(p[2], 160, 3)
    }

    @Test fun scaleBicubicProducesValidImage() {
        val src = solid(4, 4, 100, 100, 100)
        val out = IGT.scale(src, 8, 8, IGT.Interpolation.BICUBIC)
        assertEquals(8, out.width); assertEquals(8, out.height)
        // Interior pixel should be close to source colour.
        val p = PixelOps.pixelAt(out, 4, 4)
        assertNear(p[0], 100, 5)
    }

    // ---- rotate (arbitrary angle) --------------------------------------

    @Test fun rotateZeroIsNearIdentity() {
        val src = grid3x3()
        val out = IGT.rotate(src, 0.0, IGT.Interpolation.NEAREST, IGT.RotateBounds.CROP)
        for (y in 0 until 3) for (x in 0 until 3) {
            val sp = PixelOps.pixelAt(src, x, y)
            val op = PixelOps.pixelAt(out, x, y)
            assertNear(sp[0], op[0], 2)
        }
    }

    @Test fun rotateFitEnlarges45Deg() {
        val src = PixelContainer(10, 10)
        val out = IGT.rotate(src, PI / 4, IGT.Interpolation.NEAREST, IGT.RotateBounds.FIT)
        assertTrue(out.width >= 14 && out.height >= 14)
    }

    @Test fun rotateCropPreservesDimensions() {
        val src = PixelContainer(10, 10)
        val out = IGT.rotate(src, PI / 4, IGT.Interpolation.NEAREST, IGT.RotateBounds.CROP)
        assertEquals(10, out.width)
        assertEquals(10, out.height)
    }

    @Test fun rotateOutsideIsTransparent() {
        val src = solid(4, 4, 255, 0, 0)
        val out = IGT.rotate(src, PI / 4, IGT.Interpolation.NEAREST, IGT.RotateBounds.FIT)
        // Corner of the output bounding box should be outside the rotated square.
        val p = PixelOps.pixelAt(out, 0, 0)
        assertEquals(0, p[3])
    }

    // ---- translate -----------------------------------------------------

    @Test fun translateShiftsPixels() {
        val src = PixelContainer(4, 4)
        PixelOps.setPixel(src, 1, 1, 200, 100, 50, 255)
        val out = IGT.translate(src, 1.0, 1.0, IGT.Interpolation.NEAREST)
        val p = PixelOps.pixelAt(out, 2, 2)
        assertContentEquals(intArrayOf(200, 100, 50, 255), p)
    }

    @Test fun translateZeroIsIdentity() {
        val src = grid3x3()
        val out = IGT.translate(src, 0.0, 0.0, IGT.Interpolation.NEAREST)
        for (y in 0 until 3) for (x in 0 until 3)
            assertContentEquals(PixelOps.pixelAt(src, x, y), PixelOps.pixelAt(out, x, y))
    }

    // ---- affine --------------------------------------------------------

    @Test fun affineIdentityIsIdentity() {
        val src = grid3x3()
        val id = arrayOf(doubleArrayOf(1.0, 0.0, 0.0), doubleArrayOf(0.0, 1.0, 0.0))
        val out = IGT.affine(src, id, IGT.Interpolation.NEAREST)
        for (y in 0 until 3) for (x in 0 until 3)
            assertContentEquals(PixelOps.pixelAt(src, x, y), PixelOps.pixelAt(out, x, y))
    }

    @Test fun affineTranslationMatchesTranslate() {
        val src = PixelContainer(5, 5)
        PixelOps.setPixel(src, 2, 2, 100, 150, 200, 255)
        // Forward translate by (1, 1): m = [[1,0,1],[0,1,1]]
        val m = arrayOf(doubleArrayOf(1.0, 0.0, 1.0), doubleArrayOf(0.0, 1.0, 1.0))
        val out = IGT.affine(src, m, IGT.Interpolation.NEAREST)
        assertContentEquals(intArrayOf(100, 150, 200, 255), PixelOps.pixelAt(out, 3, 3))
    }

    @Test fun affineRejectsBadShape() {
        val src = PixelContainer(2, 2)
        val bad = arrayOf(doubleArrayOf(1.0, 0.0))
        assertFails { IGT.affine(src, bad) }
    }

    @Test fun affineRejectsSingular() {
        val src = PixelContainer(2, 2)
        val singular = arrayOf(doubleArrayOf(0.0, 0.0, 0.0), doubleArrayOf(0.0, 0.0, 0.0))
        assertFails { IGT.affine(src, singular) }
    }

    // ---- perspectiveWarp ----------------------------------------------

    @Test fun perspectiveIdentityIsIdentity() {
        val src = grid3x3()
        val id = arrayOf(
            doubleArrayOf(1.0, 0.0, 0.0),
            doubleArrayOf(0.0, 1.0, 0.0),
            doubleArrayOf(0.0, 0.0, 1.0)
        )
        val out = IGT.perspectiveWarp(src, id, IGT.Interpolation.NEAREST)
        for (y in 0 until 3) for (x in 0 until 3)
            assertContentEquals(PixelOps.pixelAt(src, x, y), PixelOps.pixelAt(out, x, y))
    }

    @Test fun perspectiveDoublesWhenScalingHomographyUsed() {
        val src = solid(4, 4, 90, 90, 90)
        // Scale by 0.5 from output to source (inverse map): out coord -> out*0.5
        val h = arrayOf(
            doubleArrayOf(0.5, 0.0, 0.0),
            doubleArrayOf(0.0, 0.5, 0.0),
            doubleArrayOf(0.0, 0.0, 1.0)
        )
        val out = IGT.perspectiveWarp(src, h, IGT.Interpolation.NEAREST)
        val p = PixelOps.pixelAt(out, 1, 1)
        assertNear(p[0], 90, 5)
    }

    @Test fun perspectiveRejectsBadShape() {
        val src = PixelContainer(2, 2)
        val bad = arrayOf(doubleArrayOf(1.0))
        assertFails { IGT.perspectiveWarp(src, bad) }
    }

    @Test fun perspectiveHandlesZeroW() {
        val src = solid(2, 2, 100, 100, 100)
        // Degenerate row 3 produces w == 0 for every output pixel.
        val h = arrayOf(
            doubleArrayOf(1.0, 0.0, 0.0),
            doubleArrayOf(0.0, 1.0, 0.0),
            doubleArrayOf(0.0, 0.0, 0.0)
        )
        val out = IGT.perspectiveWarp(src, h, IGT.Interpolation.NEAREST)
        assertContentEquals(intArrayOf(0, 0, 0, 0), PixelOps.pixelAt(out, 0, 0))
    }

    // ---- invert3x3 -----------------------------------------------------

    @Test fun invert3x3OfIdentityIsIdentity() {
        val id = arrayOf(
            doubleArrayOf(1.0, 0.0, 0.0),
            doubleArrayOf(0.0, 1.0, 0.0),
            doubleArrayOf(0.0, 0.0, 1.0)
        )
        val inv = IGT.invert3x3(id)
        for (r in 0 until 3) for (c in 0 until 3)
            assertTrue(abs(inv[r][c] - id[r][c]) < 1e-9)
    }

    @Test fun invert3x3Roundtrip() {
        val m = arrayOf(
            doubleArrayOf(2.0, 0.0, 1.0),
            doubleArrayOf(0.0, 3.0, 2.0),
            doubleArrayOf(0.0, 0.0, 1.0)
        )
        val inv = IGT.invert3x3(m)
        // m * inv ≈ identity
        for (r in 0 until 3) for (c in 0 until 3) {
            var s = 0.0
            for (k in 0 until 3) s += m[r][k] * inv[k][c]
            val expected = if (r == c) 1.0 else 0.0
            assertTrue(abs(s - expected) < 1e-9, "m*inv[$r][$c] = $s, expected $expected")
        }
    }

    @Test fun invert3x3RejectsSingular() {
        val singular = arrayOf(
            doubleArrayOf(1.0, 2.0, 3.0),
            doubleArrayOf(2.0, 4.0, 6.0),   // row2 = 2 * row1
            doubleArrayOf(3.0, 6.0, 9.0)
        )
        assertFails { IGT.invert3x3(singular) }
    }

    // ---- OOB policies indirectly via rotate/perspective ---------------

    @Test fun rotateReplicateDiffersFromZero() {
        val src = solid(4, 4, 255, 0, 0)
        // Build a homography that shifts way out: inverse map sends (0,0) to (-10,-10)
        val shiftOut = arrayOf(
            doubleArrayOf(1.0, 0.0, -10.0),
            doubleArrayOf(0.0, 1.0, -10.0),
            doubleArrayOf(0.0, 0.0,   1.0)
        )
        val zero = IGT.perspectiveWarp(src, shiftOut, IGT.Interpolation.NEAREST, IGT.OutOfBounds.ZERO)
        val repl = IGT.perspectiveWarp(src, shiftOut, IGT.Interpolation.NEAREST, IGT.OutOfBounds.REPLICATE)
        assertEquals(0, PixelOps.pixelAt(zero, 0, 0)[3])
        assertEquals(255, PixelOps.pixelAt(repl, 0, 0)[0])
    }

    @Test fun oobReflectAndWrapStayInBounds() {
        // Use perspectiveWarp with a pure shift so we hit OOB and check that
        // reflect/wrap produce valid (non-zero) pixels from the original.
        val src = solid(4, 4, 50, 100, 150)
        val shift = arrayOf(
            doubleArrayOf(1.0, 0.0, -2.0),
            doubleArrayOf(0.0, 1.0, -2.0),
            doubleArrayOf(0.0, 0.0,  1.0)
        )
        val reflect = IGT.perspectiveWarp(src, shift, IGT.Interpolation.NEAREST, IGT.OutOfBounds.REFLECT)
        val wrap    = IGT.perspectiveWarp(src, shift, IGT.Interpolation.NEAREST, IGT.OutOfBounds.WRAP)
        // All pixels in a uniform image should come back as the original colour.
        assertContentEquals(intArrayOf(50, 100, 150, 255), PixelOps.pixelAt(reflect, 0, 0))
        assertContentEquals(intArrayOf(50, 100, 150, 255), PixelOps.pixelAt(wrap, 0, 0))
    }

    // ---- interpolation selection --------------------------------------

    @Test fun allInterpolationModesProduceValidOutputs() {
        val src = solid(8, 8, 120, 60, 30)
        for (interp in IGT.Interpolation.entries) {
            val out = IGT.scale(src, 4, 4, interp)
            assertEquals(4, out.width); assertEquals(4, out.height)
        }
    }

    @Test fun transformsProduceCorrectOutputDimensions() {
        val src = PixelContainer(5, 7)
        assertEquals(5 to 7, IGT.flipHorizontal(src).let { it.width to it.height })
        assertEquals(5 to 7, IGT.flipVertical(src).let { it.width to it.height })
        assertEquals(7 to 5, IGT.rotate90CW(src).let { it.width to it.height })
        assertEquals(7 to 5, IGT.rotate90CCW(src).let { it.width to it.height })
        assertEquals(5 to 7, IGT.rotate180(src).let { it.width to it.height })
    }
}
