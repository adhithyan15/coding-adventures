using CodingAdventures.Point2D;

namespace CodingAdventures.Point2D.Tests;

public sealed class Point2DTests
{
    [Fact]
    public void PointArithmeticAndVectorOperationsWork()
    {
        var point = new Point(3, 4);
        var other = new Point(1, 2);

        Assert.Equal(new Point(4, 6), point.Add(other));
        Assert.Equal(new Point(2, 2), point.Subtract(other));
        Assert.Equal(new Point(6, 8), point.Scale(2));
        Assert.Equal(new Point(-3, -4), point.Negate());
        Assert.Equal(11, point.Dot(other));
        Assert.Equal(2, point.Cross(other));
        Assert.Equal(5, point.Magnitude(), 9);
        Assert.Equal(25, point.MagnitudeSquared(), 9);
    }

    [Fact]
    public void PointDerivedOperationsWork()
    {
        var point = new Point(3, 4);

        Assert.Equal(new Point(0, 0), Point.Origin());
        Assert.Equal(new Point(0.6, 0.8), point.Normalize());
        Assert.Equal(Point.Origin(), Point.Origin().Normalize());
        Assert.Equal(5, point.Distance(Point.Origin()), 9);
        Assert.Equal(25, point.DistanceSquared(Point.Origin()), 9);
        Assert.Equal(new Point(5, 5), Point.Origin().Lerp(new Point(10, 10), 0.5));
        Assert.Equal(new Point(-4, 3), point.Perpendicular());
        Assert.Equal(Math.Atan2(4, 3), point.Angle(), 9);
    }

    [Fact]
    public void RectAccessorsAndContainmentWork()
    {
        var rect = new Rect(10, 20, 30, 40);

        Assert.Equal(Rect.Zero(), new Rect(0, 0, 0, 0));
        Assert.Equal(new Rect(10, 20, 30, 40), Rect.FromPoints(new Point(10, 20), new Point(40, 60)));
        Assert.Equal(new Point(10, 20), rect.MinPoint());
        Assert.Equal(new Point(40, 60), rect.MaxPoint());
        Assert.Equal(new Point(25, 40), rect.Center());
        Assert.False(rect.IsEmpty());
        Assert.True(new Rect(0, 0, 0, 5).IsEmpty());
        Assert.True(rect.ContainsPoint(new Point(10, 20)));
        Assert.False(rect.ContainsPoint(new Point(40, 60)));
    }

    [Fact]
    public void RectUnionIntersectionAndExpansionWork()
    {
        var left = new Rect(0, 0, 10, 10);
        var right = new Rect(5, 5, 10, 10);

        Assert.Equal(new Rect(0, 0, 15, 15), left.Union(right));
        Assert.Equal(right, Rect.Zero().Union(right));
        Assert.Equal(left, left.Union(Rect.Zero()));
        Assert.Equal(new Rect(5, 5, 5, 5), left.Intersection(right));
        Assert.Null(left.Intersection(new Rect(20, 20, 2, 2)));
        Assert.Equal(new Rect(-2, -2, 14, 14), left.ExpandBy(2));
    }
}
