using CodingAdventures.PixelContainer;
using PxContainer = global::CodingAdventures.PixelContainer.PixelContainer;

namespace CodingAdventures.ImagePointOps;

/// <summary>
/// IMG03 — Per-pixel point operations on PixelContainer.
///
/// Every method in this class transforms each pixel independently using only
/// that pixel's value — no neighbourhood, no frequency domain, no geometry.
/// This is the simplest family of image operations and forms the backbone of
/// colour correction, tonemapping, and simple special effects.
///
/// <h2>Two domains</h2>
/// Some operations live in the u8-domain: they treat raw sRGB bytes as just
/// numbers to flip, threshold, or posterize. Others live in the linear-light
/// domain: they decode sRGB to linear f64, operate, then re-encode. The
/// distinction matters because sRGB is perceptually uniform but physically
/// non-linear — mixing light amounts (gamma, exposure, matrix blends) demands
/// linear arithmetic, while bit-twiddling tricks (invert, threshold, posterize)
/// are happy in u8 space.
///
/// <h2>sRGB transfer function</h2>
/// decode (sRGB → linear): c = byte/255;
///   c ≤ 0.04045 ? c/12.92 : ((c+0.055)/1.055)^2.4
/// encode (linear → sRGB): c' = clamp(c,0,1);
///   c' ≤ 0.0031308 ? c'*12.92 : 1.055*c'^(1/2.4) − 0.055;
///   byte = round(c' * 255)
///
/// <h2>LUT optimisation</h2>
/// Decoding is the hot path for every linear-light op. A 256-entry lookup
/// table built once at static init replaces a branch + pow() per pixel with a
/// single array read. The table is indexed by the raw byte value (0–255).
/// </summary>
public static class ImagePointOps
{
    /// <summary>
    /// 256-entry sRGB→linear LUT built at static init time. Indexed by the
    /// raw byte value (0–255). Static readonly so the JIT can hoist bounds
    /// checks and the CPU can keep it hot in L1.
    /// </summary>
    private static readonly double[] SrgbToLinear = BuildSrgbLut();

    private static double[] BuildSrgbLut()
    {
        var lut = new double[256];
        for (int i = 0; i < 256; i++)
        {
            double c = i / 255.0;
            lut[i] = c <= 0.04045 ? c / 12.92 : Math.Pow((c + 0.055) / 1.055, 2.4);
        }
        return lut;
    }

    /// <summary>sRGB byte → linear [0,1] via LUT.</summary>
    private static double Decode(byte b) => SrgbToLinear[b];

    /// <summary>Linear [0,1] → sRGB byte via the piecewise gamma encode.</summary>
    private static byte Encode(double linear)
    {
        double c = Math.Clamp(linear, 0.0, 1.0);
        double s = c <= 0.0031308 ? c * 12.92 : 1.055 * Math.Pow(c, 1.0 / 2.4) - 0.055;
        return (byte)Math.Round(s * 255.0);
    }

    /// <summary>
    /// The tiny workhorse every point-op ultimately calls: allocate an output
    /// of the same size, then for each input pixel write f(pixel) to the same
    /// coordinate in the output. Point ops don't depend on neighbours, so the
    /// loop order doesn't matter — row-major is just cache-friendly.
    /// </summary>
    private static PxContainer MapPixels(PxContainer src, Func<Rgba, Rgba> f)
    {
        var outImg = PixelContainers.Create(src.Width, src.Height);
        for (int y = 0; y < src.Height; y++)
        {
            for (int x = 0; x < src.Width; x++)
            {
                outImg.SetPixel(x, y, f(src.GetPixel(x, y)));
            }
        }
        return outImg;
    }

    // ────────────────────────────────────────────────────────────────────
    // U8-domain operations — work directly on raw sRGB bytes.
    // These treat the byte values as opaque integers; the non-linearity of
    // sRGB encoding is irrelevant because we're not doing physical blending.
    // ────────────────────────────────────────────────────────────────────

    /// <summary>
    /// Photographic negative. Each RGB channel is replaced with (255 − c);
    /// alpha is preserved because inverting opacity is a different operation.
    /// </summary>
    public static PxContainer Invert(PxContainer src) =>
        MapPixels(src, p => new Rgba((byte)(255 - p.R), (byte)(255 - p.G), (byte)(255 - p.B), p.A));

    /// <summary>
    /// Binary threshold on the simple (R+G+B)/3 average. Pixels whose average
    /// channel is ≥ value become white (255,255,255); the rest become black.
    /// Alpha is preserved. This is the classical luminance-free threshold —
    /// cheaper but less perceptually aligned than ThresholdLuminance.
    /// </summary>
    public static PxContainer Threshold(PxContainer src, byte value) =>
        MapPixels(src, p =>
        {
            int avg = (p.R + p.G + p.B) / 3;
            byte c = avg >= value ? (byte)255 : (byte)0;
            return new Rgba(c, c, c, p.A);
        });

    /// <summary>
    /// Binary threshold on Rec. 709 luminance Y = 0.2126 R + 0.7152 G + 0.0722 B.
    /// Weights reflect the relative contribution of each channel to human
    /// perception, so this threshold respects what the eye calls "bright".
    /// </summary>
    public static PxContainer ThresholdLuminance(PxContainer src, byte value) =>
        MapPixels(src, p =>
        {
            double y = 0.2126 * p.R + 0.7152 * p.G + 0.0722 * p.B;
            byte c = y >= value ? (byte)255 : (byte)0;
            return new Rgba(c, c, c, p.A);
        });

    /// <summary>
    /// Reduce the tonal range to a fixed number of equally spaced levels.
    /// step = 255/(levels−1); q(v) = round(round(v/step)*step). With levels=2
    /// this gives a pure black-and-white image; with levels=4 a four-tone
    /// poster. Alpha is preserved.
    /// </summary>
    public static PxContainer Posterize(PxContainer src, byte levels)
    {
        if (levels < 2) throw new ArgumentException("levels must be ≥ 2", nameof(levels));
        double step = 255.0 / (levels - 1);
        byte Q(byte v) => (byte)Math.Round(Math.Round(v / step) * step);
        return MapPixels(src, p => new Rgba(Q(p.R), Q(p.G), Q(p.B), p.A));
    }

    /// <summary>
    /// Swap the R and B channels — the classic RGB↔BGR fix for broken codec
    /// byte orders. Green and alpha stay put.
    /// </summary>
    public static PxContainer SwapRgbBgr(PxContainer src) =>
        MapPixels(src, p => new Rgba(p.B, p.G, p.R, p.A));

    /// <summary>
    /// Extract a single channel as a greyscale image with full opacity.
    /// channel 0=R, 1=G, 2=B, 3=A. The extracted value fills R, G, and B;
    /// alpha is forced to 255 so the result is visible regardless of source
    /// transparency.
    /// </summary>
    public static PxContainer ExtractChannel(PxContainer src, int channel)
    {
        if (channel < 0 || channel > 3)
            throw new ArgumentException("channel must be 0..3", nameof(channel));
        return MapPixels(src, p =>
        {
            byte v = channel switch { 0 => p.R, 1 => p.G, 2 => p.B, _ => p.A };
            return new Rgba(v, v, v, 255);
        });
    }

    /// <summary>
    /// Additive brightness in u8 space. delta is added to each RGB channel
    /// and clamped to [0,255]. Alpha is preserved. This is intentionally a
    /// u8-domain op because "add a constant to every pixel" is how most
    /// graphics pipelines describe brightness controls to users.
    /// </summary>
    public static PxContainer Brightness(PxContainer src, int delta)
    {
        static byte Clamp(int v) => (byte)Math.Clamp(v, 0, 255);
        return MapPixels(src, p =>
            new Rgba(Clamp(p.R + delta), Clamp(p.G + delta), Clamp(p.B + delta), p.A));
    }

    /// <summary>
    /// Classic u8-domain contrast using the photographer's formula:
    /// f = (259·(factor·255 + 255)) / (255·(259 − factor·255))
    /// adj = clamp(f·(c − 128) + 128, 0, 255)
    /// factor=0 is a no-op; positive values increase contrast around 128.
    /// </summary>
    public static PxContainer Contrast(PxContainer src, double factor)
    {
        double denom = 259.0 - factor * 255.0;
        if (Math.Abs(denom) < 1e-9)
            throw new ArgumentOutOfRangeException(nameof(factor),
                "factor causes a singular (divide-by-zero) contrast formula");
        double f = 259.0 * (factor * 255.0 + 255.0) / (255.0 * denom);
        byte Adj(byte c) => (byte)Math.Clamp(Math.Round(f * (c - 128) + 128), 0, 255);
        return MapPixels(src, p => new Rgba(Adj(p.R), Adj(p.G), Adj(p.B), p.A));
    }

    // ────────────────────────────────────────────────────────────────────
    // Linear-light operations — decode sRGB first, operate, re-encode.
    // These physically correspond to manipulating light amounts, so they
    // composite, mix, and scale the way real photons do.
    // ────────────────────────────────────────────────────────────────────

    /// <summary>
    /// Power-law tonemap in linear light: decode, raise to g, encode.
    /// g &lt; 1 brightens midtones; g &gt; 1 darkens them. This is NOT the
    /// sRGB encode curve — it's an additional creative gamma layered on top.
    /// </summary>
    public static PxContainer Gamma(PxContainer src, double g)
    {
        if (g <= 0)
            throw new ArgumentOutOfRangeException(nameof(g), "gamma must be positive (> 0)");
        return MapPixels(src, p =>
        {
            double r = Math.Pow(Decode(p.R), g);
            double gc = Math.Pow(Decode(p.G), g);
            double b = Math.Pow(Decode(p.B), g);
            return new Rgba(Encode(r), Encode(gc), Encode(b), p.A);
        });
    }

    /// <summary>
    /// Exposure in stops: linear *= 2^stops. +1 stop doubles the light;
    /// −1 stop halves it. Clamped to [0,1] in linear space before re-encode.
    /// </summary>
    public static PxContainer Exposure(PxContainer src, double stops)
    {
        double scale = Math.Pow(2.0, stops);
        return MapPixels(src, p =>
        {
            double r = Decode(p.R) * scale;
            double g = Decode(p.G) * scale;
            double b = Decode(p.B) * scale;
            return new Rgba(Encode(r), Encode(g), Encode(b), p.A);
        });
    }

    /// <summary>
    /// How to weight R, G, B when collapsing to greyscale.
    /// Rec709 is the modern HDTV standard (0.2126/0.7152/0.0722).
    /// Bt601 is the older NTSC/PAL standard (0.299/0.587/0.114).
    /// Average is unweighted (1/3 each) — useful for physics, bad for eyes.
    /// </summary>
    public enum GreyscaleMethod { Rec709, Bt601, Average }

    /// <summary>
    /// Collapse colour to luma in linear light. Weights come from the chosen
    /// standard; the weighted sum of linear RGB is re-encoded and broadcast
    /// to all three channels. Alpha is preserved.
    /// </summary>
    public static PxContainer Greyscale(PxContainer src, GreyscaleMethod method = GreyscaleMethod.Rec709)
    {
        (double wr, double wg, double wb) = method switch
        {
            GreyscaleMethod.Rec709 => (0.2126, 0.7152, 0.0722),
            GreyscaleMethod.Bt601 => (0.299, 0.587, 0.114),
            GreyscaleMethod.Average => (1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0),
            _ => (0.2126, 0.7152, 0.0722)
        };
        return MapPixels(src, p =>
        {
            double y = wr * Decode(p.R) + wg * Decode(p.G) + wb * Decode(p.B);
            byte v = Encode(y);
            return new Rgba(v, v, v, p.A);
        });
    }

    /// <summary>
    /// Microsoft's classic sepia matrix applied in linear light. The mix
    /// leans warm brown regardless of input hue — the signature "old photo"
    /// look. Alpha is preserved.
    /// </summary>
    public static PxContainer Sepia(PxContainer src) =>
        MapPixels(src, p =>
        {
            double r = Decode(p.R), g = Decode(p.G), b = Decode(p.B);
            double rOut = 0.393 * r + 0.769 * g + 0.189 * b;
            double gOut = 0.349 * r + 0.686 * g + 0.168 * b;
            double bOut = 0.272 * r + 0.534 * g + 0.131 * b;
            return new Rgba(Encode(rOut), Encode(gOut), Encode(bOut), p.A);
        });

    /// <summary>
    /// Apply an arbitrary 3×3 colour matrix in linear light. Rows are
    /// output channels, columns are input channels. out[i] = Σ m[i,j]·in[j].
    /// Alpha is preserved.
    /// </summary>
    /// <exception cref="ArgumentException">Matrix must be 3×3.</exception>
    public static PxContainer ColourMatrix(PxContainer src, double[,] m)
    {
        ArgumentNullException.ThrowIfNull(m);
        if (m.GetLength(0) != 3 || m.GetLength(1) != 3)
            throw new ArgumentException("matrix must be 3×3", nameof(m));
        return MapPixels(src, p =>
        {
            double r = Decode(p.R), g = Decode(p.G), b = Decode(p.B);
            double rOut = m[0, 0] * r + m[0, 1] * g + m[0, 2] * b;
            double gOut = m[1, 0] * r + m[1, 1] * g + m[1, 2] * b;
            double bOut = m[2, 0] * r + m[2, 1] * g + m[2, 2] * b;
            return new Rgba(Encode(rOut), Encode(gOut), Encode(bOut), p.A);
        });
    }

    /// <summary>
    /// Pull each channel toward or away from luminance in linear light.
    /// factor=1 is a no-op; factor=0 collapses to greyscale; factor=2 doubles
    /// distance from Y (over-saturated). lerp(Y, channel, factor) is the core.
    /// </summary>
    public static PxContainer Saturate(PxContainer src, double factor) =>
        MapPixels(src, p =>
        {
            double r = Decode(p.R), g = Decode(p.G), b = Decode(p.B);
            double y = 0.2126 * r + 0.7152 * g + 0.0722 * b;
            double rOut = y + (r - y) * factor;
            double gOut = y + (g - y) * factor;
            double bOut = y + (b - y) * factor;
            return new Rgba(Encode(rOut), Encode(gOut), Encode(bOut), p.A);
        });

    /// <summary>
    /// Rotate hue by degrees in HSV (computed from linear RGB). Saturation
    /// and value are preserved. This is a perceptual hue shift, not a simple
    /// channel rotation.
    /// </summary>
    public static PxContainer HueRotate(PxContainer src, double degrees) =>
        MapPixels(src, p =>
        {
            double r = Decode(p.R), g = Decode(p.G), b = Decode(p.B);
            var (h, s, v) = RgbToHsv(r, g, b);
            h = (h + degrees) % 360.0;
            if (h < 0) h += 360.0;
            var (r2, g2, b2) = HsvToRgb(h, s, v);
            return new Rgba(Encode(r2), Encode(g2), Encode(b2), p.A);
        });

    /// <summary>
    /// Standard RGB→HSV, operating on normalised [0,1] RGB. H is in [0,360),
    /// S and V in [0,1]. Handles the achromatic case (delta≈0) by picking H=0.
    /// </summary>
    private static (double H, double S, double V) RgbToHsv(double r, double g, double b)
    {
        double cmax = Math.Max(r, Math.Max(g, b));
        double cmin = Math.Min(r, Math.Min(g, b));
        double delta = cmax - cmin;
        double h = 0;
        if (delta > 1e-6)
        {
            if (cmax == r) h = 60 * (((g - b) / delta) % 6);
            else if (cmax == g) h = 60 * ((b - r) / delta + 2);
            else h = 60 * ((r - g) / delta + 4);
        }
        if (h < 0) h += 360;
        double s = cmax < 1e-6 ? 0 : delta / cmax;
        return (h, s, cmax);
    }

    /// <summary>
    /// Standard HSV→RGB using the chroma decomposition. Clamps to [0,1] on
    /// output to absorb any floating-point drift around the sector boundaries.
    /// </summary>
    private static (double R, double G, double B) HsvToRgb(double h, double s, double v)
    {
        double c = v * s;
        double x = c * (1 - Math.Abs((h / 60.0) % 2 - 1));
        double m = v - c;
        double r1, g1, b1;
        int sector = (int)(h / 60) % 6;
        (r1, g1, b1) = sector switch
        {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x)
        };
        return (Math.Clamp(r1 + m, 0, 1), Math.Clamp(g1 + m, 0, 1), Math.Clamp(b1 + m, 0, 1));
    }

    /// <summary>
    /// Reinterpret each RGB byte as an sRGB-encoded value, decode it to
    /// linear, then re-encode the linear value as a byte (multiply by 255).
    /// Use this to produce "linear-byte" images for debugging — they look
    /// darker because the low end of sRGB stretches into a wider linear range.
    /// </summary>
    public static PxContainer SrgbToLinearImage(PxContainer src) =>
        MapPixels(src, p =>
        {
            byte r = (byte)Math.Round(Math.Clamp(Decode(p.R), 0, 1) * 255.0);
            byte g = (byte)Math.Round(Math.Clamp(Decode(p.G), 0, 1) * 255.0);
            byte b = (byte)Math.Round(Math.Clamp(Decode(p.B), 0, 1) * 255.0);
            return new Rgba(r, g, b, p.A);
        });

    /// <summary>
    /// Inverse of <see cref="SrgbToLinearImage"/>. Treat each byte as a
    /// linear value in [0,1], apply the sRGB encode curve, return the byte.
    /// </summary>
    public static PxContainer LinearToSrgbImage(PxContainer src) =>
        MapPixels(src, p =>
        {
            double rl = p.R / 255.0;
            double gl = p.G / 255.0;
            double bl = p.B / 255.0;
            return new Rgba(Encode(rl), Encode(gl), Encode(bl), p.A);
        });

    /// <summary>
    /// Apply independent 256-entry u8→u8 LUTs to R, G, B. Alpha is
    /// preserved. This is the fastest way to implement any monotone
    /// per-channel curve — tone curves, gamma, colour grading — because
    /// each pixel becomes three array reads.
    /// </summary>
    /// <exception cref="ArgumentException">Each LUT must have 256 entries.</exception>
    public static PxContainer ApplyLut1dU8(PxContainer src, byte[] lutR, byte[] lutG, byte[] lutB)
    {
        ArgumentNullException.ThrowIfNull(lutR);
        ArgumentNullException.ThrowIfNull(lutG);
        ArgumentNullException.ThrowIfNull(lutB);
        if (lutR.Length != 256) throw new ArgumentException("lutR must have 256 entries", nameof(lutR));
        if (lutG.Length != 256) throw new ArgumentException("lutG must have 256 entries", nameof(lutG));
        if (lutB.Length != 256) throw new ArgumentException("lutB must have 256 entries", nameof(lutB));
        return MapPixels(src, p => new Rgba(lutR[p.R], lutG[p.G], lutB[p.B], p.A));
    }

    /// <summary>
    /// Build a u8→u8 LUT from a linear-domain function f: [0,1]→[0,1].
    /// For each byte i: lut[i] = Encode(f(Decode(i))). Use with
    /// <see cref="ApplyLut1dU8"/> to fold any linear-light curve down to a
    /// single array read per channel per pixel.
    /// </summary>
    public static byte[] BuildLut1dU8(Func<double, double> f)
    {
        ArgumentNullException.ThrowIfNull(f);
        var lut = new byte[256];
        for (int i = 0; i < 256; i++)
        {
            lut[i] = Encode(f(Decode((byte)i)));
        }
        return lut;
    }

    /// <summary>
    /// Convenience: build a per-channel power-law (gamma) LUT in linear light.
    /// Equivalent to <c>BuildLut1dU8(x =&gt; Math.Pow(x, g))</c>.
    /// </summary>
    public static byte[] BuildGammaLut(double g) => BuildLut1dU8(x => Math.Pow(x, g));
}
