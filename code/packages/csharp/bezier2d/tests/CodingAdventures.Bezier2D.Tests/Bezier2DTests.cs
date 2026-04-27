using CodingAdventures.Point2D;

namespace CodingAdventures.Bezier2D.Tests;

public sealed class Bezier2DTests
{
    private const double Epsilon = 1e-9;

    [Fact]
    public void VersionExists()
    {
        Assert.Equal("0.1.0", Bezier2D.Version);
    }

    [Fact]
    public void QuadraticEvaluateDerivativeSplitAndElevateWork()
    {
        var curve = new QuadraticBezier(new Point(0, 0), new Point(1, 2), new Point(2, 0));

        Assert.True(PointClose(curve.Evaluate(0), curve.P0));
        Assert.True(PointClose(curve.Evaluate(1), curve.P2));
        Assert.True(PointClose(curve.Evaluate(0.5), new Point(1, 1)));
        Assert.True(PointClose(curve.Derivative(0), new Point(2, 4)));

        var (left, right) = curve.Split(0.5);
        var splitPoint = curve.Evaluate(0.5);
        Assert.True(PointClose(left.P2, splitPoint));
        Assert.True(PointClose(right.P0, splitPoint));

        var cubic = curve.Elevate();
        foreach (var t in new[] { 0.0, 0.25, 0.5, 0.75, 1.0 })
        {
            Assert.True(PointClose(curve.Evaluate(t), cubic.Evaluate(t)));
        }
    }

    [Fact]
    public void QuadraticPolylineAndBoundingBoxWork()
    {
        var straight = new QuadraticBezier(new Point(0, 0), new Point(1, 0), new Point(2, 0));
        Assert.Equal(2, straight.ToPolyline(0.1).Count);

        var curve = new QuadraticBezier(new Point(0, 0), new Point(0, 10), new Point(10, 0));
        var points = curve.ToPolyline(0.1);
        Assert.True(points.Count > 2);
        Assert.True(PointClose(points[0], curve.P0));
        Assert.True(PointClose(points[^1], curve.P2));

        var bounds = curve.BoundingBox();
        Assert.True(bounds.X <= 0);
        Assert.True(bounds.Y <= 0);
        Assert.True(bounds.X + bounds.Width >= 10);
        Assert.True(bounds.Height > 0);
        Assert.Throws<ArgumentOutOfRangeException>(() => curve.ToPolyline(-0.1));
    }

    [Fact]
    public void CubicEvaluateDerivativeAndSplitWork()
    {
        var curve = new CubicBezier(new Point(0, 0), new Point(1, 2), new Point(3, 2), new Point(4, 0));

        Assert.True(PointClose(curve.Evaluate(0), curve.P0));
        Assert.True(PointClose(curve.Evaluate(1), curve.P3));
        Assert.True(Close(curve.Evaluate(0.5).X, 2.0));

        var straight = new CubicBezier(new Point(0, 0), new Point(1, 0), new Point(2, 0), new Point(3, 0));
        Assert.True(PointClose(straight.Derivative(0), new Point(3, 0)));

        var (left, right) = curve.Split(0.5);
        var splitPoint = curve.Evaluate(0.5);
        Assert.True(PointClose(left.P3, splitPoint));
        Assert.True(PointClose(right.P0, splitPoint));
    }

    [Fact]
    public void CubicPolylineAndBoundingBoxWork()
    {
        var straight = new CubicBezier(new Point(0, 0), new Point(1, 0), new Point(2, 0), new Point(3, 0));
        Assert.Equal(2, straight.ToPolyline(0.1).Count);

        var curve = new CubicBezier(new Point(0, 0), new Point(0, 10), new Point(10, 10), new Point(10, 0));
        var points = curve.ToPolyline(0.1);
        Assert.True(points.Count > 2);
        Assert.True(PointClose(points[0], curve.P0));
        Assert.True(PointClose(points[^1], curve.P3));

        var bounds = curve.BoundingBox();
        for (var index = 0; index <= 20; index++)
        {
            var point = curve.Evaluate(index / 20.0);
            Assert.True(point.X >= bounds.X - 1e-6 && point.X <= bounds.X + bounds.Width + 1e-6);
            Assert.True(point.Y >= bounds.Y - 1e-6 && point.Y <= bounds.Y + bounds.Height + 1e-6);
        }

        var flatBounds = straight.BoundingBox();
        Assert.Equal(new Rect(0, 0, 3, 0), flatBounds);
        Assert.Throws<ArgumentOutOfRangeException>(() => curve.ToPolyline(double.NaN));
    }

    private static bool Close(double left, double right) => Math.Abs(left - right) < Epsilon;

    private static bool PointClose(Point left, Point right) => Close(left.X, right.X) && Close(left.Y, right.Y);
}
