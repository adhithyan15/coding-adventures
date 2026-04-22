using CodingAdventures.ImagePointOps;
using CodingAdventures.PixelContainer;
using PxContainer = global::CodingAdventures.PixelContainer.PixelContainer;

namespace CodingAdventures.ImagePointOps.Tests;

public class ImagePointOpsTests
{
    /// <summary>Build a 1×1 image containing one pixel — the atomic test fixture.</summary>
    private static PxContainer OnePixel(byte r, byte g, byte b, byte a)
    {
        var p = PixelContainers.Create(1, 1);
        p.SetPixel(0, 0, r, g, b, a);
        return p;
    }

    // ── Invert ──────────────────────────────────────────────────────
    [Fact]
    public void Invert_FlipsRgbPreservesAlpha()
    {
        var src = OnePixel(10, 20, 30, 128);
        var res = ImagePointOps.Invert(src).GetPixel(0, 0);
        Assert.Equal(245, res.R);
        Assert.Equal(235, res.G);
        Assert.Equal(225, res.B);
        Assert.Equal(128, res.A);
    }

    [Fact]
    public void Invert_WhiteBecomesBlack()
    {
        var src = OnePixel(255, 255, 255, 255);
        var res = ImagePointOps.Invert(src).GetPixel(0, 0);
        Assert.Equal(new Rgba(0, 0, 0, 255), res);
    }

    [Fact]
    public void Invert_PreservesDimensions()
    {
        var src = PixelContainers.Create(4, 3);
        var res = ImagePointOps.Invert(src);
        Assert.Equal(4, res.Width);
        Assert.Equal(3, res.Height);
    }

    // ── Threshold ───────────────────────────────────────────────────
    [Fact]
    public void Threshold_AboveBecomesWhite()
    {
        var src = OnePixel(200, 200, 200, 99);
        var res = ImagePointOps.Threshold(src, 128).GetPixel(0, 0);
        Assert.Equal(new Rgba(255, 255, 255, 99), res);
    }

    [Fact]
    public void Threshold_BelowBecomesBlack()
    {
        var src = OnePixel(50, 50, 50, 77);
        var res = ImagePointOps.Threshold(src, 128).GetPixel(0, 0);
        Assert.Equal(new Rgba(0, 0, 0, 77), res);
    }

    [Fact]
    public void ThresholdLuminance_UsesRec709Weights()
    {
        // Pure green at 200 → Y ≈ 0.7152 * 200 = 143 ≥ 128 → white
        var src = OnePixel(0, 200, 0, 255);
        var res = ImagePointOps.ThresholdLuminance(src, 128).GetPixel(0, 0);
        Assert.Equal(255, res.R);
    }

    [Fact]
    public void ThresholdLuminance_BlueIsWeakestChannel()
    {
        // Pure blue at 200 → Y ≈ 0.0722 * 200 = 14 < 128 → black
        var src = OnePixel(0, 0, 200, 255);
        var res = ImagePointOps.ThresholdLuminance(src, 128).GetPixel(0, 0);
        Assert.Equal(0, res.R);
    }

    // ── Posterize ───────────────────────────────────────────────────
    [Fact]
    public void Posterize_TwoLevelsBinarizes()
    {
        var src = OnePixel(100, 130, 200, 255);
        var res = ImagePointOps.Posterize(src, 2).GetPixel(0, 0);
        // step=255, round(round(100/255)*255) = round(0*255) = 0
        // round(round(130/255)*255) = round(1*255) = 255
        Assert.Equal(0, res.R);
        Assert.Equal(255, res.G);
        Assert.Equal(255, res.B);
    }

    [Fact]
    public void Posterize_RejectsTooFewLevels()
    {
        Assert.Throws<ArgumentException>(() =>
            ImagePointOps.Posterize(OnePixel(0, 0, 0, 0), 1));
    }

    // ── SwapRgbBgr ──────────────────────────────────────────────────
    [Fact]
    public void SwapRgbBgr_SwapsRedAndBlue()
    {
        var src = OnePixel(10, 20, 30, 40);
        var res = ImagePointOps.SwapRgbBgr(src).GetPixel(0, 0);
        Assert.Equal(new Rgba(30, 20, 10, 40), res);
    }

    // ── ExtractChannel ──────────────────────────────────────────────
    [Fact]
    public void ExtractChannel_Red()
    {
        var res = ImagePointOps.ExtractChannel(OnePixel(200, 100, 50, 99), 0).GetPixel(0, 0);
        Assert.Equal(new Rgba(200, 200, 200, 255), res);
    }

    [Fact]
    public void ExtractChannel_Alpha()
    {
        var res = ImagePointOps.ExtractChannel(OnePixel(1, 2, 3, 77), 3).GetPixel(0, 0);
        Assert.Equal(new Rgba(77, 77, 77, 255), res);
    }

    [Fact]
    public void ExtractChannel_RejectsOutOfRange()
    {
        Assert.Throws<ArgumentException>(() =>
            ImagePointOps.ExtractChannel(OnePixel(0, 0, 0, 0), 4));
    }

    // ── Brightness ──────────────────────────────────────────────────
    [Fact]
    public void Brightness_PositiveDeltaAdds()
    {
        var res = ImagePointOps.Brightness(OnePixel(100, 100, 100, 50), 50).GetPixel(0, 0);
        Assert.Equal(150, res.R);
        Assert.Equal(50, res.A);
    }

    [Fact]
    public void Brightness_ClampsAt255()
    {
        var res = ImagePointOps.Brightness(OnePixel(200, 200, 200, 50), 100).GetPixel(0, 0);
        Assert.Equal(255, res.R);
    }

    [Fact]
    public void Brightness_ClampsAt0()
    {
        var res = ImagePointOps.Brightness(OnePixel(50, 50, 50, 99), -100).GetPixel(0, 0);
        Assert.Equal(0, res.R);
    }

    // ── Contrast ────────────────────────────────────────────────────
    [Fact]
    public void Contrast_ZeroFactorPreservesMidGrey()
    {
        var res = ImagePointOps.Contrast(OnePixel(128, 128, 128, 255), 0.0).GetPixel(0, 0);
        Assert.Equal(128, res.R);
    }

    [Fact]
    public void Contrast_PositiveFactorStretches()
    {
        // Bright pixel gets brighter, dark gets darker
        var bright = ImagePointOps.Contrast(OnePixel(200, 200, 200, 255), 0.5).GetPixel(0, 0);
        var dark = ImagePointOps.Contrast(OnePixel(60, 60, 60, 255), 0.5).GetPixel(0, 0);
        Assert.True(bright.R > 200);
        Assert.True(dark.R < 60);
    }

    // ── Gamma ───────────────────────────────────────────────────────
    [Fact]
    public void Gamma_OneIsNoOp()
    {
        var src = OnePixel(100, 150, 200, 99);
        var res = ImagePointOps.Gamma(src, 1.0).GetPixel(0, 0);
        // Decode then encode should be near-identity (±1 from rounding)
        Assert.InRange(res.R, 99, 101);
        Assert.Equal(99, res.A);
    }

    [Fact]
    public void Gamma_LessThanOneBrightens()
    {
        var src = OnePixel(64, 64, 64, 255);
        var res = ImagePointOps.Gamma(src, 0.5).GetPixel(0, 0);
        Assert.True(res.R > 64);
    }

    // ── Exposure ────────────────────────────────────────────────────
    [Fact]
    public void Exposure_PlusOneDoublesLinearLight()
    {
        // 18% grey ≈ 118 in sRGB; +1 stop ≈ 36% linear ≈ sRGB 163
        var res = ImagePointOps.Exposure(OnePixel(118, 118, 118, 255), 1.0).GetPixel(0, 0);
        Assert.True(res.R > 150);
    }

    [Fact]
    public void Exposure_ClampsAtWhite()
    {
        var res = ImagePointOps.Exposure(OnePixel(255, 255, 255, 255), 3.0).GetPixel(0, 0);
        Assert.Equal(255, res.R);
    }

    // ── Greyscale ───────────────────────────────────────────────────
    [Fact]
    public void Greyscale_PreservesMonoGreyscale()
    {
        var res = ImagePointOps.Greyscale(OnePixel(100, 100, 100, 200)).GetPixel(0, 0);
        Assert.Equal(res.R, res.G);
        Assert.Equal(res.G, res.B);
        Assert.Equal(200, res.A);
    }

    [Fact]
    public void Greyscale_Bt601DifferentFromRec709()
    {
        var src = OnePixel(255, 0, 0, 255);
        var rec709 = ImagePointOps.Greyscale(src, ImagePointOps.GreyscaleMethod.Rec709).GetPixel(0, 0);
        var bt601 = ImagePointOps.Greyscale(src, ImagePointOps.GreyscaleMethod.Bt601).GetPixel(0, 0);
        // BT.601 weights red higher (0.299 vs 0.2126) → brighter result
        Assert.True(bt601.R > rec709.R);
    }

    [Fact]
    public void Greyscale_AverageIsEqualWeights()
    {
        var src = OnePixel(255, 255, 0, 255);
        var res = ImagePointOps.Greyscale(src, ImagePointOps.GreyscaleMethod.Average).GetPixel(0, 0);
        Assert.True(res.R > 0);
    }

    // ── Sepia ───────────────────────────────────────────────────────
    [Fact]
    public void Sepia_PureWhiteLandsWarm()
    {
        var res = ImagePointOps.Sepia(OnePixel(255, 255, 255, 200)).GetPixel(0, 0);
        // Red output dominates because coefficients sum highest for R row
        Assert.True(res.R >= res.G);
        Assert.True(res.G >= res.B);
        Assert.Equal(200, res.A);
    }

    // ── ColourMatrix ────────────────────────────────────────────────
    [Fact]
    public void ColourMatrix_IdentityIsNearlyNoOp()
    {
        var m = new double[,] { { 1, 0, 0 }, { 0, 1, 0 }, { 0, 0, 1 } };
        var res = ImagePointOps.ColourMatrix(OnePixel(100, 150, 200, 99), m).GetPixel(0, 0);
        Assert.InRange(res.R, 99, 101);
        Assert.InRange(res.G, 149, 151);
        Assert.InRange(res.B, 199, 201);
    }

    [Fact]
    public void ColourMatrix_RejectsNonSquareMatrix()
    {
        var m = new double[,] { { 1, 0 }, { 0, 1 } };
        Assert.Throws<ArgumentException>(() =>
            ImagePointOps.ColourMatrix(OnePixel(0, 0, 0, 0), m));
    }

    [Fact]
    public void ColourMatrix_RejectsNull()
    {
        Assert.Throws<ArgumentNullException>(() =>
            ImagePointOps.ColourMatrix(OnePixel(0, 0, 0, 0), null!));
    }

    // ── Saturate ────────────────────────────────────────────────────
    [Fact]
    public void Saturate_ZeroCollapsesToGrey()
    {
        var res = ImagePointOps.Saturate(OnePixel(255, 0, 0, 255), 0.0).GetPixel(0, 0);
        Assert.Equal(res.R, res.G);
        Assert.Equal(res.G, res.B);
    }

    [Fact]
    public void Saturate_OneIsNoOp()
    {
        var src = OnePixel(100, 150, 200, 255);
        var res = ImagePointOps.Saturate(src, 1.0).GetPixel(0, 0);
        Assert.InRange(res.R, 99, 101);
        Assert.InRange(res.G, 149, 151);
        Assert.InRange(res.B, 199, 201);
    }

    // ── HueRotate ───────────────────────────────────────────────────
    [Fact]
    public void HueRotate_ZeroIsNoOp()
    {
        var src = OnePixel(255, 100, 50, 200);
        var res = ImagePointOps.HueRotate(src, 0.0).GetPixel(0, 0);
        // Should be very close to original
        Assert.InRange(Math.Abs(res.R - 255), 0, 2);
        Assert.Equal(200, res.A);
    }

    [Fact]
    public void HueRotate_120DegreesRedToGreen()
    {
        var src = OnePixel(255, 0, 0, 255);
        var res = ImagePointOps.HueRotate(src, 120.0).GetPixel(0, 0);
        // After 120° rotation, red should become green-dominant
        Assert.True(res.G > res.R);
        Assert.True(res.G > res.B);
    }

    [Fact]
    public void HueRotate_NegativeWrapsCorrectly()
    {
        var src = OnePixel(255, 0, 0, 255);
        var res = ImagePointOps.HueRotate(src, -120.0).GetPixel(0, 0);
        // -120 = +240 → red becomes blue-dominant
        Assert.True(res.B > res.R);
    }

    [Fact]
    public void HueRotate_GreyStaysGrey()
    {
        var res = ImagePointOps.HueRotate(OnePixel(128, 128, 128, 255), 90.0).GetPixel(0, 0);
        // Achromatic input has undefined hue; should stay near-grey
        Assert.InRange(Math.Abs(res.R - res.G), 0, 2);
        Assert.InRange(Math.Abs(res.G - res.B), 0, 2);
    }

    // ── sRGB↔Linear byte images ─────────────────────────────────────
    [Fact]
    public void SrgbToLinearImage_Roundtrip_Approximate()
    {
        var src = OnePixel(128, 128, 128, 200);
        var linear = ImagePointOps.SrgbToLinearImage(src);
        var back = ImagePointOps.LinearToSrgbImage(linear);
        var res = back.GetPixel(0, 0);
        Assert.InRange(Math.Abs(res.R - 128), 0, 2);
    }

    [Fact]
    public void SrgbToLinearImage_BlackAndWhitePreserved()
    {
        var black = ImagePointOps.SrgbToLinearImage(OnePixel(0, 0, 0, 255)).GetPixel(0, 0);
        var white = ImagePointOps.SrgbToLinearImage(OnePixel(255, 255, 255, 255)).GetPixel(0, 0);
        Assert.Equal(0, black.R);
        Assert.Equal(255, white.R);
    }

    [Fact]
    public void LinearToSrgbImage_MidGreyBrightens()
    {
        // Linear 128 (≈0.5) → sRGB ≈ 188
        var res = ImagePointOps.LinearToSrgbImage(OnePixel(128, 128, 128, 255)).GetPixel(0, 0);
        Assert.True(res.R > 128);
    }

    // ── LUT ─────────────────────────────────────────────────────────
    [Fact]
    public void ApplyLut1dU8_IdentityIsNoOp()
    {
        var id = new byte[256];
        for (int i = 0; i < 256; i++) id[i] = (byte)i;
        var src = OnePixel(100, 150, 200, 99);
        var res = ImagePointOps.ApplyLut1dU8(src, id, id, id).GetPixel(0, 0);
        Assert.Equal(new Rgba(100, 150, 200, 99), res);
    }

    [Fact]
    public void ApplyLut1dU8_IndependentPerChannel()
    {
        var rlut = new byte[256]; Array.Fill(rlut, (byte)10);
        var glut = new byte[256]; Array.Fill(glut, (byte)20);
        var blut = new byte[256]; Array.Fill(blut, (byte)30);
        var res = ImagePointOps.ApplyLut1dU8(OnePixel(0, 0, 0, 99), rlut, glut, blut).GetPixel(0, 0);
        Assert.Equal(new Rgba(10, 20, 30, 99), res);
    }

    [Fact]
    public void ApplyLut1dU8_RejectsBadLutR()
    {
        var bad = new byte[128];
        var good = new byte[256];
        Assert.Throws<ArgumentException>(() =>
            ImagePointOps.ApplyLut1dU8(OnePixel(0, 0, 0, 0), bad, good, good));
    }

    [Fact]
    public void ApplyLut1dU8_RejectsBadLutG()
    {
        var bad = new byte[128];
        var good = new byte[256];
        Assert.Throws<ArgumentException>(() =>
            ImagePointOps.ApplyLut1dU8(OnePixel(0, 0, 0, 0), good, bad, good));
    }

    [Fact]
    public void ApplyLut1dU8_RejectsBadLutB()
    {
        var bad = new byte[128];
        var good = new byte[256];
        Assert.Throws<ArgumentException>(() =>
            ImagePointOps.ApplyLut1dU8(OnePixel(0, 0, 0, 0), good, good, bad));
    }

    [Fact]
    public void ApplyLut1dU8_RejectsNullLut()
    {
        var good = new byte[256];
        Assert.Throws<ArgumentNullException>(() =>
            ImagePointOps.ApplyLut1dU8(OnePixel(0, 0, 0, 0), null!, good, good));
    }

    [Fact]
    public void BuildLut1dU8_IdentityFunctionIsRoundtrip()
    {
        var lut = ImagePointOps.BuildLut1dU8(x => x);
        Assert.Equal(256, lut.Length);
        Assert.Equal(0, lut[0]);
        Assert.Equal(255, lut[255]);
    }

    [Fact]
    public void BuildLut1dU8_RejectsNull()
    {
        Assert.Throws<ArgumentNullException>(() =>
            ImagePointOps.BuildLut1dU8(null!));
    }

    [Fact]
    public void BuildGammaLut_IsUsableWithApplyLut()
    {
        var lut = ImagePointOps.BuildGammaLut(0.5);
        Assert.Equal(256, lut.Length);
        var res = ImagePointOps.ApplyLut1dU8(OnePixel(64, 64, 64, 255), lut, lut, lut).GetPixel(0, 0);
        // γ=0.5 brightens midtones
        Assert.True(res.R > 64);
    }

    // ── Multi-pixel sanity ──────────────────────────────────────────
    [Fact]
    public void MapPixels_OperatesOnEveryPixel()
    {
        var src = PixelContainers.Create(2, 2);
        src.SetPixel(0, 0, 10, 20, 30, 40);
        src.SetPixel(1, 0, 50, 60, 70, 80);
        src.SetPixel(0, 1, 90, 100, 110, 120);
        src.SetPixel(1, 1, 130, 140, 150, 160);
        var res = ImagePointOps.Invert(src);
        Assert.Equal(245, res.GetPixel(0, 0).R);
        Assert.Equal(205, res.GetPixel(1, 0).R);
        Assert.Equal(165, res.GetPixel(0, 1).R);
        Assert.Equal(125, res.GetPixel(1, 1).R);
    }

    [Fact]
    public void OriginalIsNotMutated()
    {
        var src = OnePixel(10, 20, 30, 40);
        ImagePointOps.Invert(src);
        var stillThere = src.GetPixel(0, 0);
        Assert.Equal(new Rgba(10, 20, 30, 40), stillThere);
    }
}
