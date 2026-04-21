using CodingAdventures.PixelContainer;
using PxContainer = global::CodingAdventures.PixelContainer.PixelContainer;

namespace CodingAdventures.ImageGeometricTransforms;

/// <summary>
/// Which interpolation kernel to use when a transform samples between
/// integer pixel coordinates.
///
/// <list type="bullet">
/// <item><b>Nearest</b> — pick the closest pixel. Fastest, blockiest.
/// Ideal for pixel-art upscales where you want to preserve hard edges.</item>
/// <item><b>Bilinear</b> — weighted sum of the four enclosing pixels.
/// Smooth, cheap, slightly blurry. The go-to general-purpose kernel.</item>
/// <item><b>Bicubic</b> — Catmull-Rom over a 4×4 neighbourhood. Sharper
/// than bilinear, can slightly overshoot into negatives or &gt;1
/// (we clamp on encode). The default when quality matters more than speed.</item>
/// </list>
/// </summary>
public enum Interpolation { Nearest, Bilinear, Bicubic }

/// <summary>
/// How <see cref="ImageGeometricTransforms.Rotate"/> chooses the output canvas.
/// Fit expands the canvas so no pixel is clipped; Crop keeps the original
/// dimensions and lets corners fall outside.
/// </summary>
public enum RotateBounds { Fit, Crop }

/// <summary>
/// What to do when a sample coordinate falls outside the source image.
///
/// <list type="bullet">
/// <item><b>Zero</b> — return (0,0,0,0). Simple, always safe.</item>
/// <item><b>Replicate</b> — clamp to the nearest edge pixel. Prevents dark
/// haloes on the border; the default for rotate-fit previews.</item>
/// <item><b>Reflect</b> — mirror at the edge. Tile-able kernels prefer this.</item>
/// <item><b>Wrap</b> — modulo. Texture-atlas / seamless-tile semantics.</item>
/// </list>
/// </summary>
public enum OutOfBounds { Zero, Replicate, Reflect, Wrap }

/// <summary>
/// IMG04 — Geometric transforms on PixelContainer.
///
/// Every transform works by <i>inverse warping</i>: for each pixel in the
/// destination, we compute where it came from in the source and sample
/// there. This avoids the gaps and overlaps that a forward scatter would
/// produce.
///
/// <h2>Sampling in linear light</h2>
/// All continuous transforms (Scale, Rotate, Translate, Affine,
/// PerspectiveWarp) decode the four / sixteen neighbours to linear light
/// before blending, then re-encode the result. Bilinear blending of sRGB
/// bytes would darken grey gradients and produce wrong hues on saturated
/// colours, so we pay the pow() cost for physically correct interpolation.
///
/// <h2>Lossless vs continuous</h2>
/// Flips and 90°/180° rotations are <i>lossless</i>: they just copy pixels
/// to new coordinates. Crop is the same — a pure memcpy per row. Scale,
/// rotate-by-degrees, translate, affine, and perspective all need
/// interpolation and are therefore <i>continuous</i>.
///
/// <h2>No dependency on image-point-ops</h2>
/// We duplicate the sRGB LUT rather than take a package dependency — the
/// two packages are siblings, not stacked. If you need LUT-based point-ops
/// as well, add both packages to your project.
/// </summary>
public static class ImageGeometricTransforms
{
    /// <summary>
    /// 256-entry sRGB→linear LUT (independent copy — no dep on image-point-ops).
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

    private static double Decode(byte b) => SrgbToLinear[b];

    private static byte Encode(double linear)
    {
        double c = Math.Clamp(linear, 0.0, 1.0);
        double s = c <= 0.0031308 ? c * 12.92 : 1.055 * Math.Pow(c, 1.0 / 2.4) - 0.055;
        return (byte)Math.Round(s * 255.0);
    }

    // ────────────────────────────────────────────────────────────────
    // Out-of-bounds policy
    // ────────────────────────────────────────────────────────────────

    /// <summary>
    /// Map a potentially out-of-range integer coordinate to an in-range
    /// index, or return −1 to signal "use the zero pixel". The returned
    /// index is guaranteed to be in [0, maxVal) unless the sentinel is used.
    /// </summary>
    private static int ResolveCoord(int x, int maxVal, OutOfBounds oob)
    {
        if (x >= 0 && x < maxVal) return x;
        return oob switch
        {
            OutOfBounds.Zero => -1,
            OutOfBounds.Replicate => Math.Clamp(x, 0, maxVal - 1),
            OutOfBounds.Reflect => ReflectCoord(x, maxVal),
            OutOfBounds.Wrap => WrapCoord(x, maxVal),
            _ => -1
        };
    }

    /// <summary>
    /// Mirror at the edges with period 2*maxVal. Index maxVal maps back
    /// to maxVal-1, index -1 maps to 0, etc.
    /// </summary>
    private static int ReflectCoord(int x, int maxVal)
    {
        if (maxVal <= 0) return 0;
        int period = checked(2 * maxVal);
        int xm = x % period;
        if (xm < 0) xm += period;
        return xm >= maxVal ? period - xm - 1 : xm;
    }

    /// <summary>Modulo with negative-safe wraparound.</summary>
    private static int WrapCoord(int x, int maxVal)
    {
        if (maxVal <= 0) return 0;
        int xm = x % maxVal;
        return xm < 0 ? xm + maxVal : xm;
    }

    /// <summary>
    /// Fetch a pixel applying the OOB policy. Returns the default (0,0,0,0)
    /// when the policy is Zero and the coordinate is outside.
    /// </summary>
    private static Rgba GetPixelOob(PxContainer src, int x, int y, OutOfBounds oob)
    {
        int xi = ResolveCoord(x, src.Width, oob);
        int yi = ResolveCoord(y, src.Height, oob);
        if (xi < 0 || yi < 0) return default;
        return src.GetPixel(xi, yi);
    }

    // ────────────────────────────────────────────────────────────────
    // Interpolation kernels
    // ────────────────────────────────────────────────────────────────

    /// <summary>
    /// Catmull-Rom cubic kernel. Smooth, C¹, passes through sample points.
    /// The classic "bicubic" kernel for image resampling.
    /// </summary>
    private static double CatmullRom(double d)
    {
        d = Math.Abs(d);
        if (d < 1.0) return 1.5 * d * d * d - 2.5 * d * d + 1.0;
        if (d < 2.0) return -0.5 * d * d * d + 2.5 * d * d - 4.0 * d + 2.0;
        return 0.0;
    }

    private static Rgba SampleNearest(PxContainer src, double u, double v, OutOfBounds oob)
    {
        int xi = ResolveCoord((int)Math.Round(u), src.Width, oob);
        int yi = ResolveCoord((int)Math.Round(v), src.Height, oob);
        if (xi < 0 || yi < 0) return default;
        return src.GetPixel(xi, yi);
    }

    private static Rgba SampleBilinear(PxContainer src, double u, double v, OutOfBounds oob)
    {
        int x0 = (int)Math.Floor(u), x1 = x0 + 1;
        int y0 = (int)Math.Floor(v), y1 = y0 + 1;
        double fx = u - x0, fy = v - y0;
        var p00 = GetPixelOob(src, x0, y0, oob);
        var p10 = GetPixelOob(src, x1, y0, oob);
        var p01 = GetPixelOob(src, x0, y1, oob);
        var p11 = GetPixelOob(src, x1, y1, oob);
        static double Lerp(double a, double b, double t) => a + t * (b - a);
        // Blend RGB in linear light; alpha stays in u8 space because alpha
        // has no sRGB gamma — it's already a linear coverage value.
        byte R = Encode(Lerp(Lerp(Decode(p00.R), Decode(p10.R), fx), Lerp(Decode(p01.R), Decode(p11.R), fx), fy));
        byte G = Encode(Lerp(Lerp(Decode(p00.G), Decode(p10.G), fx), Lerp(Decode(p01.G), Decode(p11.G), fx), fy));
        byte B = Encode(Lerp(Lerp(Decode(p00.B), Decode(p10.B), fx), Lerp(Decode(p01.B), Decode(p11.B), fx), fy));
        byte A = (byte)Math.Round(Lerp(Lerp(p00.A, p10.A, fx), Lerp(p01.A, p11.A, fx), fy));
        return new Rgba(R, G, B, A);
    }

    private static Rgba SampleBicubic(PxContainer src, double u, double v, OutOfBounds oob)
    {
        int x0 = (int)Math.Floor(u), y0 = (int)Math.Floor(v);
        double fx = u - x0, fy = v - y0;
        double[] wx = new double[4], wy = new double[4];
        for (int k = 0; k < 4; k++)
        {
            wx[k] = CatmullRom(fx - (k - 1));
            wy[k] = CatmullRom(fy - (k - 1));
        }
        double accR = 0, accG = 0, accB = 0, accA = 0;
        for (int ky = 0; ky < 4; ky++)
        {
            for (int kx = 0; kx < 4; kx++)
            {
                var px = GetPixelOob(src, x0 - 1 + kx, y0 - 1 + ky, oob);
                double w = wx[kx] * wy[ky];
                accR += Decode(px.R) * w;
                accG += Decode(px.G) * w;
                accB += Decode(px.B) * w;
                accA += px.A * w;
            }
        }
        return new Rgba(
            Encode(accR), Encode(accG), Encode(accB),
            (byte)Math.Round(Math.Clamp(accA, 0, 255)));
    }

    /// <summary>
    /// Dispatch to the right kernel based on the requested interpolation.
    /// </summary>
    private static Rgba SamplePixel(PxContainer src, double u, double v, Interpolation interp, OutOfBounds oob) =>
        interp switch
        {
            Interpolation.Nearest => SampleNearest(src, u, v, oob),
            Interpolation.Bilinear => SampleBilinear(src, u, v, oob),
            Interpolation.Bicubic => SampleBicubic(src, u, v, oob),
            _ => SampleNearest(src, u, v, oob)
        };

    // ────────────────────────────────────────────────────────────────
    // Lossless transforms — pure pixel relocations, no sampling needed.
    // ────────────────────────────────────────────────────────────────

    /// <summary>Mirror left-right: out[x,y] = src[width-1-x, y].</summary>
    public static PxContainer FlipHorizontal(PxContainer src)
    {
        var outImg = PixelContainers.Create(src.Width, src.Height);
        for (int y = 0; y < src.Height; y++)
            for (int x = 0; x < src.Width; x++)
                outImg.SetPixel(x, y, src.GetPixel(src.Width - 1 - x, y));
        return outImg;
    }

    /// <summary>Mirror top-bottom: out[x,y] = src[x, height-1-y].</summary>
    public static PxContainer FlipVertical(PxContainer src)
    {
        var outImg = PixelContainers.Create(src.Width, src.Height);
        for (int y = 0; y < src.Height; y++)
            for (int x = 0; x < src.Width; x++)
                outImg.SetPixel(x, y, src.GetPixel(x, src.Height - 1 - y));
        return outImg;
    }

    /// <summary>
    /// Rotate 90° clockwise. Output is (H × W). Inverse warp:
    /// srcX = y', srcY = src.Height − 1 − x'.
    /// </summary>
    public static PxContainer Rotate90CW(PxContainer src)
    {
        var outImg = PixelContainers.Create(src.Height, src.Width);
        for (int y = 0; y < outImg.Height; y++)
            for (int x = 0; x < outImg.Width; x++)
                outImg.SetPixel(x, y, src.GetPixel(y, src.Height - 1 - x));
        return outImg;
    }

    /// <summary>
    /// Rotate 90° counter-clockwise. Output is (H × W). Inverse warp:
    /// srcX = src.Width − 1 − y', srcY = x'.
    /// </summary>
    public static PxContainer Rotate90CCW(PxContainer src)
    {
        var outImg = PixelContainers.Create(src.Height, src.Width);
        for (int y = 0; y < outImg.Height; y++)
            for (int x = 0; x < outImg.Width; x++)
                outImg.SetPixel(x, y, src.GetPixel(src.Width - 1 - y, x));
        return outImg;
    }

    /// <summary>Rotate 180°: srcX = W − 1 − x', srcY = H − 1 − y'.</summary>
    public static PxContainer Rotate180(PxContainer src)
    {
        var outImg = PixelContainers.Create(src.Width, src.Height);
        for (int y = 0; y < src.Height; y++)
            for (int x = 0; x < src.Width; x++)
                outImg.SetPixel(x, y, src.GetPixel(src.Width - 1 - x, src.Height - 1 - y));
        return outImg;
    }

    /// <summary>
    /// Extract a (w × h) rectangle starting at (x0, y0). Coordinates outside
    /// the source become transparent black. This is forgiving by design —
    /// callers can pass any rectangle and get a well-sized output.
    /// </summary>
    public static PxContainer Crop(PxContainer src, int x0, int y0, int w, int h)
    {
        if (w < 0) throw new ArgumentException("width must be non-negative", nameof(w));
        if (h < 0) throw new ArgumentException("height must be non-negative", nameof(h));
        var outImg = PixelContainers.Create(w, h);
        for (int y = 0; y < h; y++)
        {
            for (int x = 0; x < w; x++)
            {
                int sx = x0 + x, sy = y0 + y;
                if (sx >= 0 && sx < src.Width && sy >= 0 && sy < src.Height)
                    outImg.SetPixel(x, y, src.GetPixel(sx, sy));
                // else leave as default (0,0,0,0)
            }
        }
        return outImg;
    }

    // ────────────────────────────────────────────────────────────────
    // Continuous transforms — need interpolation.
    // ────────────────────────────────────────────────────────────────

    /// <summary>
    /// Resample to (outW × outH) using the pixel-centre convention.
    ///
    /// The "+0.5" / "−0.5" dance aligns the centres of output pixels with
    /// the centres of the corresponding source regions, rather than their
    /// top-left corners. This avoids the half-pixel drift that naïve
    /// <c>u = x * src.Width / outW</c> would introduce.
    /// </summary>
    public static PxContainer Scale(PxContainer src, int outW, int outH, Interpolation interp, OutOfBounds oob)
    {
        if (outW <= 0) throw new ArgumentException("outW must be positive", nameof(outW));
        if (outH <= 0) throw new ArgumentException("outH must be positive", nameof(outH));
        var outImg = PixelContainers.Create(outW, outH);
        double sx = (double)outW / src.Width;
        double sy = (double)outH / src.Height;
        for (int y = 0; y < outH; y++)
        {
            for (int x = 0; x < outW; x++)
            {
                double u = (x + 0.5) / sx - 0.5;
                double v = (y + 0.5) / sy - 0.5;
                outImg.SetPixel(x, y, SamplePixel(src, u, v, interp, oob));
            }
        }
        return outImg;
    }

    /// <summary>
    /// Rotate by <paramref name="degrees"/> counter-clockwise around the
    /// image centre.
    ///
    /// <list type="bullet">
    /// <item><b>Fit</b> — expand the canvas to the bounding box so no pixel
    /// is clipped. Output dimensions grow as |W cos θ| + |H sin θ| ×
    /// |H cos θ| + |W sin θ|.</item>
    /// <item><b>Crop</b> — keep the same (W × H) canvas; corners fall outside
    /// and are filled via the OOB policy.</item>
    /// </list>
    ///
    /// Inverse warp matrix around the centre (cx, cy):
    ///   u = cos θ · (x − cxOut) + sin θ · (y − cyOut) + cxIn
    ///   v = −sin θ · (x − cxOut) + cos θ · (y − cyOut) + cyIn
    /// </summary>
    public static PxContainer Rotate(PxContainer src, double degrees, RotateBounds bounds, Interpolation interp, OutOfBounds oob)
    {
        double rad = degrees * Math.PI / 180.0;
        double cosT = Math.Cos(rad);
        double sinT = Math.Sin(rad);
        int inW = src.Width, inH = src.Height, outW, outH;
        if (bounds == RotateBounds.Fit)
        {
            outW = (int)Math.Ceiling(Math.Abs(inW * cosT) + Math.Abs(inH * sinT));
            outH = (int)Math.Ceiling(Math.Abs(inH * cosT) + Math.Abs(inW * sinT));
        }
        else
        {
            outW = inW;
            outH = inH;
        }
        // Guard against pathological canvas sizes — the PixelContainer
        // constructor would throw, but surfacing our own message is friendlier.
        if ((long)outW * outH > int.MaxValue / 4)
            throw new ArgumentException($"Rotated canvas too large: {outW}×{outH}");
        double cxIn = inW / 2.0, cyIn = inH / 2.0;
        double cxOut = outW / 2.0, cyOut = outH / 2.0;
        var outImg = PixelContainers.Create(outW, outH);
        for (int y = 0; y < outH; y++)
        {
            for (int x = 0; x < outW; x++)
            {
                double dx = x - cxOut, dy = y - cyOut;
                double u = cosT * dx + sinT * dy + cxIn;
                double v = -sinT * dx + cosT * dy + cyIn;
                outImg.SetPixel(x, y, SamplePixel(src, u, v, interp, oob));
            }
        }
        return outImg;
    }

    /// <summary>
    /// Translate by (tx, ty). Output has the same dimensions as the source;
    /// revealed area is filled by the OOB policy.
    /// </summary>
    public static PxContainer Translate(PxContainer src, double tx, double ty, Interpolation interp, OutOfBounds oob)
    {
        var outImg = PixelContainers.Create(src.Width, src.Height);
        for (int y = 0; y < src.Height; y++)
        {
            for (int x = 0; x < src.Width; x++)
            {
                outImg.SetPixel(x, y, SamplePixel(src, x - tx, y - ty, interp, oob));
            }
        }
        return outImg;
    }

    /// <summary>
    /// Apply a 2×3 affine forward matrix. The matrix is inverted internally
    /// so the caller can think in "what the transform does" terms rather
    /// than "how to sample backward". Rows are
    /// [a b c] [d e f] meaning x' = a x + b y + c, y' = d x + e y + f.
    /// </summary>
    public static PxContainer Affine(PxContainer src, double[,] m, Interpolation interp, OutOfBounds oob)
    {
        ArgumentNullException.ThrowIfNull(m);
        if (m.GetLength(0) != 2 || m.GetLength(1) != 3)
            throw new ArgumentException("matrix must be 2×3", nameof(m));
        double a = m[0, 0], b = m[0, 1], c = m[0, 2];
        double d = m[1, 0], e = m[1, 1], f = m[1, 2];
        double det = a * e - b * d;
        if (Math.Abs(det) < 1e-12)
            throw new ArgumentException("affine matrix is singular", nameof(m));
        // Analytic inverse of the linear part; the translation is handled
        // separately inside the loop by subtracting (c, f) from (x, y).
        double ia = e / det, ib = -b / det, ic = -d / det, id_ = a / det;
        var outImg = PixelContainers.Create(src.Width, src.Height);
        for (int y = 0; y < src.Height; y++)
        {
            for (int x = 0; x < src.Width; x++)
            {
                double dx = x - c, dy = y - f;
                double u = ia * dx + ib * dy;
                double v = ic * dx + id_ * dy;
                outImg.SetPixel(x, y, SamplePixel(src, u, v, interp, oob));
            }
        }
        return outImg;
    }

    /// <summary>
    /// Apply a 3×3 perspective (homography) matrix. The matrix is inverted
    /// internally. At each output pixel we compute (u·w, v·w, w) in
    /// homogeneous space and divide by w. A tiny w is treated as infinity
    /// (the corresponding output pixel is left transparent).
    /// </summary>
    public static PxContainer PerspectiveWarp(PxContainer src, double[,] m, Interpolation interp, OutOfBounds oob)
    {
        ArgumentNullException.ThrowIfNull(m);
        if (m.GetLength(0) != 3 || m.GetLength(1) != 3)
            throw new ArgumentException("matrix must be 3×3", nameof(m));
        var inv = Invert3x3(m);
        var outImg = PixelContainers.Create(src.Width, src.Height);
        for (int y = 0; y < src.Height; y++)
        {
            for (int x = 0; x < src.Width; x++)
            {
                double uh = inv[0, 0] * x + inv[0, 1] * y + inv[0, 2];
                double vh = inv[1, 0] * x + inv[1, 1] * y + inv[1, 2];
                double wh = inv[2, 0] * x + inv[2, 1] * y + inv[2, 2];
                if (Math.Abs(wh) < 1e-10)
                {
                    outImg.SetPixel(x, y, default);
                    continue;
                }
                outImg.SetPixel(x, y, SamplePixel(src, uh / wh, vh / wh, interp, oob));
            }
        }
        return outImg;
    }

    /// <summary>
    /// Closed-form 3×3 matrix inverse via the cofactor formula. Throws
    /// if the matrix is singular — perspective warps need a well-conditioned
    /// homography.
    /// </summary>
    private static double[,] Invert3x3(double[,] m)
    {
        double a = m[0, 0], b = m[0, 1], c = m[0, 2];
        double d = m[1, 0], e = m[1, 1], f = m[1, 2];
        double g = m[2, 0], h = m[2, 1], ii = m[2, 2];
        double det = a * (e * ii - f * h) - b * (d * ii - f * g) + c * (d * h - e * g);
        if (Math.Abs(det) < 1e-12)
            throw new ArgumentException("perspective matrix is singular", nameof(m));
        return new double[,]
        {
            {  (e * ii - f * h) / det, -(b * ii - c * h) / det,  (b * f - c * e) / det },
            { -(d * ii - f * g) / det,  (a * ii - c * g) / det, -(a * f - c * d) / det },
            {  (d * h  - e * g) / det, -(a * h  - b * g) / det,  (a * e - b * d) / det }
        };
    }
}
