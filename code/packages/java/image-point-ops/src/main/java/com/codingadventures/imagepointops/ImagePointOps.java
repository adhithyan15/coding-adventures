package com.codingadventures.imagepointops;

import com.codingadventures.pixelcontainer.PixelContainer;
import com.codingadventures.pixelcontainer.PixelOps;

import java.util.function.DoubleUnaryOperator;

/**
 * IMG03 — Per-pixel point operations on {@link PixelContainer}.
 *
 * <p>Every method in this class is a <em>point operation</em>: it produces
 * an output pixel from the corresponding input pixel alone, using no
 * neighbourhood information. That makes all of them data-parallel and
 * trivially vectorisable, and allows them to compose freely.</p>
 *
 * <h2>Two domains</h2>
 * <ul>
 *   <li><b>u8-domain</b>: {@link #invert}, {@link #threshold}, posterize,
 *       channel ops, {@link #brightness}. These operate on the sRGB byte
 *       directly.</li>
 *   <li><b>Linear-light domain</b>: {@link #contrast}, {@link #gamma},
 *       {@link #exposure}, {@link #greyscale}, {@link #sepia},
 *       {@link #colourMatrix}, {@link #saturate}, {@link #hueRotate}.
 *       These decode sRGB to linear light, operate, then re-encode.</li>
 * </ul>
 *
 * <h2>sRGB ↔ linear</h2>
 * <pre>
 *   decode: c = byte/255; c <= 0.04045 ? c/12.92 : ((c+0.055)/1.055)^2.4
 *   encode: c <= 0.0031308 ? c*12.92 : 1.055*c^(1/2.4)-0.055; round*255
 * </pre>
 *
 * <p>A 256-entry LUT handles {@code decode} because there are only 256
 * possible inputs.</p>
 */
public final class ImagePointOps {
    private ImagePointOps() {}

    /* ------------------------------------------------------------------ */
    /* sRGB ↔ linear plumbing.                                            */
    /* ------------------------------------------------------------------ */

    /**
     * Precomputed sRGB-byte → linear-float lookup table. Filled at class
     * load because there are exactly 256 possible inputs and the per-call
     * pow() is far more expensive than the array access.
     */
    private static final double[] SRGB_TO_LINEAR = new double[256];
    static {
        for (int i = 0; i < 256; i++) {
            double c = i / 255.0;
            SRGB_TO_LINEAR[i] = (c <= 0.04045) ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
        }
    }

    /** Decode an sRGB byte to linear-light [0, 1]. */
    private static double decode(int b) {
        return SRGB_TO_LINEAR[b & 0xFF];
    }

    /** Encode a linear-light value in [0, 1] back to an sRGB byte. */
    private static int encode(double linear) {
        double c = Math.max(0.0, Math.min(1.0, linear));
        double srgb = (c <= 0.0031308) ? c * 12.92 : 1.055 * Math.pow(c, 1.0 / 2.4) - 0.055;
        return (int) Math.round(srgb * 255.0);
    }

    /**
     * Functional interface used to describe the per-pixel transform.
     * Kept package-private — callers use the named ops.
     */
    @FunctionalInterface
    interface PixelMapper {
        int[] map(int r, int g, int b, int a);
    }

    /**
     * Apply {@code f} independently to every pixel of {@code src},
     * producing a new container. Input is never mutated.
     */
    private static PixelContainer mapPixels(PixelContainer src, PixelMapper f) {
        PixelContainer out = new PixelContainer(src.width, src.height);
        for (int y = 0; y < src.height; y++) {
            for (int x = 0; x < src.width; x++) {
                int[] px = PixelOps.pixelAt(src, x, y);
                int[] outPx = f.map(px[0], px[1], px[2], px[3]);
                PixelOps.setPixel(out, x, y, outPx[0], outPx[1], outPx[2], outPx[3]);
            }
        }
        return out;
    }

    /** Clamp to an integer in [0, 255]. */
    private static int clampByte(int v) {
        return v < 0 ? 0 : (v > 255 ? 255 : v);
    }

    private static int clampByte(double v) {
        if (v <= 0) return 0;
        if (v >= 255) return 255;
        return (int) Math.round(v);
    }

    /* ------------------------------------------------------------------ */
    /* u8-domain ops.                                                     */
    /* ------------------------------------------------------------------ */

    /**
     * Photographic negative: {@code (255-r, 255-g, 255-b, a)}. Alpha is
     * untouched because transparency is not a colour.
     */
    public static PixelContainer invert(PixelContainer src) {
        return mapPixels(src, (r, g, b, a) -> new int[]{255 - r, 255 - g, 255 - b, a});
    }

    /**
     * Binary threshold on the arithmetic mean of RGB. Pixels with a mean
     * {@code >= t} become white, others black. Alpha preserved.
     *
     * <p>This uses the mean rather than perceptual luminance because it's
     * the classical "Otsu-style" binarisation — see
     * {@link #thresholdLuminance} for the perceptual version.</p>
     */
    public static PixelContainer threshold(PixelContainer src, int t) {
        return mapPixels(src, (r, g, b, a) -> {
            int avg = (r + g + b) / 3;
            int v = (avg >= t) ? 255 : 0;
            return new int[]{v, v, v, a};
        });
    }

    /**
     * Binary threshold on Rec. 709 luminance. More perceptually uniform
     * than plain mean threshold.
     */
    public static PixelContainer thresholdLuminance(PixelContainer src, int t) {
        return mapPixels(src, (r, g, b, a) -> {
            double y = 0.2126 * r + 0.7152 * g + 0.0722 * b;
            int v = (y >= t) ? 255 : 0;
            return new int[]{v, v, v, a};
        });
    }

    /**
     * Posterize — reduce each channel to {@code levels} evenly spaced
     * values. With {@code levels = 2} you get a hard black/white per
     * channel; with {@code levels = 256} you get the identity.
     *
     * <p>For each channel value v in [0, 255]:
     * {@code step = 255 / (levels-1); out = round(round(v/step) * step)}.</p>
     */
    public static PixelContainer posterize(PixelContainer src, int levels) {
        if (levels < 2) levels = 2;
        final double step = 255.0 / (levels - 1);
        return mapPixels(src, (r, g, b, a) -> new int[]{
            (int) Math.round(Math.round(r / step) * step),
            (int) Math.round(Math.round(g / step) * step),
            (int) Math.round(Math.round(b / step) * step),
            a
        });
    }

    /** Swap red and blue channels. Handy for BGR ↔ RGB codec interop. */
    public static PixelContainer swapRgbBgr(PixelContainer src) {
        return mapPixels(src, (r, g, b, a) -> new int[]{b, g, r, a});
    }

    /**
     * Extract channel {@code ch} (0=R, 1=G, 2=B, 3=A) into a greyscale
     * RGBA image. Alpha is forced to 255 so the result is visible even
     * when extracting the alpha channel itself.
     */
    public static PixelContainer extractChannel(PixelContainer src, int ch) {
        return mapPixels(src, (r, g, b, a) -> {
            int v = switch (ch) {
                case 0 -> r;
                case 1 -> g;
                case 2 -> b;
                case 3 -> a;
                default -> 0;
            };
            return new int[]{v, v, v, 255};
        });
    }

    /**
     * Add {@code delta} to each colour channel in u8 space, clamping to
     * [0, 255]. This is the naive brightness adjustment — for perceptually
     * correct brightening use {@link #exposure}.
     */
    public static PixelContainer brightness(PixelContainer src, int delta) {
        return mapPixels(src, (r, g, b, a) -> new int[]{
            clampByte(r + delta),
            clampByte(g + delta),
            clampByte(b + delta),
            a
        });
    }

    /* ------------------------------------------------------------------ */
    /* Linear-light domain ops.                                           */
    /* ------------------------------------------------------------------ */

    /**
     * Classic "GIMP-style" contrast in u8 space around midpoint 128.
     *
     * <p>{@code factor} ∈ [-255, 255]: 0 = identity, positive stretches,
     * negative flattens. The formula {@code f = 259*(factor+255) /
     * (255*(259-factor))} was popularised by GIMP and produces the
     * familiar "increase contrast" slider behaviour.</p>
     */
    public static PixelContainer contrast(PixelContainer src, double factor) {
        final double f = (259.0 * (factor + 255.0)) / (255.0 * (259.0 - factor));
        return mapPixels(src, (r, g, b, a) -> new int[]{
            clampByte(f * (r - 128) + 128),
            clampByte(f * (g - 128) + 128),
            clampByte(f * (b - 128) + 128),
            a
        });
    }

    /**
     * Raise the linear-light signal to a power. {@code g > 1} darkens
     * midtones, {@code g < 1} brightens them.
     */
    public static PixelContainer gamma(PixelContainer src, double g) {
        return mapPixels(src, (r, gg, b, a) -> new int[]{
            encode(Math.pow(decode(r), g)),
            encode(Math.pow(decode(gg), g)),
            encode(Math.pow(decode(b), g)),
            a
        });
    }

    /**
     * Exposure compensation in f-stops. Each stop doubles (or halves) the
     * linear-light intensity. +1 stop = photograph taken with 2× the light.
     */
    public static PixelContainer exposure(PixelContainer src, double stops) {
        final double scale = Math.pow(2.0, stops);
        return mapPixels(src, (r, g, b, a) -> new int[]{
            encode(decode(r) * scale),
            encode(decode(g) * scale),
            encode(decode(b) * scale),
            a
        });
    }

    /**
     * Desaturate to greyscale using the chosen weights. All work is done
     * in linear light so mixing is physically correct.
     */
    public static PixelContainer greyscale(PixelContainer src, GreyscaleMethod method) {
        final double wr, wg, wb;
        switch (method) {
            case REC709  -> { wr = 0.2126; wg = 0.7152; wb = 0.0722; }
            case BT601   -> { wr = 0.299;  wg = 0.587;  wb = 0.114;  }
            case AVERAGE -> { wr = 1.0/3;  wg = 1.0/3;  wb = 1.0/3;  }
            default      -> { wr = 1.0/3;  wg = 1.0/3;  wb = 1.0/3;  }
        }
        return mapPixels(src, (r, g, b, a) -> {
            double y = wr * decode(r) + wg * decode(g) + wb * decode(b);
            int v = encode(y);
            return new int[]{v, v, v, a};
        });
    }

    /**
     * Classic sepia tone matrix, applied in linear light.
     * <pre>
     *   rOut = 0.393*r + 0.769*g + 0.189*b
     *   gOut = 0.349*r + 0.686*g + 0.168*b
     *   bOut = 0.272*r + 0.534*g + 0.131*b
     * </pre>
     */
    public static PixelContainer sepia(PixelContainer src) {
        return mapPixels(src, (r, g, b, a) -> {
            double lr = decode(r), lg = decode(g), lb = decode(b);
            double outR = 0.393 * lr + 0.769 * lg + 0.189 * lb;
            double outG = 0.349 * lr + 0.686 * lg + 0.168 * lb;
            double outB = 0.272 * lr + 0.534 * lg + 0.131 * lb;
            return new int[]{encode(outR), encode(outG), encode(outB), a};
        });
    }

    /**
     * Apply an arbitrary 3×3 colour matrix to every pixel's linear RGB.
     * Extremely general: greyscale, sepia, channel mixing, and many
     * artistic LUTs can all be expressed as 3×3 matrices.
     */
    public static PixelContainer colourMatrix(PixelContainer src, double[][] m) {
        return mapPixels(src, (r, g, b, a) -> {
            double lr = decode(r), lg = decode(g), lb = decode(b);
            double outR = m[0][0] * lr + m[0][1] * lg + m[0][2] * lb;
            double outG = m[1][0] * lr + m[1][1] * lg + m[1][2] * lb;
            double outB = m[2][0] * lr + m[2][1] * lg + m[2][2] * lb;
            return new int[]{encode(outR), encode(outG), encode(outB), a};
        });
    }

    /**
     * Boost ({@code factor > 1}) or cut ({@code factor < 1}) saturation,
     * blending each channel toward its luminance.
     *
     * <p>Implemented as a per-channel lerp toward Y with {@code factor}.
     * {@code factor = 0} fully desaturates (greyscale); {@code factor = 1}
     * is identity; {@code factor = 2} doubles chroma.</p>
     */
    public static PixelContainer saturate(PixelContainer src, double factor) {
        return mapPixels(src, (r, g, b, a) -> {
            double lr = decode(r), lg = decode(g), lb = decode(b);
            double y = 0.2126 * lr + 0.7152 * lg + 0.0722 * lb;
            // lerp(Y, c, factor) == Y + factor*(c - Y)
            double outR = y + factor * (lr - y);
            double outG = y + factor * (lg - y);
            double outB = y + factor * (lb - y);
            return new int[]{encode(outR), encode(outG), encode(outB), a};
        });
    }

    /**
     * Rotate hue by {@code degrees} degrees on the HSV hue circle.
     * Conversion and arithmetic happen in linear light.
     */
    public static PixelContainer hueRotate(PixelContainer src, double degrees) {
        return mapPixels(src, (r, g, b, a) -> {
            double[] hsv = rgbToHsv(decode(r), decode(g), decode(b));
            double h = hsv[0] + degrees;
            h = ((h % 360) + 360) % 360;   // wrap to [0, 360)
            double[] rgb = hsvToRgb(h, hsv[1], hsv[2]);
            return new int[]{encode(rgb[0]), encode(rgb[1]), encode(rgb[2]), a};
        });
    }

    /** Linear-light RGB (0..1) → HSV with H in degrees [0,360), S and V in [0,1]. */
    private static double[] rgbToHsv(double r, double g, double b) {
        double cmax = Math.max(r, Math.max(g, b));
        double cmin = Math.min(r, Math.min(g, b));
        double delta = cmax - cmin;
        double h = 0;
        if (delta > 1e-6) {
            if (cmax == r)      h = 60 * (((g - b) / delta) % 6);
            else if (cmax == g) h = 60 * ((b - r) / delta + 2);
            else                h = 60 * ((r - g) / delta + 4);
        }
        if (h < 0) h += 360;
        double s = (cmax < 1e-6) ? 0 : delta / cmax;
        return new double[]{h, s, cmax};
    }

    /** HSV (H in degrees, S/V in [0,1]) → linear-light RGB in [0,1]. */
    private static double[] hsvToRgb(double h, double s, double v) {
        double c = v * s;
        double x = c * (1 - Math.abs((h / 60.0) % 2 - 1));
        double m = v - c;
        double r1, g1, b1;
        int sector = (int) (h / 60) % 6;
        if (sector < 0) sector += 6;
        switch (sector) {
            case 0 -> { r1 = c; g1 = x; b1 = 0; }
            case 1 -> { r1 = x; g1 = c; b1 = 0; }
            case 2 -> { r1 = 0; g1 = c; b1 = x; }
            case 3 -> { r1 = 0; g1 = x; b1 = c; }
            case 4 -> { r1 = x; g1 = 0; b1 = c; }
            default -> { r1 = c; g1 = 0; b1 = x; }
        }
        return new double[]{
            Math.max(0, Math.min(1, r1 + m)),
            Math.max(0, Math.min(1, g1 + m)),
            Math.max(0, Math.min(1, b1 + m))
        };
    }

    /* ------------------------------------------------------------------ */
    /* Whole-image sRGB ↔ linear conversion.                              */
    /* ------------------------------------------------------------------ */

    /**
     * Reinterpret each sRGB byte as if it were linear-light — i.e. the
     * output bytes equal {@code round(decode(input) * 255)}. Useful when
     * feeding a pipeline that expects linear bytes.
     */
    public static PixelContainer srgbToLinearImage(PixelContainer src) {
        return mapPixels(src, (r, g, b, a) -> new int[]{
            (int) Math.round(decode(r) * 255.0),
            (int) Math.round(decode(g) * 255.0),
            (int) Math.round(decode(b) * 255.0),
            a
        });
    }

    /**
     * Inverse of {@link #srgbToLinearImage}: treat each byte as linear
     * light in [0, 1] and re-encode to sRGB.
     */
    public static PixelContainer linearToSrgbImage(PixelContainer src) {
        return mapPixels(src, (r, g, b, a) -> new int[]{
            encode(r / 255.0),
            encode(g / 255.0),
            encode(b / 255.0),
            a
        });
    }

    /* ------------------------------------------------------------------ */
    /* 1-D LUTs.                                                           */
    /* ------------------------------------------------------------------ */

    /**
     * Apply a per-channel 1-D LUT. Each byte of R, G, B is replaced by
     * the corresponding LUT entry. Alpha is untouched.
     *
     * <p>LUTs must be 256 bytes each.</p>
     */
    public static PixelContainer applyLut1dU8(PixelContainer src, byte[] lutR, byte[] lutG, byte[] lutB) {
        if (lutR.length < 256 || lutG.length < 256 || lutB.length < 256)
            throw new IllegalArgumentException("Each LUT must have at least 256 entries");
        return mapPixels(src, (r, g, b, a) -> new int[]{
            lutR[r] & 0xFF,
            lutG[g] & 0xFF,
            lutB[b] & 0xFF,
            a
        });
    }

    /**
     * Build a 256-entry LUT that applies {@code f} in linear light.
     * {@code lut[i] = encode(f(decode(i)))}. Handy for precomputing any
     * monotone tonal curve and then applying it cheaply.
     */
    public static byte[] buildLut1dU8(DoubleUnaryOperator f) {
        byte[] lut = new byte[256];
        for (int i = 0; i < 256; i++) {
            double y = f.applyAsDouble(decode(i));
            lut[i] = (byte) encode(y);
        }
        return lut;
    }

    /** Convenience: build a LUT for {@code x -> x^g}, i.e. a gamma curve. */
    public static byte[] buildGammaLut(double g) {
        return buildLut1dU8(x -> Math.pow(x, g));
    }
}
