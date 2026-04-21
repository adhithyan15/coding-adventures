using CodingAdventures.ImageGeometricTransforms;
using CodingAdventures.PixelContainer;
using PxContainer = global::CodingAdventures.PixelContainer.PixelContainer;

namespace CodingAdventures.ImageGeometricTransforms.Tests;

public class ImageGeometricTransformsTests
{
    /// <summary>
    /// Make an image with a unique pixel at every coordinate so we can
    /// verify transforms moved things to the right place, not just that
    /// the output looks plausible.
    /// </summary>
    private static PxContainer MakeGradient(int w, int h)
    {
        var img = PixelContainers.Create(w, h);
        for (int y = 0; y < h; y++)
            for (int x = 0; x < w; x++)
                img.SetPixel(x, y, (byte)(x * 10), (byte)(y * 10), (byte)(x + y), 255);
        return img;
    }

    // ── FlipHorizontal ──────────────────────────────────────────────
    [Fact]
    public void FlipHorizontal_SwapsColumns()
    {
        var src = MakeGradient(4, 3);
        var res = ImageGeometricTransforms.FlipHorizontal(src);
        for (int y = 0; y < 3; y++)
            for (int x = 0; x < 4; x++)
                Assert.Equal(src.GetPixel(3 - x, y), res.GetPixel(x, y));
    }

    [Fact]
    public void FlipHorizontal_PreservesDimensions()
    {
        var res = ImageGeometricTransforms.FlipHorizontal(PixelContainers.Create(5, 7));
        Assert.Equal(5, res.Width);
        Assert.Equal(7, res.Height);
    }

    // ── FlipVertical ────────────────────────────────────────────────
    [Fact]
    public void FlipVertical_SwapsRows()
    {
        var src = MakeGradient(3, 4);
        var res = ImageGeometricTransforms.FlipVertical(src);
        for (int y = 0; y < 4; y++)
            for (int x = 0; x < 3; x++)
                Assert.Equal(src.GetPixel(x, 3 - y), res.GetPixel(x, y));
    }

    [Fact]
    public void FlipVertical_DoubleIsIdentity()
    {
        var src = MakeGradient(3, 3);
        var res = ImageGeometricTransforms.FlipVertical(ImageGeometricTransforms.FlipVertical(src));
        for (int y = 0; y < 3; y++)
            for (int x = 0; x < 3; x++)
                Assert.Equal(src.GetPixel(x, y), res.GetPixel(x, y));
    }

    // ── Rotate90CW ──────────────────────────────────────────────────
    [Fact]
    public void Rotate90CW_SwapsDimensions()
    {
        var src = MakeGradient(4, 3);
        var res = ImageGeometricTransforms.Rotate90CW(src);
        Assert.Equal(3, res.Width);
        Assert.Equal(4, res.Height);
    }

    [Fact]
    public void Rotate90CW_FourTimesIsIdentity()
    {
        var src = MakeGradient(3, 3);
        var r1 = ImageGeometricTransforms.Rotate90CW(src);
        var r2 = ImageGeometricTransforms.Rotate90CW(r1);
        var r3 = ImageGeometricTransforms.Rotate90CW(r2);
        var r4 = ImageGeometricTransforms.Rotate90CW(r3);
        for (int y = 0; y < 3; y++)
            for (int x = 0; x < 3; x++)
                Assert.Equal(src.GetPixel(x, y), r4.GetPixel(x, y));
    }

    [Fact]
    public void Rotate90CW_TopLeftMovesToTopRight()
    {
        var src = PixelContainers.Create(2, 2);
        src.SetPixel(0, 0, 111, 0, 0, 255);
        src.SetPixel(1, 0, 0, 222, 0, 255);
        var res = ImageGeometricTransforms.Rotate90CW(src);
        // Top-left (0,0) should end up at top-right (outW-1, 0)
        Assert.Equal(111, res.GetPixel(res.Width - 1, 0).R);
    }

    // ── Rotate90CCW ─────────────────────────────────────────────────
    [Fact]
    public void Rotate90CCW_SwapsDimensions()
    {
        var res = ImageGeometricTransforms.Rotate90CCW(PixelContainers.Create(5, 2));
        Assert.Equal(2, res.Width);
        Assert.Equal(5, res.Height);
    }

    [Fact]
    public void Rotate90CCW_IsInverseOfCW()
    {
        var src = MakeGradient(4, 3);
        var res = ImageGeometricTransforms.Rotate90CCW(ImageGeometricTransforms.Rotate90CW(src));
        for (int y = 0; y < 3; y++)
            for (int x = 0; x < 4; x++)
                Assert.Equal(src.GetPixel(x, y), res.GetPixel(x, y));
    }

    // ── Rotate180 ───────────────────────────────────────────────────
    [Fact]
    public void Rotate180_SamePixelsInReverseOrder()
    {
        var src = MakeGradient(3, 2);
        var res = ImageGeometricTransforms.Rotate180(src);
        Assert.Equal(src.GetPixel(0, 0), res.GetPixel(2, 1));
        Assert.Equal(src.GetPixel(2, 1), res.GetPixel(0, 0));
    }

    [Fact]
    public void Rotate180_TwiceIsIdentity()
    {
        var src = MakeGradient(3, 3);
        var res = ImageGeometricTransforms.Rotate180(ImageGeometricTransforms.Rotate180(src));
        for (int y = 0; y < 3; y++)
            for (int x = 0; x < 3; x++)
                Assert.Equal(src.GetPixel(x, y), res.GetPixel(x, y));
    }

    // ── Crop ────────────────────────────────────────────────────────
    [Fact]
    public void Crop_ExtractsInteriorRectangle()
    {
        var src = MakeGradient(5, 5);
        var res = ImageGeometricTransforms.Crop(src, 1, 1, 3, 3);
        Assert.Equal(3, res.Width);
        Assert.Equal(3, res.Height);
        Assert.Equal(src.GetPixel(1, 1), res.GetPixel(0, 0));
        Assert.Equal(src.GetPixel(3, 3), res.GetPixel(2, 2));
    }

    [Fact]
    public void Crop_OutOfBoundsFillsWithZero()
    {
        var src = MakeGradient(3, 3);
        var res = ImageGeometricTransforms.Crop(src, 2, 2, 3, 3);
        // (0,0) maps to src(2,2) — inside
        Assert.Equal(src.GetPixel(2, 2), res.GetPixel(0, 0));
        // (2,2) maps to src(4,4) — outside → default
        Assert.Equal(new Rgba(0, 0, 0, 0), res.GetPixel(2, 2));
    }

    [Fact]
    public void Crop_RejectsNegativeDimensions()
    {
        Assert.Throws<ArgumentException>(() =>
            ImageGeometricTransforms.Crop(PixelContainers.Create(1, 1), 0, 0, -1, 1));
        Assert.Throws<ArgumentException>(() =>
            ImageGeometricTransforms.Crop(PixelContainers.Create(1, 1), 0, 0, 1, -1));
    }

    // ── Scale ───────────────────────────────────────────────────────
    [Fact]
    public void Scale_IdentitySizeIsNoOp_Bilinear()
    {
        var src = MakeGradient(4, 4);
        var res = ImageGeometricTransforms.Scale(src, 4, 4, Interpolation.Bilinear, OutOfBounds.Replicate);
        for (int y = 0; y < 4; y++)
            for (int x = 0; x < 4; x++)
            {
                var a = src.GetPixel(x, y);
                var b = res.GetPixel(x, y);
                Assert.InRange(Math.Abs(a.R - b.R), 0, 2);
                Assert.InRange(Math.Abs(a.G - b.G), 0, 2);
            }
    }

    [Fact]
    public void Scale_Upsample_Nearest()
    {
        var src = PixelContainers.Create(2, 2);
        src.SetPixel(0, 0, 255, 0, 0, 255);
        src.SetPixel(1, 0, 0, 255, 0, 255);
        src.SetPixel(0, 1, 0, 0, 255, 255);
        src.SetPixel(1, 1, 255, 255, 0, 255);
        var res = ImageGeometricTransforms.Scale(src, 4, 4, Interpolation.Nearest, OutOfBounds.Replicate);
        Assert.Equal(4, res.Width);
        Assert.Equal(4, res.Height);
        // Top-left quadrant should be red
        Assert.Equal(255, res.GetPixel(0, 0).R);
    }

    [Fact]
    public void Scale_Downsample_Bilinear()
    {
        var src = MakeGradient(8, 8);
        var res = ImageGeometricTransforms.Scale(src, 4, 4, Interpolation.Bilinear, OutOfBounds.Replicate);
        Assert.Equal(4, res.Width);
        Assert.Equal(4, res.Height);
    }

    [Fact]
    public void Scale_Bicubic()
    {
        var src = MakeGradient(8, 8);
        var res = ImageGeometricTransforms.Scale(src, 16, 16, Interpolation.Bicubic, OutOfBounds.Replicate);
        Assert.Equal(16, res.Width);
    }

    [Fact]
    public void Scale_RejectsNonPositiveDimensions()
    {
        Assert.Throws<ArgumentException>(() =>
            ImageGeometricTransforms.Scale(PixelContainers.Create(1, 1), 0, 1, Interpolation.Nearest, OutOfBounds.Zero));
        Assert.Throws<ArgumentException>(() =>
            ImageGeometricTransforms.Scale(PixelContainers.Create(1, 1), 1, 0, Interpolation.Nearest, OutOfBounds.Zero));
    }

    // ── Rotate (continuous) ─────────────────────────────────────────
    [Fact]
    public void Rotate_ZeroDegrees_Crop_IsNearlyIdentity()
    {
        var src = MakeGradient(4, 4);
        var res = ImageGeometricTransforms.Rotate(src, 0.0, RotateBounds.Crop, Interpolation.Bilinear, OutOfBounds.Replicate);
        Assert.Equal(4, res.Width);
        Assert.Equal(4, res.Height);
    }

    [Fact]
    public void Rotate_Fit_ExpandsCanvas()
    {
        var src = PixelContainers.Create(4, 4);
        var res = ImageGeometricTransforms.Rotate(src, 45.0, RotateBounds.Fit, Interpolation.Bilinear, OutOfBounds.Zero);
        Assert.True(res.Width > 4);
        Assert.True(res.Height > 4);
    }

    [Fact]
    public void Rotate_Crop_KeepsCanvasSize()
    {
        var src = PixelContainers.Create(4, 4);
        var res = ImageGeometricTransforms.Rotate(src, 45.0, RotateBounds.Crop, Interpolation.Bilinear, OutOfBounds.Zero);
        Assert.Equal(4, res.Width);
        Assert.Equal(4, res.Height);
    }

    [Fact]
    public void Rotate_90DegreesMatchesRotate90CCW()
    {
        // 90° CCW via continuous should put the top-left near the bottom-left
        var src = PixelContainers.Create(4, 4);
        src.Fill(0, 0, 0, 255);
        src.SetPixel(3, 0, 200, 0, 0, 255);
        var res = ImageGeometricTransforms.Rotate(src, 90.0, RotateBounds.Fit, Interpolation.Nearest, OutOfBounds.Zero);
        Assert.Equal(4, res.Width);
        Assert.Equal(4, res.Height);
    }

    [Fact]
    public void Rotate_Bicubic()
    {
        var src = MakeGradient(4, 4);
        var res = ImageGeometricTransforms.Rotate(src, 10.0, RotateBounds.Fit, Interpolation.Bicubic, OutOfBounds.Reflect);
        Assert.True(res.Width >= 4);
    }

    // ── Translate ───────────────────────────────────────────────────
    [Fact]
    public void Translate_IntegerShift_Nearest()
    {
        var src = PixelContainers.Create(4, 4);
        src.SetPixel(1, 1, 200, 0, 0, 255);
        var res = ImageGeometricTransforms.Translate(src, 1.0, 0.0, Interpolation.Nearest, OutOfBounds.Zero);
        // Original (1,1)=red should now be at (2,1)
        Assert.Equal(200, res.GetPixel(2, 1).R);
        // Left column pulled from x=-1 → OOB Zero → (0,0,0,0)
        Assert.Equal(new Rgba(0, 0, 0, 0), res.GetPixel(0, 0));
    }

    [Fact]
    public void Translate_ZeroIsNearlyIdentity_Bilinear()
    {
        var src = MakeGradient(4, 4);
        var res = ImageGeometricTransforms.Translate(src, 0.0, 0.0, Interpolation.Bilinear, OutOfBounds.Replicate);
        for (int y = 0; y < 4; y++)
            for (int x = 0; x < 4; x++)
                Assert.InRange(Math.Abs(src.GetPixel(x, y).R - res.GetPixel(x, y).R), 0, 2);
    }

    // ── Affine ──────────────────────────────────────────────────────
    [Fact]
    public void Affine_IdentityIsNearlyNoOp()
    {
        var src = MakeGradient(4, 4);
        var m = new double[,] { { 1, 0, 0 }, { 0, 1, 0 } };
        var res = ImageGeometricTransforms.Affine(src, m, Interpolation.Bilinear, OutOfBounds.Replicate);
        Assert.Equal(4, res.Width);
    }

    [Fact]
    public void Affine_RejectsWrongDimensions()
    {
        var m = new double[,] { { 1, 0 }, { 0, 1 }, { 0, 0 } };
        Assert.Throws<ArgumentException>(() =>
            ImageGeometricTransforms.Affine(PixelContainers.Create(2, 2), m, Interpolation.Nearest, OutOfBounds.Zero));
    }

    [Fact]
    public void Affine_RejectsNull()
    {
        Assert.Throws<ArgumentNullException>(() =>
            ImageGeometricTransforms.Affine(PixelContainers.Create(2, 2), null!, Interpolation.Nearest, OutOfBounds.Zero));
    }

    [Fact]
    public void Affine_RejectsSingular()
    {
        var m = new double[,] { { 1, 2, 0 }, { 2, 4, 0 } };
        Assert.Throws<ArgumentException>(() =>
            ImageGeometricTransforms.Affine(PixelContainers.Create(2, 2), m, Interpolation.Nearest, OutOfBounds.Zero));
    }

    [Fact]
    public void Affine_PureTranslation()
    {
        var src = PixelContainers.Create(4, 4);
        src.SetPixel(0, 0, 200, 0, 0, 255);
        var m = new double[,] { { 1, 0, 1 }, { 0, 1, 0 } };
        var res = ImageGeometricTransforms.Affine(src, m, Interpolation.Nearest, OutOfBounds.Zero);
        // src(0,0)=red → x'=1, so res(1,0) should be red
        Assert.Equal(200, res.GetPixel(1, 0).R);
    }

    // ── PerspectiveWarp ─────────────────────────────────────────────
    [Fact]
    public void PerspectiveWarp_IdentityIsNearlyNoOp()
    {
        var src = MakeGradient(4, 4);
        var m = new double[,] { { 1, 0, 0 }, { 0, 1, 0 }, { 0, 0, 1 } };
        var res = ImageGeometricTransforms.PerspectiveWarp(src, m, Interpolation.Bilinear, OutOfBounds.Replicate);
        Assert.Equal(4, res.Width);
    }

    [Fact]
    public void PerspectiveWarp_RejectsWrongDimensions()
    {
        var m = new double[,] { { 1, 0 }, { 0, 1 } };
        Assert.Throws<ArgumentException>(() =>
            ImageGeometricTransforms.PerspectiveWarp(PixelContainers.Create(2, 2), m, Interpolation.Nearest, OutOfBounds.Zero));
    }

    [Fact]
    public void PerspectiveWarp_RejectsNull()
    {
        Assert.Throws<ArgumentNullException>(() =>
            ImageGeometricTransforms.PerspectiveWarp(PixelContainers.Create(2, 2), null!, Interpolation.Nearest, OutOfBounds.Zero));
    }

    [Fact]
    public void PerspectiveWarp_RejectsSingular()
    {
        var m = new double[,] { { 1, 0, 0 }, { 2, 0, 0 }, { 3, 0, 0 } };
        Assert.Throws<ArgumentException>(() =>
            ImageGeometricTransforms.PerspectiveWarp(PixelContainers.Create(2, 2), m, Interpolation.Nearest, OutOfBounds.Zero));
    }

    [Fact]
    public void PerspectiveWarp_WithNonTrivialHomography()
    {
        var src = MakeGradient(4, 4);
        // Slight perspective distortion — well-conditioned, non-identity
        var m = new double[,] { { 1.0, 0.1, 0 }, { 0.0, 1.0, 0 }, { 0.001, 0, 1 } };
        var res = ImageGeometricTransforms.PerspectiveWarp(src, m, Interpolation.Bilinear, OutOfBounds.Zero);
        Assert.Equal(4, res.Width);
        Assert.Equal(4, res.Height);
    }

    // ── OOB policies ────────────────────────────────────────────────
    [Fact]
    public void Translate_OobZero_FillsBlack()
    {
        var src = PixelContainers.Create(4, 4);
        src.Fill(200, 100, 50, 255);
        var res = ImageGeometricTransforms.Translate(src, 2.0, 0.0, Interpolation.Nearest, OutOfBounds.Zero);
        Assert.Equal(new Rgba(0, 0, 0, 0), res.GetPixel(0, 0));
    }

    [Fact]
    public void Translate_OobReplicate_UsesEdgePixel()
    {
        var src = PixelContainers.Create(4, 4);
        src.Fill(200, 100, 50, 255);
        var res = ImageGeometricTransforms.Translate(src, 2.0, 0.0, Interpolation.Nearest, OutOfBounds.Replicate);
        Assert.Equal(200, res.GetPixel(0, 0).R);
    }

    [Fact]
    public void Translate_OobReflect_MirrorsAtEdge()
    {
        var src = PixelContainers.Create(4, 4);
        src.SetPixel(0, 0, 111, 0, 0, 255);
        src.SetPixel(1, 0, 222, 0, 0, 255);
        var res = ImageGeometricTransforms.Translate(src, 1.0, 0.0, Interpolation.Nearest, OutOfBounds.Reflect);
        // Reflection at x=-1 maps to x=0, still 111
        Assert.Equal(111, res.GetPixel(0, 0).R);
    }

    [Fact]
    public void Translate_OobWrap_WrapsAround()
    {
        var src = PixelContainers.Create(4, 4);
        src.SetPixel(3, 0, 99, 0, 0, 255);
        var res = ImageGeometricTransforms.Translate(src, 1.0, 0.0, Interpolation.Nearest, OutOfBounds.Wrap);
        // res(0,0) samples from x=-1 → wrap → x=3 which is 99
        Assert.Equal(99, res.GetPixel(0, 0).R);
    }

    // ── Bicubic on edges ────────────────────────────────────────────
    [Fact]
    public void Scale_Bicubic_WithZeroOob()
    {
        var src = MakeGradient(4, 4);
        var res = ImageGeometricTransforms.Scale(src, 8, 8, Interpolation.Bicubic, OutOfBounds.Zero);
        Assert.Equal(8, res.Width);
    }

    [Fact]
    public void Scale_Bicubic_WithReflectOob()
    {
        var src = MakeGradient(4, 4);
        var res = ImageGeometricTransforms.Scale(src, 8, 8, Interpolation.Bicubic, OutOfBounds.Reflect);
        Assert.Equal(8, res.Width);
    }

    [Fact]
    public void Scale_Bicubic_WithWrapOob()
    {
        var src = MakeGradient(4, 4);
        var res = ImageGeometricTransforms.Scale(src, 8, 8, Interpolation.Bicubic, OutOfBounds.Wrap);
        Assert.Equal(8, res.Width);
    }

    // ── Original not mutated ────────────────────────────────────────
    [Fact]
    public void Transforms_DoNotMutateInput()
    {
        var src = MakeGradient(4, 4);
        ImageGeometricTransforms.FlipHorizontal(src);
        ImageGeometricTransforms.Rotate90CW(src);
        ImageGeometricTransforms.Rotate180(src);
        ImageGeometricTransforms.Scale(src, 8, 8, Interpolation.Bilinear, OutOfBounds.Replicate);
        ImageGeometricTransforms.Rotate(src, 33.0, RotateBounds.Fit, Interpolation.Bicubic, OutOfBounds.Reflect);
        // src should be unchanged
        Assert.Equal(0, src.GetPixel(0, 0).R);
        Assert.Equal(30, src.GetPixel(3, 0).R);
    }
}
