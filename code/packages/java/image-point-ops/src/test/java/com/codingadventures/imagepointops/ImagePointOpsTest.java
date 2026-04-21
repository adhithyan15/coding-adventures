package com.codingadventures.imagepointops;

import com.codingadventures.pixelcontainer.PixelContainer;
import com.codingadventures.pixelcontainer.PixelOps;
import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.*;

class ImagePointOpsTest {

    /** Helper: a single-pixel container for tight checks. */
    private static PixelContainer single(int r, int g, int b, int a) {
        PixelContainer c = PixelOps.create(1, 1);
        PixelOps.setPixel(c, 0, 0, r, g, b, a);
        return c;
    }

    private static int[] px(PixelContainer c, int x, int y) {
        return PixelOps.pixelAt(c, x, y);
    }

    /* ---------------- u8-domain ---------------- */

    @Test
    void invertBasic() {
        PixelContainer out = ImagePointOps.invert(single(10, 20, 30, 128));
        assertArrayEquals(new int[]{245, 235, 225, 128}, px(out, 0, 0));
    }

    @Test
    void invertExtremes() {
        PixelContainer out = ImagePointOps.invert(single(0, 255, 128, 255));
        assertArrayEquals(new int[]{255, 0, 127, 255}, px(out, 0, 0));
    }

    @Test
    void invertPreservesDimensions() {
        PixelContainer src = PixelOps.create(4, 3);
        PixelContainer out = ImagePointOps.invert(src);
        assertEquals(4, out.width);
        assertEquals(3, out.height);
    }

    @Test
    void thresholdBelow() {
        PixelContainer out = ImagePointOps.threshold(single(50, 50, 50, 255), 128);
        assertArrayEquals(new int[]{0, 0, 0, 255}, px(out, 0, 0));
    }

    @Test
    void thresholdAbove() {
        PixelContainer out = ImagePointOps.threshold(single(200, 200, 200, 255), 128);
        assertArrayEquals(new int[]{255, 255, 255, 255}, px(out, 0, 0));
    }

    @Test
    void thresholdLuminanceIsPerceptual() {
        // Pure green is brighter perceptually (Y=0.7152*255≈182) than mean-equivalent.
        PixelContainer out = ImagePointOps.thresholdLuminance(single(0, 255, 0, 255), 150);
        assertArrayEquals(new int[]{255, 255, 255, 255}, px(out, 0, 0));
    }

    @Test
    void thresholdLuminanceBlueFails() {
        // Pure blue has very low luminance (Y=0.0722*255≈18).
        PixelContainer out = ImagePointOps.thresholdLuminance(single(0, 0, 255, 255), 50);
        assertArrayEquals(new int[]{0, 0, 0, 255}, px(out, 0, 0));
    }

    @Test
    void posterizeTwoLevels() {
        PixelContainer out = ImagePointOps.posterize(single(100, 200, 10, 255), 2);
        // step = 255, so each channel rounds to 0 or 255
        int[] p = px(out, 0, 0);
        for (int i = 0; i < 3; i++) assertTrue(p[i] == 0 || p[i] == 255);
    }

    @Test
    void posterizeIdentityAt256Levels() {
        PixelContainer out = ImagePointOps.posterize(single(42, 142, 242, 255), 256);
        assertArrayEquals(new int[]{42, 142, 242, 255}, px(out, 0, 0));
    }

    @Test
    void posterizeMinimumLevels() {
        // levels < 2 should clamp to 2.
        PixelContainer out = ImagePointOps.posterize(single(100, 200, 10, 255), 1);
        int[] p = px(out, 0, 0);
        for (int i = 0; i < 3; i++) assertTrue(p[i] == 0 || p[i] == 255);
    }

    @Test
    void swapRgbBgr() {
        PixelContainer out = ImagePointOps.swapRgbBgr(single(1, 2, 3, 4));
        assertArrayEquals(new int[]{3, 2, 1, 4}, px(out, 0, 0));
    }

    @Test
    void extractChannelRed() {
        PixelContainer out = ImagePointOps.extractChannel(single(10, 20, 30, 40), 0);
        assertArrayEquals(new int[]{10, 10, 10, 255}, px(out, 0, 0));
    }

    @Test
    void extractChannelGreen() {
        PixelContainer out = ImagePointOps.extractChannel(single(10, 20, 30, 40), 1);
        assertArrayEquals(new int[]{20, 20, 20, 255}, px(out, 0, 0));
    }

    @Test
    void extractChannelBlue() {
        PixelContainer out = ImagePointOps.extractChannel(single(10, 20, 30, 40), 2);
        assertArrayEquals(new int[]{30, 30, 30, 255}, px(out, 0, 0));
    }

    @Test
    void extractChannelAlpha() {
        PixelContainer out = ImagePointOps.extractChannel(single(10, 20, 30, 40), 3);
        assertArrayEquals(new int[]{40, 40, 40, 255}, px(out, 0, 0));
    }

    @Test
    void extractChannelInvalidReturnsZero() {
        PixelContainer out = ImagePointOps.extractChannel(single(10, 20, 30, 40), 9);
        assertArrayEquals(new int[]{0, 0, 0, 255}, px(out, 0, 0));
    }

    @Test
    void brightnessPositiveClamps() {
        PixelContainer out = ImagePointOps.brightness(single(200, 100, 50, 255), 100);
        assertArrayEquals(new int[]{255, 200, 150, 255}, px(out, 0, 0));
    }

    @Test
    void brightnessNegativeClamps() {
        PixelContainer out = ImagePointOps.brightness(single(200, 100, 50, 255), -100);
        assertArrayEquals(new int[]{100, 0, 0, 255}, px(out, 0, 0));
    }

    /* ---------------- linear-light ---------------- */

    @Test
    void contrastIdentityAtZero() {
        PixelContainer out = ImagePointOps.contrast(single(100, 128, 200, 255), 0);
        int[] p = px(out, 0, 0);
        // f=1 so output equals input for each channel.
        assertEquals(100, p[0]);
        assertEquals(128, p[1]);
        assertEquals(200, p[2]);
    }

    @Test
    void contrastIncreaseStretchesAwayFrom128() {
        PixelContainer out = ImagePointOps.contrast(single(100, 128, 200, 255), 100);
        int[] p = px(out, 0, 0);
        assertTrue(p[0] < 100);   // darks get darker
        assertEquals(128, p[1]);  // midpoint unchanged
        assertTrue(p[2] > 200);   // brights get brighter
    }

    @Test
    void gammaIdentity() {
        PixelContainer out = ImagePointOps.gamma(single(100, 150, 200, 255), 1.0);
        int[] p = px(out, 0, 0);
        for (int i = 0; i < 3; i++) assertTrue(Math.abs(p[i] - new int[]{100,150,200,255}[i]) <= 1);
    }

    @Test
    void gammaDarkenMidtones() {
        PixelContainer out = ImagePointOps.gamma(single(128, 128, 128, 255), 2.0);
        // g>1 in linear space darkens midtones.
        assertTrue(px(out, 0, 0)[0] < 128);
    }

    @Test
    void exposureZeroIsIdentity() {
        PixelContainer out = ImagePointOps.exposure(single(100, 150, 200, 255), 0);
        int[] p = px(out, 0, 0);
        for (int i = 0; i < 3; i++) assertTrue(Math.abs(p[i] - new int[]{100,150,200,255}[i]) <= 1);
    }

    @Test
    void exposurePositiveBrightens() {
        PixelContainer out = ImagePointOps.exposure(single(50, 50, 50, 255), 1.0);
        assertTrue(px(out, 0, 0)[0] > 50);
    }

    @Test
    void exposurePositiveClamps() {
        PixelContainer out = ImagePointOps.exposure(single(200, 200, 200, 255), 5.0);
        assertArrayEquals(new int[]{255, 255, 255, 255}, px(out, 0, 0));
    }

    @Test
    void greyscaleRec709() {
        // Pure red: Y = 0.2126, encoded back ~ small value.
        PixelContainer out = ImagePointOps.greyscale(single(255, 0, 0, 255), GreyscaleMethod.REC709);
        int[] p = px(out, 0, 0);
        assertEquals(p[0], p[1]);
        assertEquals(p[1], p[2]);
        assertTrue(p[0] > 50 && p[0] < 170);
    }

    @Test
    void greyscaleBt601DifferentFromRec709() {
        PixelContainer r1 = ImagePointOps.greyscale(single(255, 0, 0, 255), GreyscaleMethod.REC709);
        PixelContainer r2 = ImagePointOps.greyscale(single(255, 0, 0, 255), GreyscaleMethod.BT601);
        assertNotEquals(px(r1, 0, 0)[0], px(r2, 0, 0)[0]);
    }

    @Test
    void greyscaleAverage() {
        PixelContainer out = ImagePointOps.greyscale(single(255, 255, 255, 255), GreyscaleMethod.AVERAGE);
        assertEquals(255, px(out, 0, 0)[0]);
    }

    @Test
    void sepiaWhiteBecomesWarm() {
        PixelContainer out = ImagePointOps.sepia(single(255, 255, 255, 255));
        int[] p = px(out, 0, 0);
        // sepia matrix rows sum: r=1.351, g=1.203, b=0.937 → R clamps, G clamps, B doesn't.
        assertEquals(255, p[0]);
        assertEquals(255, p[1]);
        assertTrue(p[2] < 255);
    }

    @Test
    void colourMatrixIdentity() {
        double[][] id = { {1,0,0}, {0,1,0}, {0,0,1} };
        PixelContainer out = ImagePointOps.colourMatrix(single(100, 150, 200, 255), id);
        int[] p = px(out, 0, 0);
        for (int i = 0; i < 3; i++) assertTrue(Math.abs(p[i] - new int[]{100,150,200,255}[i]) <= 1);
    }

    @Test
    void saturateZeroGivesGrey() {
        PixelContainer out = ImagePointOps.saturate(single(200, 50, 10, 255), 0.0);
        int[] p = px(out, 0, 0);
        assertEquals(p[0], p[1]);
        assertEquals(p[1], p[2]);
    }

    @Test
    void saturateIdentityAtOne() {
        PixelContainer out = ImagePointOps.saturate(single(100, 150, 200, 255), 1.0);
        int[] p = px(out, 0, 0);
        for (int i = 0; i < 3; i++) assertTrue(Math.abs(p[i] - new int[]{100,150,200,255}[i]) <= 1);
    }

    @Test
    void hueRotate360IsIdentity() {
        PixelContainer out = ImagePointOps.hueRotate(single(200, 50, 10, 255), 360);
        int[] p = px(out, 0, 0);
        for (int i = 0; i < 3; i++) assertTrue(Math.abs(p[i] - new int[]{200,50,10,255}[i]) <= 2);
    }

    @Test
    void hueRotate180InvertsHue() {
        // Red rotated 180 → cyan-ish.
        PixelContainer out = ImagePointOps.hueRotate(single(255, 0, 0, 255), 180);
        int[] p = px(out, 0, 0);
        assertTrue(p[0] < p[1] || p[0] < p[2]);
    }

    @Test
    void hueRotateNegative() {
        // -120 should wrap to 240 and produce same result as +240.
        PixelContainer a = ImagePointOps.hueRotate(single(255, 0, 0, 255), -120);
        PixelContainer b = ImagePointOps.hueRotate(single(255, 0, 0, 255), 240);
        int[] pa = px(a, 0, 0), pb = px(b, 0, 0);
        for (int i = 0; i < 3; i++) assertTrue(Math.abs(pa[i] - pb[i]) <= 2);
    }

    @Test
    void hueRotateGreyscaleUnchanged() {
        // Saturation 0 → hue rotation should not change the colour.
        PixelContainer out = ImagePointOps.hueRotate(single(128, 128, 128, 255), 90);
        int[] p = px(out, 0, 0);
        assertEquals(p[0], p[1]);
        assertEquals(p[1], p[2]);
    }

    /* ---------------- whole-image sRGB ↔ linear ---------------- */

    @Test
    void srgbToLinearRoundTripApproximate() {
        PixelContainer src = single(100, 150, 200, 255);
        PixelContainer lin = ImagePointOps.srgbToLinearImage(src);
        PixelContainer back = ImagePointOps.linearToSrgbImage(lin);
        int[] p = px(back, 0, 0);
        // Quantisation through 8-bit midtones loses a little precision.
        for (int i = 0; i < 3; i++) assertTrue(Math.abs(p[i] - new int[]{100,150,200,255}[i]) <= 3);
    }

    /* ---------------- LUTs ---------------- */

    @Test
    void applyLut1dU8Identity() {
        byte[] lut = new byte[256];
        for (int i = 0; i < 256; i++) lut[i] = (byte) i;
        PixelContainer out = ImagePointOps.applyLut1dU8(single(10, 20, 30, 40), lut, lut, lut);
        assertArrayEquals(new int[]{10, 20, 30, 40}, px(out, 0, 0));
    }

    @Test
    void applyLut1dU8Inverts() {
        byte[] lut = new byte[256];
        for (int i = 0; i < 256; i++) lut[i] = (byte) (255 - i);
        PixelContainer out = ImagePointOps.applyLut1dU8(single(10, 20, 30, 40), lut, lut, lut);
        assertArrayEquals(new int[]{245, 235, 225, 40}, px(out, 0, 0));
    }

    @Test
    void buildLut1dU8Monotone() {
        byte[] lut = ImagePointOps.buildLut1dU8(x -> x); // identity in linear
        // Identity through decode/encode should approximately round-trip.
        for (int i = 0; i < 256; i++) {
            int v = lut[i] & 0xFF;
            assertTrue(Math.abs(v - i) <= 1, "LUT[" + i + "] off: " + v);
        }
    }

    @Test
    void buildGammaLutIsDecreasing() {
        byte[] lut = ImagePointOps.buildGammaLut(2.0);
        // Gamma 2.0 in linear darkens, so LUT values should mostly be below input.
        int below = 0;
        for (int i = 1; i < 256; i++) if ((lut[i] & 0xFF) <= i) below++;
        assertTrue(below > 200);
    }

    /* ---------------- multi-pixel correctness ---------------- */

    @Test
    void appliesToEveryPixel() {
        PixelContainer src = PixelOps.create(3, 2);
        for (int y = 0; y < 2; y++)
            for (int x = 0; x < 3; x++)
                PixelOps.setPixel(src, x, y, 10, 20, 30, 255);
        PixelContainer out = ImagePointOps.invert(src);
        for (int y = 0; y < 2; y++)
            for (int x = 0; x < 3; x++)
                assertArrayEquals(new int[]{245, 235, 225, 255}, px(out, x, y));
    }

    @Test
    void doesNotMutateInput() {
        PixelContainer src = single(10, 20, 30, 40);
        ImagePointOps.invert(src);
        assertArrayEquals(new int[]{10, 20, 30, 40}, px(src, 0, 0));
    }
}
