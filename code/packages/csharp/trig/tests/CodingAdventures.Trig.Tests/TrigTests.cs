namespace CodingAdventures.Trig.Tests;

public sealed class TrigTests
{
    private static bool ApproxEqual(double left, double right, double tolerance = 1e-10) =>
        Math.Abs(left - right) < tolerance;

    [Fact]
    public void SinMatchesKnownPoints()
    {
        Assert.True(ApproxEqual(Trig.Sin(0.0), 0.0));
        Assert.True(ApproxEqual(Trig.Sin(Trig.PI / 2.0), 1.0));
        Assert.True(ApproxEqual(Trig.Sin(Trig.PI), 0.0));
        Assert.True(ApproxEqual(Trig.Sin(3.0 * Trig.PI / 2.0), -1.0));
    }

    [Fact]
    public void CosMatchesKnownPoints()
    {
        Assert.True(ApproxEqual(Trig.Cos(0.0), 1.0));
        Assert.True(ApproxEqual(Trig.Cos(Trig.PI / 2.0), 0.0));
        Assert.True(ApproxEqual(Trig.Cos(Trig.PI), -1.0));
    }

    [Fact]
    public void SinAndCosRespectOddEvenSymmetry()
    {
        var values = new[] { 0.5, 1.0, 2.0, Trig.PI / 4.0, Trig.PI / 3.0 };
        foreach (var value in values)
        {
            Assert.True(ApproxEqual(Trig.Sin(-value), -Trig.Sin(value)));
            Assert.True(ApproxEqual(Trig.Cos(-value), Trig.Cos(value)));
        }
    }

    [Fact]
    public void PythagoreanIdentityHolds()
    {
        var values = new[] { 0.0, 0.5, 1.0, Trig.PI / 6.0, Trig.PI / 4.0, Trig.PI / 3.0, Trig.PI / 2.0, Trig.PI, 2.5, 5.0 };
        foreach (var value in values)
        {
            var sine = Trig.Sin(value);
            var cosine = Trig.Cos(value);
            Assert.True(ApproxEqual(sine * sine + cosine * cosine, 1.0));
        }
    }

    [Fact]
    public void RangeReductionKeepsLargeInputsAccurate()
    {
        Assert.True(ApproxEqual(Trig.Sin(1000.0 * Trig.PI), 0.0));
        Assert.True(ApproxEqual(Trig.Cos(500.0 * 2.0 * Trig.PI), 1.0));
    }

    [Fact]
    public void AngleConversionsRoundTrip()
    {
        Assert.True(ApproxEqual(Trig.Radians(180.0), Trig.PI));
        Assert.True(ApproxEqual(Trig.Radians(90.0), Trig.PI / 2.0));
        Assert.True(ApproxEqual(Trig.Degrees(Trig.PI), 180.0));
        Assert.True(ApproxEqual(Trig.Degrees(Trig.PI / 2.0), 90.0));

        foreach (var degrees in new[] { 0.0, 45.0, 90.0, 180.0, 270.0, 360.0 })
        {
            Assert.True(ApproxEqual(Trig.Degrees(Trig.Radians(degrees)), degrees));
        }
    }

    [Fact]
    public void SqrtMatchesKnownValues()
    {
        static bool Relative(double actual, double expected) =>
            Math.Abs(actual - expected) / Math.Abs(expected) <= 1e-12;

        Assert.Equal(0.0, Trig.Sqrt(0.0));
        Assert.True(ApproxEqual(Trig.Sqrt(1.0), 1.0));
        Assert.True(ApproxEqual(Trig.Sqrt(4.0), 2.0));
        Assert.True(ApproxEqual(Trig.Sqrt(9.0), 3.0));
        Assert.True(ApproxEqual(Trig.Sqrt(2.0), 1.41421356237));
        Assert.True(ApproxEqual(Trig.Sqrt(0.25), 0.5));
        Assert.True(Math.Abs(Trig.Sqrt(1e10) - 1e5) < 1e-4);
        Assert.True(Relative(Trig.Sqrt(1e-100), 1e-50));
        Assert.True(Relative(Trig.Sqrt(double.Epsilon), 2.2227587494850775e-162));
        Assert.True(Relative(Trig.Sqrt(double.MaxValue), 1.3407807929942596e154));
        Assert.True(BitConverter.DoubleToInt64Bits(Trig.Sqrt(-0.0)) < 0);
        Assert.True(double.IsPositiveInfinity(Trig.Sqrt(double.PositiveInfinity)));
        Assert.True(double.IsNaN(Trig.Sqrt(double.NaN)));
    }

    [Fact]
    public void SqrtRejectsNegativeInputs()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => Trig.Sqrt(-1.0));
    }

    [Fact]
    public void TanMatchesKnownValues()
    {
        Assert.True(ApproxEqual(Trig.Tan(0.0), 0.0));
        Assert.True(ApproxEqual(Trig.Tan(Trig.PI / 4.0), 1.0));
        Assert.True(ApproxEqual(Trig.Tan(Trig.PI / 6.0), 1.0 / Trig.Sqrt(3.0)));
        Assert.True(ApproxEqual(Trig.Tan(-Trig.PI / 4.0), -1.0));
    }

    [Fact]
    public void AtanMatchesKnownValues()
    {
        Assert.Equal(0.0, Trig.Atan(0.0));
        Assert.True(ApproxEqual(Trig.Atan(1.0), Trig.PI / 4.0));
        Assert.True(ApproxEqual(Trig.Atan(-1.0), -Trig.PI / 4.0));
        Assert.True(ApproxEqual(Trig.Atan(Trig.Sqrt(3.0)), Trig.PI / 3.0));
        Assert.True(ApproxEqual(Trig.Atan(1.0 / Trig.Sqrt(3.0)), Trig.PI / 6.0));
        Assert.True(Math.Abs(Trig.Atan(1e10) - Trig.PI / 2.0) < 1e-5);
        Assert.True(Math.Abs(Trig.Atan(-1e10) + Trig.PI / 2.0) < 1e-5);
    }

    [Fact]
    public void Atan2HandlesAxesAndQuadrants()
    {
        Assert.True(ApproxEqual(Trig.Atan2(0.0, 1.0), 0.0));
        Assert.True(ApproxEqual(Trig.Atan2(1.0, 0.0), Trig.PI / 2.0));
        Assert.True(ApproxEqual(Trig.Atan2(0.0, -1.0), Trig.PI));
        Assert.True(ApproxEqual(Trig.Atan2(-1.0, 0.0), -Trig.PI / 2.0));
        Assert.True(ApproxEqual(Trig.Atan2(1.0, 1.0), Trig.PI / 4.0));
        Assert.True(ApproxEqual(Trig.Atan2(1.0, -1.0), 3.0 * Trig.PI / 4.0));
        Assert.True(ApproxEqual(Trig.Atan2(-1.0, -1.0), -3.0 * Trig.PI / 4.0));
        Assert.True(ApproxEqual(Trig.Atan2(-1.0, 1.0), -Trig.PI / 4.0));
    }
}
