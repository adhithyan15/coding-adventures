package com.codingadventures.imagegeometrictransforms;

import com.codingadventures.pixelcontainer.PixelContainer;
import com.codingadventures.pixelcontainer.PixelOps;

/**
 * IMG04 — Geometric Transforms on {@link PixelContainer}.
 *
 * <p>Where IMG03 (point ops) changes a pixel's <em>value</em>, IMG04
 * changes a pixel's <em>position</em>. All spatial operations live here:
 * flips, 90° rotations, crop, scale, free rotation, affine warp, and
 * perspective warp.</p>
 *
 * <h2>Lossless vs continuous</h2>
 * <p>Flips, 90° rotations, and crop are pure byte reshuffles — no
 * sampling, no interpolation, no colour-space work. The continuous
 * transforms ({@link #scale}, {@link #rotate}, {@link #translate},
 * {@link #affine}, {@link #perspectiveWarp}) all need to sample at
 * fractional source coordinates, so they pick up an interpolation mode
 * and an out-of-bounds policy.</p>
 *
 * <h2>Inverse warps</h2>
 * <p>Every continuous op walks the <em>output</em> pixels and computes
 * the corresponding source coordinate, then samples. The alternative —
 * forward warping — leaves holes and doubled pixels in the output. The
 * price is that matrix-based transforms must invert their matrix once.</p>
 *
 * <h2>Linear light</h2>
 * <p>Bilinear and bicubic blending is done in linear light for colour
 * channels (decoded from sRGB via a 256-entry LUT). Alpha blends in u8
 * because it carries no perceptual gamma.</p>
 */
public final class ImageGeometricTransforms {
    private ImageGeometricTransforms() {}

    /* ------------------------------------------------------------------ */
    /* sRGB ↔ linear plumbing (duplicated from image-point-ops so this    */
    /* package has no cross-IMG dependency).                              */
    /* ------------------------------------------------------------------ */

    private static final double[] SRGB_TO_LINEAR = new double[256];
    static {
        for (int i = 0; i < 256; i++) {
            double c = i / 255.0;
            SRGB_TO_LINEAR[i] = (c <= 0.04045) ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
        }
    }

    private static double decode(int b) {
        return SRGB_TO_LINEAR[b & 0xFF];
    }

    private static int encode(double linear) {
        double c = Math.max(0.0, Math.min(1.0, linear));
        double s = (c <= 0.0031308) ? c * 12.92 : 1.055 * Math.pow(c, 1.0 / 2.4) - 0.055;
        return (int) Math.round(s * 255);
    }

    /* ------------------------------------------------------------------ */
    /* Out-of-bounds coordinate resolution.                               */
    /* ------------------------------------------------------------------ */

    /**
     * Map an arbitrary integer coordinate to a valid index in [0, maxVal)
     * according to the chosen policy. Returns {@code -1} when the
     * caller should treat the sample as transparent black (the {@code ZERO}
     * policy).
     */
    private static int resolveCoord(int x, int maxVal, OutOfBounds oob) {
        if (x >= 0 && x < maxVal) return x;
        switch (oob) {
            case ZERO: return -1;
            case REPLICATE: return Math.max(0, Math.min(maxVal - 1, x));
            case REFLECT: {
                // period = 2*maxVal; within one period the first half is
                // forward, the second half mirrored.
                int period = 2 * maxVal;
                int xm = x % period;
                if (xm < 0) xm += period;
                return (xm >= maxVal) ? period - xm - 1 : xm;
            }
            case WRAP: {
                int xm = x % maxVal;
                if (xm < 0) xm += maxVal;
                return xm;
            }
            default: return -1;
        }
    }

    /** Fetch a pixel with OOB policy applied; returns {0,0,0,0} when ZERO. */
    private static int[] getPixelOob(PixelContainer src, int x, int y, OutOfBounds oob) {
        int xi = resolveCoord(x, src.width, oob);
        int yi = resolveCoord(y, src.height, oob);
        if (xi < 0 || yi < 0) return new int[]{0, 0, 0, 0};
        return PixelOps.pixelAt(src, xi, yi);
    }

    /* ------------------------------------------------------------------ */
    /* Sampling.                                                          */
    /* ------------------------------------------------------------------ */

    private static int[] samplePixel(PixelContainer src, double u, double v,
                                     Interpolation interp, OutOfBounds oob) {
        switch (interp) {
            case NEAREST:  return sampleNearest(src, u, v, oob);
            case BILINEAR: return sampleBilinear(src, u, v, oob);
            case BICUBIC:  return sampleBicubic(src, u, v, oob);
            default:       return new int[]{0, 0, 0, 0};
        }
    }

    private static int[] sampleNearest(PixelContainer src, double u, double v, OutOfBounds oob) {
        int xi = resolveCoord((int) Math.round(u), src.width, oob);
        int yi = resolveCoord((int) Math.round(v), src.height, oob);
        if (xi < 0 || yi < 0) return new int[]{0, 0, 0, 0};
        return PixelOps.pixelAt(src, xi, yi);
    }

    /**
     * Bilinear sample. Colour channels are decoded to linear light before
     * blending to avoid the classic "muddy midtones" of blending in sRGB.
     * Alpha blends linearly in [0, 255] because it has no perceptual gamma.
     */
    private static int[] sampleBilinear(PixelContainer src, double u, double v, OutOfBounds oob) {
        int x0 = (int) Math.floor(u), x1 = x0 + 1;
        int y0 = (int) Math.floor(v), y1 = y0 + 1;
        double fx = u - x0, fy = v - y0;
        int[] p00 = getPixelOob(src, x0, y0, oob);
        int[] p10 = getPixelOob(src, x1, y0, oob);
        int[] p01 = getPixelOob(src, x0, y1, oob);
        int[] p11 = getPixelOob(src, x1, y1, oob);
        int[] result = new int[4];
        for (int c = 0; c < 3; c++) {
            double v00 = decode(p00[c]), v10 = decode(p10[c]);
            double v01 = decode(p01[c]), v11 = decode(p11[c]);
            double top = v00 + fx * (v10 - v00);
            double bot = v01 + fx * (v11 - v01);
            result[c] = encode(top + fy * (bot - top));
        }
        double a00 = p00[3], a10 = p10[3], a01 = p01[3], a11 = p11[3];
        double atop = a00 + fx * (a10 - a00);
        double abot = a01 + fx * (a11 - a01);
        int a = (int) Math.round(atop + fy * (abot - atop));
        if (a < 0) a = 0;
        if (a > 255) a = 255;
        result[3] = a;
        return result;
    }

    /**
     * Catmull-Rom cubic kernel. Passes through sample points with
     * continuous first derivative, giving sharper results than bilinear
     * without the severity of a sinc.
     */
    private static double catmullRom(double d) {
        d = Math.abs(d);
        if (d < 1.0) return 1.5 * d * d * d - 2.5 * d * d + 1.0;
        if (d < 2.0) return -0.5 * d * d * d + 2.5 * d * d - 4.0 * d + 2.0;
        return 0.0;
    }

    /**
     * Bicubic sample over a 4×4 neighbourhood with separable Catmull-Rom
     * weights. Again, colour channels are blended in linear light.
     */
    private static int[] sampleBicubic(PixelContainer src, double u, double v, OutOfBounds oob) {
        int x0 = (int) Math.floor(u);
        int y0 = (int) Math.floor(v);
        double fx = u - x0, fy = v - y0;
        double[] wx = new double[4];
        double[] wy = new double[4];
        for (int k = 0; k < 4; k++) {
            wx[k] = catmullRom(fx - (k - 1));
            wy[k] = catmullRom(fy - (k - 1));
        }
        double[] acc = new double[4];
        for (int ky = 0; ky < 4; ky++) {
            for (int kx = 0; kx < 4; kx++) {
                int[] pxv = getPixelOob(src, x0 - 1 + kx, y0 - 1 + ky, oob);
                double w = wx[kx] * wy[ky];
                for (int c = 0; c < 3; c++) acc[c] += decode(pxv[c]) * w;
                acc[3] += pxv[3] * w;
            }
        }
        int a = (int) Math.round(acc[3]);
        if (a < 0) a = 0;
        if (a > 255) a = 255;
        return new int[]{encode(acc[0]), encode(acc[1]), encode(acc[2]), a};
    }

    /* ------------------------------------------------------------------ */
    /* Lossless transforms — pure index shuffles, no sampling.            */
    /* ------------------------------------------------------------------ */

    /** Horizontal mirror: output column x reads source column width-1-x. */
    public static PixelContainer flipHorizontal(PixelContainer src) {
        PixelContainer out = new PixelContainer(src.width, src.height);
        for (int y = 0; y < src.height; y++) {
            for (int x = 0; x < src.width; x++) {
                int[] p = PixelOps.pixelAt(src, src.width - 1 - x, y);
                PixelOps.setPixel(out, x, y, p[0], p[1], p[2], p[3]);
            }
        }
        return out;
    }

    /** Vertical mirror: output row y reads source row height-1-y. */
    public static PixelContainer flipVertical(PixelContainer src) {
        PixelContainer out = new PixelContainer(src.width, src.height);
        for (int y = 0; y < src.height; y++) {
            for (int x = 0; x < src.width; x++) {
                int[] p = PixelOps.pixelAt(src, x, src.height - 1 - y);
                PixelOps.setPixel(out, x, y, p[0], p[1], p[2], p[3]);
            }
        }
        return out;
    }

    /**
     * Rotate 90° clockwise. Output dimensions swap: outW=src.height,
     * outH=src.width. The inverse index is
     * {@code srcX = y', srcY = src.height - 1 - x'}.
     */
    public static PixelContainer rotate90CW(PixelContainer src) {
        PixelContainer out = new PixelContainer(src.height, src.width);
        for (int y = 0; y < out.height; y++) {
            for (int x = 0; x < out.width; x++) {
                int[] p = PixelOps.pixelAt(src, y, src.height - 1 - x);
                PixelOps.setPixel(out, x, y, p[0], p[1], p[2], p[3]);
            }
        }
        return out;
    }

    /** Rotate 90° counter-clockwise. Inverse: srcX = src.width-1-y', srcY = x'. */
    public static PixelContainer rotate90CCW(PixelContainer src) {
        PixelContainer out = new PixelContainer(src.height, src.width);
        for (int y = 0; y < out.height; y++) {
            for (int x = 0; x < out.width; x++) {
                int[] p = PixelOps.pixelAt(src, src.width - 1 - y, x);
                PixelOps.setPixel(out, x, y, p[0], p[1], p[2], p[3]);
            }
        }
        return out;
    }

    /** Rotate 180°: (x', y') reads (w-1-x', h-1-y'). Dimensions unchanged. */
    public static PixelContainer rotate180(PixelContainer src) {
        PixelContainer out = new PixelContainer(src.width, src.height);
        for (int y = 0; y < src.height; y++) {
            for (int x = 0; x < src.width; x++) {
                int[] p = PixelOps.pixelAt(src, src.width - 1 - x, src.height - 1 - y);
                PixelOps.setPixel(out, x, y, p[0], p[1], p[2], p[3]);
            }
        }
        return out;
    }

    /**
     * Crop a rectangle starting at {@code (x0, y0)} with size {@code (w, h)}.
     * Coordinates outside the source become transparent black.
     */
    public static PixelContainer crop(PixelContainer src, int x0, int y0, int w, int h) {
        PixelContainer out = new PixelContainer(w, h);
        for (int y = 0; y < h; y++) {
            for (int x = 0; x < w; x++) {
                int sx = x0 + x, sy = y0 + y;
                if (sx >= 0 && sx < src.width && sy >= 0 && sy < src.height) {
                    int[] p = PixelOps.pixelAt(src, sx, sy);
                    PixelOps.setPixel(out, x, y, p[0], p[1], p[2], p[3]);
                }
            }
        }
        return out;
    }

    /* ------------------------------------------------------------------ */
    /* Continuous transforms.                                             */
    /* ------------------------------------------------------------------ */

    /**
     * Scale to {@code outW × outH} using the pixel-centre model — the
     * centre of output pixel (x, y) maps to source coordinate
     * {@code ((x+0.5)/sx - 0.5, (y+0.5)/sy - 0.5)}. The {@code -0.5} shift
     * is what avoids the classic "half-pixel offset" artifact.
     */
    public static PixelContainer scale(PixelContainer src, int outW, int outH,
                                       Interpolation interp, OutOfBounds oob) {
        PixelContainer out = new PixelContainer(outW, outH);
        double sx = (double) outW / src.width;
        double sy = (double) outH / src.height;
        for (int y = 0; y < outH; y++) {
            for (int x = 0; x < outW; x++) {
                double u = (x + 0.5) / sx - 0.5;
                double v = (y + 0.5) / sy - 0.5;
                int[] p = samplePixel(src, u, v, interp, oob);
                PixelOps.setPixel(out, x, y, p[0], p[1], p[2], p[3]);
            }
        }
        return out;
    }

    /**
     * Rotate by {@code degrees} (counter-clockwise).
     *
     * <p>With {@link RotateBounds#FIT} the output canvas is sized to
     * enclose the rotated rectangle; with {@link RotateBounds#CROP} the
     * dimensions are preserved and corners are clipped.</p>
     *
     * <p>The rotation is implemented as an inverse warp: for each output
     * pixel we apply the opposite rotation (CW by the same angle) to map
     * back into source coordinates.</p>
     */
    public static PixelContainer rotate(PixelContainer src, double degrees,
                                        RotateBounds bounds,
                                        Interpolation interp, OutOfBounds oob) {
        double rad = Math.toRadians(degrees);
        double cosT = Math.cos(rad), sinT = Math.sin(rad);
        int inW = src.width, inH = src.height;
        int outW, outH;
        if (bounds == RotateBounds.FIT) {
            outW = (int) Math.ceil(Math.abs(inW * cosT) + Math.abs(inH * sinT));
            outH = (int) Math.ceil(Math.abs(inH * cosT) + Math.abs(inW * sinT));
        } else {
            outW = inW; outH = inH;
        }
        if ((long) outW * outH > Integer.MAX_VALUE / 4)
            throw new IllegalArgumentException("Rotated canvas too large: " + outW + "×" + outH);
        double cxIn = inW / 2.0, cyIn = inH / 2.0;
        double cxOut = outW / 2.0, cyOut = outH / 2.0;
        PixelContainer out = new PixelContainer(outW, outH);
        for (int y = 0; y < outH; y++) {
            for (int x = 0; x < outW; x++) {
                double dx = x - cxOut, dy = y - cyOut;
                // Inverse rotation: CW by the same angle.
                double u =  cosT * dx + sinT * dy + cxIn;
                double v = -sinT * dx + cosT * dy + cyIn;
                int[] p = samplePixel(src, u, v, interp, oob);
                PixelOps.setPixel(out, x, y, p[0], p[1], p[2], p[3]);
            }
        }
        return out;
    }

    /**
     * Translate the image by {@code (tx, ty)}. Dimensions are preserved;
     * uncovered output pixels follow the OOB policy.
     */
    public static PixelContainer translate(PixelContainer src, double tx, double ty,
                                           Interpolation interp, OutOfBounds oob) {
        PixelContainer out = new PixelContainer(src.width, src.height);
        for (int y = 0; y < src.height; y++) {
            for (int x = 0; x < src.width; x++) {
                double u = x - tx, v = y - ty;
                int[] p = samplePixel(src, u, v, interp, oob);
                PixelOps.setPixel(out, x, y, p[0], p[1], p[2], p[3]);
            }
        }
        return out;
    }

    /**
     * Affine warp with a forward 2×3 matrix
     * <pre>
     *   [ m[0][0] m[0][1] m[0][2] ]   [ a b c ]
     *   [ m[1][0] m[1][1] m[1][2] ] = [ d e f ]
     * </pre>
     * The implementation inverts the 2×2 part and uses an inverse warp.
     */
    public static PixelContainer affine(PixelContainer src, double[][] m,
                                        Interpolation interp, OutOfBounds oob) {
        double a = m[0][0], b = m[0][1], c = m[0][2];
        double d = m[1][0], e = m[1][1], f = m[1][2];
        double det = a * e - b * d;
        double ia =  e / det, ib = -b / det;
        double ic = -d / det, id =  a / det;
        int outW = src.width, outH = src.height;
        PixelContainer out = new PixelContainer(outW, outH);
        for (int y = 0; y < outH; y++) {
            for (int x = 0; x < outW; x++) {
                double dx = x - c, dy = y - f;
                double u = ia * dx + ib * dy;
                double v = ic * dx + id * dy;
                int[] p = samplePixel(src, u, v, interp, oob);
                PixelOps.setPixel(out, x, y, p[0], p[1], p[2], p[3]);
            }
        }
        return out;
    }

    /**
     * Perspective warp with a 3×3 forward homography matrix. Each output
     * pixel is unprojected through the matrix inverse, then sampled.
     */
    public static PixelContainer perspectiveWarp(PixelContainer src, double[][] m,
                                                 Interpolation interp, OutOfBounds oob) {
        double[][] inv = invert3x3(m);
        int outW = src.width, outH = src.height;
        PixelContainer out = new PixelContainer(outW, outH);
        for (int y = 0; y < outH; y++) {
            for (int x = 0; x < outW; x++) {
                double uh = inv[0][0] * x + inv[0][1] * y + inv[0][2];
                double vh = inv[1][0] * x + inv[1][1] * y + inv[1][2];
                double wh = inv[2][0] * x + inv[2][1] * y + inv[2][2];
                if (Math.abs(wh) < 1e-10) {
                    PixelOps.setPixel(out, x, y, 0, 0, 0, 0);
                    continue;
                }
                int[] p = samplePixel(src, uh / wh, vh / wh, interp, oob);
                PixelOps.setPixel(out, x, y, p[0], p[1], p[2], p[3]);
            }
        }
        return out;
    }

    /** Straightforward cofactor-based 3×3 inverse. */
    private static double[][] invert3x3(double[][] m) {
        double a = m[0][0], b = m[0][1], c = m[0][2];
        double d = m[1][0], e = m[1][1], f = m[1][2];
        double g = m[2][0], h = m[2][1], ii = m[2][2];
        double det = a * (e * ii - f * h) - b * (d * ii - f * g) + c * (d * h - e * g);
        return new double[][]{
            { (e * ii - f * h) / det, -(b * ii - c * h) / det,  (b * f - c * e) / det },
            {-(d * ii - f * g) / det,  (a * ii - c * g) / det, -(a * f - c * d) / det },
            { (d * h  - e * g) / det, -(a * h  - b * g) / det,  (a * e - b * d) / det }
        };
    }
}
