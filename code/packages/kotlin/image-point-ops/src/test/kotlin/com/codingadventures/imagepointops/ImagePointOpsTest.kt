package com.codingadventures.imagepointops

import com.codingadventures.pixelcontainer.PixelContainer
import com.codingadventures.pixelcontainer.PixelOps
import kotlin.math.abs
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFails
import kotlin.test.assertTrue
import kotlin.test.assertNotEquals

class ImagePointOpsTest {

    // --- small helpers ---------------------------------------------------

    private fun pix(r: Int, g: Int, b: Int, a: Int = 255): PixelContainer {
        val c = PixelContainer(1, 1)
        PixelOps.setPixel(c, 0, 0, r, g, b, a)
        return c
    }

    private fun get(c: PixelContainer) = PixelOps.pixelAt(c, 0, 0)

    private fun assertNear(a: Int, b: Int, tol: Int = 2) {
        assertTrue(abs(a - b) <= tol, "expected ~$b got $a (tol=$tol)")
    }

    // --- invert ----------------------------------------------------------

    @Test fun invertInvertsRGB() {
        val out = ImagePointOps.invert(pix(10, 20, 30, 128))
        assertContentEquals(intArrayOf(245, 235, 225, 128), get(out))
    }

    @Test fun invertTwiceIsIdentity() {
        val once = ImagePointOps.invert(pix(50, 100, 150, 255))
        val twice = ImagePointOps.invert(once)
        assertContentEquals(intArrayOf(50, 100, 150, 255), get(twice))
    }

    // --- threshold -------------------------------------------------------

    @Test fun thresholdBelow() {
        val out = ImagePointOps.threshold(pix(10, 10, 10), 100)
        assertContentEquals(intArrayOf(0, 0, 0, 255), get(out))
    }

    @Test fun thresholdAbove() {
        val out = ImagePointOps.threshold(pix(200, 200, 200), 100)
        assertContentEquals(intArrayOf(255, 255, 255, 255), get(out))
    }

    @Test fun thresholdLuminanceWeightsGreenMost() {
        // pure green above threshold — white; pure red below — black
        val g = ImagePointOps.thresholdLuminance(pix(0, 200, 0), 100)
        val r = ImagePointOps.thresholdLuminance(pix(200, 0, 0), 100)
        assertEquals(255, get(g)[0])
        assertEquals(0, get(r)[0])
    }

    // --- posterize -------------------------------------------------------

    @Test fun posterizeTwoLevelsGivesBlackOrWhite() {
        val dark = ImagePointOps.posterize(pix(10, 10, 10), 2)
        val light = ImagePointOps.posterize(pix(250, 250, 250), 2)
        assertEquals(0, get(dark)[0])
        assertEquals(255, get(light)[0])
    }

    @Test fun posterizeRejectsOneLevel() {
        assertFails { ImagePointOps.posterize(pix(0, 0, 0), 1) }
    }

    // --- channel ops -----------------------------------------------------

    @Test fun swapRgbBgr() {
        val out = ImagePointOps.swapRgbBgr(pix(10, 20, 30))
        assertContentEquals(intArrayOf(30, 20, 10, 255), get(out))
    }

    @Test fun extractChannelRed() {
        val out = ImagePointOps.extractChannel(pix(100, 200, 50, 200), 0)
        assertContentEquals(intArrayOf(100, 100, 100, 255), get(out))
    }

    @Test fun extractChannelAlpha() {
        val out = ImagePointOps.extractChannel(pix(100, 200, 50, 77), 3)
        assertContentEquals(intArrayOf(77, 77, 77, 255), get(out))
    }

    @Test fun extractChannelRejectsBadIndex() {
        assertFails { ImagePointOps.extractChannel(pix(0, 0, 0), 4) }
    }

    // --- brightness / contrast ------------------------------------------

    @Test fun brightnessPositiveSaturates() {
        val out = ImagePointOps.brightness(pix(250, 250, 250), 50)
        assertContentEquals(intArrayOf(255, 255, 255, 255), get(out))
    }

    @Test fun brightnessNegativeSaturates() {
        val out = ImagePointOps.brightness(pix(10, 10, 10), -50)
        assertContentEquals(intArrayOf(0, 0, 0, 255), get(out))
    }

    @Test fun contrastOneIsIdentity() {
        val out = ImagePointOps.contrast(pix(40, 80, 200), 1.0)
        assertContentEquals(intArrayOf(40, 80, 200, 255), get(out))
    }

    @Test fun contrastZeroCollapsesToMidGrey() {
        val out = ImagePointOps.contrast(pix(0, 128, 255), 0.0)
        assertContentEquals(intArrayOf(128, 128, 128, 255), get(out))
    }

    // --- gamma / exposure -----------------------------------------------

    @Test fun gammaOneIsNearIdentity() {
        val out = ImagePointOps.gamma(pix(50, 100, 150), 1.0)
        val p = get(out)
        assertNear(p[0], 50); assertNear(p[1], 100); assertNear(p[2], 150)
    }

    @Test fun gammaDarkensWithG2() {
        val p = get(ImagePointOps.gamma(pix(128, 128, 128), 2.0))
        assertTrue(p[0] < 128)
    }

    @Test fun exposureZeroIsNearIdentity() {
        val out = ImagePointOps.exposure(pix(50, 100, 150), 0.0)
        val p = get(out)
        assertNear(p[0], 50); assertNear(p[1], 100); assertNear(p[2], 150)
    }

    @Test fun exposurePositiveBrightens() {
        val p = get(ImagePointOps.exposure(pix(50, 50, 50), 1.0))
        assertTrue(p[0] > 50)
    }

    @Test fun exposureNegativeDarkens() {
        val p = get(ImagePointOps.exposure(pix(200, 200, 200), -1.0))
        assertTrue(p[0] < 200)
    }

    // --- greyscale -------------------------------------------------------

    @Test fun greyscaleRec709PureWhite() {
        val p = get(ImagePointOps.greyscale(pix(255, 255, 255), ImagePointOps.GreyscaleMethod.REC709))
        assertNear(p[0], 255, 1); assertEquals(p[0], p[1]); assertEquals(p[1], p[2])
    }

    @Test fun greyscaleBt601GivesEqualChannels() {
        val p = get(ImagePointOps.greyscale(pix(200, 100, 50), ImagePointOps.GreyscaleMethod.BT601))
        assertEquals(p[0], p[1]); assertEquals(p[1], p[2])
    }

    @Test fun greyscaleAverageGivesEqualChannels() {
        val p = get(ImagePointOps.greyscale(pix(10, 20, 30), ImagePointOps.GreyscaleMethod.AVERAGE))
        assertEquals(p[0], p[1]); assertEquals(p[1], p[2])
    }

    // --- matrix / sepia --------------------------------------------------

    @Test fun sepiaShiftsTowardsBrown() {
        val p = get(ImagePointOps.sepia(pix(255, 255, 255)))
        // R > G > B is the sepia signature
        assertTrue(p[0] >= p[1] && p[1] >= p[2])
    }

    @Test fun identityColourMatrixIsIdentity() {
        val id = arrayOf(
            doubleArrayOf(1.0, 0.0, 0.0),
            doubleArrayOf(0.0, 1.0, 0.0),
            doubleArrayOf(0.0, 0.0, 1.0)
        )
        val p = get(ImagePointOps.colourMatrix(pix(50, 100, 150), id))
        assertNear(p[0], 50); assertNear(p[1], 100); assertNear(p[2], 150)
    }

    @Test fun colourMatrixRejectsBadShape() {
        val bad = arrayOf(doubleArrayOf(1.0, 0.0))
        assertFails { ImagePointOps.colourMatrix(pix(0, 0, 0), bad) }
    }

    // --- saturate --------------------------------------------------------

    @Test fun saturateZeroGivesGrey() {
        val p = get(ImagePointOps.saturate(pix(200, 50, 50), 0.0))
        assertEquals(p[0], p[1]); assertEquals(p[1], p[2])
    }

    @Test fun saturateOneIsNearIdentity() {
        val p = get(ImagePointOps.saturate(pix(100, 50, 20), 1.0))
        assertNear(p[0], 100); assertNear(p[1], 50); assertNear(p[2], 20)
    }

    // --- hue rotate ------------------------------------------------------

    @Test fun hueRotate360IsNearIdentity() {
        val p = get(ImagePointOps.hueRotate(pix(200, 100, 50), 360.0))
        assertNear(p[0], 200); assertNear(p[1], 100); assertNear(p[2], 50)
    }

    @Test fun hueRotate120PermutesPrimaries() {
        // red at 0° rotates to green at 120°
        val p = get(ImagePointOps.hueRotate(pix(255, 0, 0), 120.0))
        assertTrue(p[1] > p[0] && p[1] > p[2])
    }

    // --- srgb<->linear image --------------------------------------------

    @Test fun srgbToLinearRoundTripApproximate() {
        val src = pix(128, 64, 200)
        val lin = ImagePointOps.srgbToLinearImage(src)
        val back = ImagePointOps.linearToSrgbImage(lin)
        val p = get(back)
        assertNear(p[0], 128, 3); assertNear(p[1], 64, 3); assertNear(p[2], 200, 3)
    }

    // --- LUTs ------------------------------------------------------------

    @Test fun buildIdentityLutThenApplyIsNoOp() {
        val id = ImagePointOps.buildLut1dU8 { it }
        val out = ImagePointOps.applyLut1dU8(pix(10, 20, 30), id, id, id)
        val p = get(out)
        assertNear(p[0], 10, 1); assertNear(p[1], 20, 1); assertNear(p[2], 30, 1)
    }

    @Test fun gammaLutProducesReasonableValues() {
        val lut = ImagePointOps.buildGammaLut(2.0)
        assertEquals(0, lut[0].toInt() and 0xFF)
        assertEquals(255, lut[255].toInt() and 0xFF)
        // gamma 2.0 darkens mids
        assertTrue((lut[128].toInt() and 0xFF) < 128)
    }

    @Test fun applyLutRejectsShortLut() {
        assertFails {
            ImagePointOps.applyLut1dU8(pix(0,0,0), ByteArray(10), ByteArray(256), ByteArray(256))
        }
    }

    // --- multi-pixel shape ----------------------------------------------

    @Test fun operationsProduceSameShape() {
        val src = PixelContainer(3, 2)
        PixelOps.fillPixels(src, 10, 20, 30, 40)
        val out = ImagePointOps.invert(src)
        assertEquals(src.width, out.width)
        assertEquals(src.height, out.height)
    }

    @Test fun operationsDoNotMutateSource() {
        val src = pix(10, 20, 30, 40)
        val before = src.data.copyOf()
        ImagePointOps.invert(src)
        ImagePointOps.gamma(src, 2.0)
        assertContentEquals(before, src.data)
    }

    @Test fun invertVsOriginalDiffers() {
        val src = pix(10, 20, 30)
        assertNotEquals(
            get(src).toList(),
            get(ImagePointOps.invert(src)).toList()
        )
    }
}
