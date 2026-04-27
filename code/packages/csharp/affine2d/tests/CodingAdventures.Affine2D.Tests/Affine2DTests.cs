using CodingAdventures.Point2D;
using TrigPackage = CodingAdventures.Trig.Trig;

namespace CodingAdventures.Affine2D.Tests;

public sealed class Affine2DTests
{
    private const double Epsilon = 1e-9;

    [Fact]
    public void VersionExists()
    {
        Assert.Equal("0.1.0", Affine2DPackage.Version);
    }

    [Fact]
    public void FactoriesApplyExpectedTransforms()
    {
        var identity = Affine2D.Identity();
        Assert.Equal(new[] { 1.0, 0.0, 0.0, 1.0, 0.0, 0.0 }, identity.ToArray());
        Assert.True(PointClose(identity.ApplyToPoint(new Point(3, 4)), new Point(3, 4)));

        Assert.True(PointClose(Affine2D.Translate(5, -3).ApplyToPoint(new Point(1, 2)), new Point(6, -1)));
        Assert.True(PointClose(Affine2D.Translate(5, -3).ApplyToVector(new Point(1, 2)), new Point(1, 2)));
        Assert.True(PointClose(Affine2D.Scale(2, 3).ApplyToPoint(new Point(1, 1)), new Point(2, 3)));
        Assert.True(PointClose(Affine2D.ScaleUniform(5).ApplyToPoint(new Point(2, 3)), new Point(10, 15)));
    }

    [Fact]
    public void RotationAndSkewFactoriesWork()
    {
        Assert.True(PointClose(Affine2D.Rotate(TrigPackage.PI / 2.0).ApplyToPoint(new Point(1, 0)), new Point(0, 1)));
        Assert.True(Affine2D.Rotate(2.0 * TrigPackage.PI).IsIdentity());

        var center = new Point(1, 0);
        Assert.True(PointClose(Affine2D.RotateAround(center, TrigPackage.PI / 2.0).ApplyToPoint(center), center));
        Assert.True(PointClose(Affine2D.SkewX(TrigPackage.PI / 4.0).ApplyToPoint(new Point(0, 1)), new Point(1, 1)));
        Assert.True(PointClose(Affine2D.SkewY(TrigPackage.PI / 4.0).ApplyToPoint(new Point(1, 0)), new Point(1, 1)));
    }

    [Fact]
    public void CompositionDeterminantAndInvertWork()
    {
        var scale = Affine2D.ScaleUniform(2);
        var translate = Affine2D.Translate(10, 0);
        var composed = translate.Multiply(scale);
        Assert.True(PointClose(composed.ApplyToPoint(new Point(1, 1)), new Point(12, 2)));

        Assert.True(AffineClose(Affine2D.Identity().Multiply(translate), translate));
        Assert.True(AffineClose(scale.Then(translate), composed));
        Assert.True(Close(Affine2D.Scale(2, 3).Determinant(), 6));
        Assert.True(Close(Affine2D.Rotate(TrigPackage.PI / 3.0).Determinant(), 1));

        var inverse = translate.Invert();
        Assert.NotNull(inverse);
        Assert.True(translate.Multiply(inverse.Value).IsIdentity());
        Assert.Null(new Affine2D(0, 0, 0, 0, 0, 0).Invert());
    }

    [Fact]
    public void PredicatesDistinguishTransformKinds()
    {
        Assert.True(Affine2D.Identity().IsIdentity());
        Assert.False(Affine2D.Translate(1, 0).IsIdentity());
        Assert.True(Affine2D.Translate(5, 3).IsTranslationOnly());
        Assert.False(Affine2D.Rotate(0.1).IsTranslationOnly());
        Assert.False(Affine2D.Scale(2, 1).IsTranslationOnly());
    }

    private static bool Close(double left, double right) => Math.Abs(left - right) < Epsilon;

    private static bool PointClose(Point left, Point right) => Close(left.X, right.X) && Close(left.Y, right.Y);

    private static bool AffineClose(Affine2D left, Affine2D right) =>
        left.ToArray().Zip(right.ToArray()).All(pair => Close(pair.First, pair.Second));
}
