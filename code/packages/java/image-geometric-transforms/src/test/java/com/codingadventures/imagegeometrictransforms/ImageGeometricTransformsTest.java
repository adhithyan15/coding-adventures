package com.codingadventures.imagegeometrictransforms;

import com.codingadventures.pixelcontainer.PixelContainer;
import com.codingadventures.pixelcontainer.PixelOps;
import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.*;

class ImageGeometricTransformsTest {

    /**
     * Build a distinctive test image with unique colours per pixel, so tests
     * can confirm placement instead of just value equality.
     */
    private static PixelContainer makeGrid(int w, int h) {
        PixelContainer c = PixelOps.create(w, h);
        for (int y = 0; y < h; y++)
            for (int x = 0; x < w; x++)
                PixelOps.setPixel(c, x, y, (x * 30) & 0xFF, (y * 30) & 0xFF, ((x + y) * 10) & 0xFF, 255);
        return c;
    }

    private static int[] px(PixelContainer c, int x, int y) {
        return PixelOps.pixelAt(c, x, y);
    }

    /* -------- Flips -------- */

    @Test
    void flipHorizontalRespectsDimensions() {
        PixelContainer src = makeGrid(3, 2);
        PixelContainer out = ImageGeometricTransforms.flipHorizontal(src);
        assertEquals(3, out.width);
        assertEquals(2, out.height);
    }

    @Test
    void flipHorizontalSwapsLeftRight() {
        PixelContainer src = makeGrid(4, 2);
        PixelContainer out = ImageGeometricTransforms.flipHorizontal(src);
        assertArrayEquals(px(src, 0, 0), px(out, 3, 0));
        assertArrayEquals(px(src, 3, 0), px(out, 0, 0));
    }

    @Test
    void flipHorizontalDoubleRestoresInput() {
        PixelContainer src = makeGrid(5, 4);
        PixelContainer twice = ImageGeometricTransforms.flipHorizontal(
            ImageGeometricTransforms.flipHorizontal(src));
        for (int y = 0; y < 4; y++)
            for (int x = 0; x < 5; x++)
                assertArrayEquals(px(src, x, y), px(twice, x, y));
    }

    @Test
    void flipVerticalSwapsTopBottom() {
        PixelContainer src = makeGrid(2, 4);
        PixelContainer out = ImageGeometricTransforms.flipVertical(src);
        assertArrayEquals(px(src, 0, 0), px(out, 0, 3));
        assertArrayEquals(px(src, 0, 3), px(out, 0, 0));
    }

    @Test
    void flipVerticalDoubleRestoresInput() {
        PixelContainer src = makeGrid(3, 3);
        PixelContainer twice = ImageGeometricTransforms.flipVertical(
            ImageGeometricTransforms.flipVertical(src));
        for (int y = 0; y < 3; y++)
            for (int x = 0; x < 3; x++)
                assertArrayEquals(px(src, x, y), px(twice, x, y));
    }

    /* -------- 90 rotations -------- */

    @Test
    void rotate90CwSwapsDimensions() {
        PixelContainer src = makeGrid(4, 2);
        PixelContainer out = ImageGeometricTransforms.rotate90CW(src);
        assertEquals(2, out.width);
        assertEquals(4, out.height);
    }

    @Test
    void rotate90CwTopLeftGoesToTopRight() {
        PixelContainer src = makeGrid(3, 2);
        PixelContainer out = ImageGeometricTransforms.rotate90CW(src);
        // (0,0) in src → (out.width-1, 0) = (1, 0) in out.
        assertArrayEquals(px(src, 0, 0), px(out, 1, 0));
    }

    @Test
    void rotate90CwFourTimesIsIdentity() {
        PixelContainer src = makeGrid(3, 2);
        PixelContainer r = src;
        for (int i = 0; i < 4; i++) r = ImageGeometricTransforms.rotate90CW(r);
        for (int y = 0; y < 2; y++)
            for (int x = 0; x < 3; x++)
                assertArrayEquals(px(src, x, y), px(r, x, y));
    }

    @Test
    void rotate90CcwIsInverseOfCw() {
        PixelContainer src = makeGrid(4, 3);
        PixelContainer back = ImageGeometricTransforms.rotate90CCW(
            ImageGeometricTransforms.rotate90CW(src));
        for (int y = 0; y < 3; y++)
            for (int x = 0; x < 4; x++)
                assertArrayEquals(px(src, x, y), px(back, x, y));
    }

    @Test
    void rotate180SwapsOppositeCorners() {
        PixelContainer src = makeGrid(3, 2);
        PixelContainer out = ImageGeometricTransforms.rotate180(src);
        assertArrayEquals(px(src, 0, 0), px(out, 2, 1));
        assertArrayEquals(px(src, 2, 1), px(out, 0, 0));
    }

    @Test
    void rotate180EqualsTwoCw() {
        PixelContainer src = makeGrid(3, 2);
        PixelContainer a = ImageGeometricTransforms.rotate180(src);
        PixelContainer b = ImageGeometricTransforms.rotate90CW(
            ImageGeometricTransforms.rotate90CW(src));
        for (int y = 0; y < 2; y++)
            for (int x = 0; x < 3; x++)
                assertArrayEquals(px(a, x, y), px(b, x, y));
    }

    /* -------- Crop -------- */

    @Test
    void cropExtractsRegion() {
        PixelContainer src = makeGrid(5, 5);
        PixelContainer out = ImageGeometricTransforms.crop(src, 1, 1, 2, 2);
        assertEquals(2, out.width);
        assertEquals(2, out.height);
        assertArrayEquals(px(src, 1, 1), px(out, 0, 0));
        assertArrayEquals(px(src, 2, 2), px(out, 1, 1));
    }

    @Test
    void cropOutsideGivesTransparent() {
        PixelContainer src = makeGrid(3, 3);
        PixelContainer out = ImageGeometricTransforms.crop(src, 5, 5, 2, 2);
        for (int y = 0; y < 2; y++)
            for (int x = 0; x < 2; x++)
                assertArrayEquals(new int[]{0, 0, 0, 0}, px(out, x, y));
    }

    @Test
    void cropPartiallyOutside() {
        PixelContainer src = makeGrid(3, 3);
        // Region straddles the right edge.
        PixelContainer out = ImageGeometricTransforms.crop(src, 2, 0, 3, 1);
        assertArrayEquals(px(src, 2, 0), px(out, 0, 0));
        assertArrayEquals(new int[]{0, 0, 0, 0}, px(out, 1, 0));
    }

    /* -------- Scale -------- */

    @Test
    void scaleToSameSizeIsApproximatelyIdentity() {
        PixelContainer src = makeGrid(4, 4);
        PixelContainer out = ImageGeometricTransforms.scale(
            src, 4, 4, Interpolation.NEAREST, OutOfBounds.REPLICATE);
        assertEquals(4, out.width);
        assertEquals(4, out.height);
        assertArrayEquals(px(src, 2, 2), px(out, 2, 2));
    }

    @Test
    void scaleDoubleNearest() {
        PixelContainer src = makeGrid(2, 2);
        PixelContainer out = ImageGeometricTransforms.scale(
            src, 4, 4, Interpolation.NEAREST, OutOfBounds.REPLICATE);
        assertEquals(4, out.width);
        assertEquals(4, out.height);
    }

    @Test
    void scaleHalveBilinear() {
        PixelContainer src = makeGrid(4, 4);
        PixelContainer out = ImageGeometricTransforms.scale(
            src, 2, 2, Interpolation.BILINEAR, OutOfBounds.REPLICATE);
        assertEquals(2, out.width);
        assertEquals(2, out.height);
    }

    @Test
    void scaleBicubic() {
        PixelContainer src = makeGrid(4, 4);
        PixelContainer out = ImageGeometricTransforms.scale(
            src, 8, 8, Interpolation.BICUBIC, OutOfBounds.REPLICATE);
        assertEquals(8, out.width);
        assertEquals(8, out.height);
    }

    /* -------- Translate -------- */

    @Test
    void translateZeroIsIdentity() {
        PixelContainer src = makeGrid(3, 3);
        PixelContainer out = ImageGeometricTransforms.translate(
            src, 0, 0, Interpolation.NEAREST, OutOfBounds.ZERO);
        for (int y = 0; y < 3; y++)
            for (int x = 0; x < 3; x++)
                assertArrayEquals(px(src, x, y), px(out, x, y));
    }

    @Test
    void translateShiftRight() {
        PixelContainer src = makeGrid(3, 1);
        PixelContainer out = ImageGeometricTransforms.translate(
            src, 1, 0, Interpolation.NEAREST, OutOfBounds.ZERO);
        // New (1,0) should equal old (0,0).
        assertArrayEquals(px(src, 0, 0), px(out, 1, 0));
    }

    @Test
    void translateOutOfBoundsZero() {
        PixelContainer src = makeGrid(3, 1);
        PixelContainer out = ImageGeometricTransforms.translate(
            src, 5, 0, Interpolation.NEAREST, OutOfBounds.ZERO);
        for (int x = 0; x < 3; x++)
            assertArrayEquals(new int[]{0, 0, 0, 0}, px(out, x, 0));
    }

    /* -------- Rotate free -------- */

    @Test
    void rotateZeroIsIdentity() {
        PixelContainer src = makeGrid(3, 3);
        PixelContainer out = ImageGeometricTransforms.rotate(
            src, 0, RotateBounds.CROP, Interpolation.NEAREST, OutOfBounds.ZERO);
        assertEquals(3, out.width);
        assertEquals(3, out.height);
        assertArrayEquals(px(src, 1, 1), px(out, 1, 1));
    }

    @Test
    void rotateFitGrowsCanvas() {
        PixelContainer src = makeGrid(4, 4);
        PixelContainer out = ImageGeometricTransforms.rotate(
            src, 45, RotateBounds.FIT, Interpolation.BILINEAR, OutOfBounds.ZERO);
        assertTrue(out.width >= 4);
        assertTrue(out.height >= 4);
    }

    @Test
    void rotateCropKeepsDimensions() {
        PixelContainer src = makeGrid(4, 4);
        PixelContainer out = ImageGeometricTransforms.rotate(
            src, 30, RotateBounds.CROP, Interpolation.BILINEAR, OutOfBounds.ZERO);
        assertEquals(4, out.width);
        assertEquals(4, out.height);
    }

    @Test
    void rotate360IsApproximatelyIdentity() {
        PixelContainer src = makeGrid(5, 5);
        PixelContainer out = ImageGeometricTransforms.rotate(
            src, 360, RotateBounds.CROP, Interpolation.BILINEAR, OutOfBounds.REPLICATE);
        // Centre pixel should be very close to the original.
        int[] a = px(src, 2, 2), b = px(out, 2, 2);
        for (int i = 0; i < 3; i++) assertTrue(Math.abs(a[i] - b[i]) <= 5);
    }

    /* -------- Affine -------- */

    @Test
    void affineIdentity() {
        PixelContainer src = makeGrid(3, 3);
        double[][] id = { {1, 0, 0}, {0, 1, 0} };
        PixelContainer out = ImageGeometricTransforms.affine(
            src, id, Interpolation.NEAREST, OutOfBounds.ZERO);
        for (int y = 0; y < 3; y++)
            for (int x = 0; x < 3; x++)
                assertArrayEquals(px(src, x, y), px(out, x, y));
    }

    @Test
    void affineTranslate() {
        PixelContainer src = makeGrid(3, 1);
        double[][] t = { {1, 0, 1}, {0, 1, 0} };
        PixelContainer out = ImageGeometricTransforms.affine(
            src, t, Interpolation.NEAREST, OutOfBounds.ZERO);
        // Forward shift by +1: output (1,0) should equal input (0,0).
        assertArrayEquals(px(src, 0, 0), px(out, 1, 0));
    }

    @Test
    void affineScale() {
        PixelContainer src = makeGrid(4, 4);
        double[][] s = { {2, 0, 0}, {0, 2, 0} };
        PixelContainer out = ImageGeometricTransforms.affine(
            src, s, Interpolation.NEAREST, OutOfBounds.REPLICATE);
        // Output canvas size equals input; scale>1 forward → input (1,1) maps to output (2,2).
        assertArrayEquals(px(src, 1, 1), px(out, 2, 2));
    }

    /* -------- Perspective -------- */

    @Test
    void perspectiveIdentity() {
        PixelContainer src = makeGrid(3, 3);
        double[][] id = { {1, 0, 0}, {0, 1, 0}, {0, 0, 1} };
        PixelContainer out = ImageGeometricTransforms.perspectiveWarp(
            src, id, Interpolation.NEAREST, OutOfBounds.ZERO);
        for (int y = 0; y < 3; y++)
            for (int x = 0; x < 3; x++)
                assertArrayEquals(px(src, x, y), px(out, x, y));
    }

    @Test
    void perspectiveDegenerateGivesZero() {
        PixelContainer src = makeGrid(3, 3);
        // Singular matrix with a row of zeros produces wh ≈ 0 for some pixels.
        // We use a highly skewed matrix to force near-zero w.
        double[][] m = { {1, 0, 0}, {0, 1, 0}, {1e12, 0, 1} };
        PixelContainer out = ImageGeometricTransforms.perspectiveWarp(
            src, m, Interpolation.NEAREST, OutOfBounds.ZERO);
        // Just sanity: output dimensions preserved and no crash.
        assertEquals(3, out.width);
        assertEquals(3, out.height);
    }

    /* -------- Out-of-bounds modes -------- */

    @Test
    void oobZeroGivesTransparent() {
        PixelContainer src = makeGrid(2, 2);
        PixelContainer out = ImageGeometricTransforms.translate(
            src, 10, 0, Interpolation.NEAREST, OutOfBounds.ZERO);
        assertArrayEquals(new int[]{0, 0, 0, 0}, px(out, 0, 0));
    }

    @Test
    void oobReplicateGivesEdge() {
        PixelContainer src = makeGrid(2, 2);
        PixelContainer out = ImageGeometricTransforms.translate(
            src, 10, 0, Interpolation.NEAREST, OutOfBounds.REPLICATE);
        // Translate +10 means output(x,y) samples source(x-10, y); far to the
        // left of the source. REPLICATE clamps to column 0.
        assertArrayEquals(px(src, 0, 0), px(out, 0, 0));
    }

    @Test
    void oobWrapTiles() {
        PixelContainer src = makeGrid(2, 1);
        PixelContainer out = ImageGeometricTransforms.translate(
            src, -2, 0, Interpolation.NEAREST, OutOfBounds.WRAP);
        // Shift by -2 wraps back to the same image.
        for (int x = 0; x < 2; x++)
            assertArrayEquals(px(src, x, 0), px(out, x, 0));
    }

    @Test
    void oobReflectMirrors() {
        PixelContainer src = makeGrid(3, 1);
        // Sample at x=-1 under REFLECT should fold back to x=0.
        PixelContainer out = ImageGeometricTransforms.translate(
            src, 1, 0, Interpolation.NEAREST, OutOfBounds.REFLECT);
        assertArrayEquals(px(src, 0, 0), px(out, 0, 0));
    }

    /* -------- Interpolation mode coverage -------- */

    @Test
    void allInterpolationModesProduceSameSizedOutput() {
        PixelContainer src = makeGrid(4, 4);
        for (Interpolation i : Interpolation.values()) {
            PixelContainer out = ImageGeometricTransforms.scale(
                src, 6, 6, i, OutOfBounds.REPLICATE);
            assertEquals(6, out.width);
            assertEquals(6, out.height);
        }
    }

    @Test
    void allOobModesWorkInBicubic() {
        PixelContainer src = makeGrid(3, 3);
        for (OutOfBounds o : OutOfBounds.values()) {
            PixelContainer out = ImageGeometricTransforms.scale(
                src, 5, 5, Interpolation.BICUBIC, o);
            assertEquals(5, out.width);
            assertEquals(5, out.height);
        }
    }

    /* -------- Alpha behaviour -------- */

    @Test
    void bilinearAlphaBlends() {
        PixelContainer src = PixelOps.create(2, 1);
        PixelOps.setPixel(src, 0, 0, 255, 0, 0, 0);
        PixelOps.setPixel(src, 1, 0, 255, 0, 0, 255);
        // Scale up so we sample between the two columns.
        PixelContainer out = ImageGeometricTransforms.scale(
            src, 4, 1, Interpolation.BILINEAR, OutOfBounds.REPLICATE);
        int midAlpha = px(out, 2, 0)[3];
        assertTrue(midAlpha > 0 && midAlpha < 255);
    }

    @Test
    void doesNotMutateInput() {
        PixelContainer src = makeGrid(3, 3);
        int[] before = px(src, 1, 1).clone();
        ImageGeometricTransforms.rotate180(src);
        ImageGeometricTransforms.flipHorizontal(src);
        ImageGeometricTransforms.scale(src, 2, 2, Interpolation.BILINEAR, OutOfBounds.ZERO);
        assertArrayEquals(before, px(src, 1, 1));
    }
}
