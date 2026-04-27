using CodingAdventures.Point2D;
using TrigPackage = CodingAdventures.Trig.Trig;

namespace CodingAdventures.Arc2D.Tests;

public sealed class Arc2DTests
{
    private const double Epsilon = 1e-5;

    [Fact]
    public void VersionExists()
    {
        Assert.Equal("0.1.0", Arc2D.Version);
    }

    [Fact]
    public void CenterArcEvaluatesTangentsAndBounds()
    {
        var arc = new CenterArc(Point.Origin(), 1, 1, 0, TrigPackage.PI / 2.0, 0);

        Assert.True(PointClose(arc.Evaluate(0), new Point(1, 0)));
        Assert.True(PointClose(arc.Evaluate(1), new Point(0, 1)));
        Assert.True(Close(arc.Tangent(0).X, 0));
        Assert.True(arc.Tangent(0).Y > 0);

        var fullCircle = new CenterArc(Point.Origin(), 1, 1, 0, 2.0 * TrigPackage.PI, 0);
        var bounds = fullCircle.BoundingBox();
        Assert.True(Math.Abs(bounds.X + 1.0) < 0.05);
        Assert.True(Math.Abs(bounds.Width - 2.0) < 0.05);
    }

    [Fact]
    public void CenterArcBuildsCubicBeziers()
    {
        var quarter = new CenterArc(Point.Origin(), 1, 1, 0, TrigPackage.PI / 2.0, 0);
        var quarterBeziers = quarter.ToCubicBeziers();
        Assert.Single(quarterBeziers);
        Assert.True(quarter.Evaluate(0.5).Distance(quarterBeziers[0].Evaluate(0.5)) < 0.001);

        var fullCircle = new CenterArc(Point.Origin(), 1, 1, 0, 2.0 * TrigPackage.PI, 0);
        var beziers = fullCircle.ToCubicBeziers();
        Assert.Equal(4, beziers.Count);
        for (var index = 0; index < beziers.Count - 1; index++)
        {
            Assert.True(beziers[index].P3.Distance(beziers[index + 1].P0) < 1e-6);
        }
    }

    [Fact]
    public void SvgArcHandlesDegenerateInputs()
    {
        Assert.Null(new SvgArc(Point.Origin(), Point.Origin(), 1, 1, 0, false, true).ToCenterArc());
        Assert.Null(new SvgArc(new Point(0, 0), new Point(1, 0), 0, 1, 0, false, true).ToCenterArc());
        Assert.Empty(new SvgArc(Point.Origin(), Point.Origin(), 1, 1, 0, false, true).ToCubicBeziers());
        Assert.Null(new SvgArc(Point.Origin(), Point.Origin(), 1, 1, 0, false, true).Evaluate(0));
        Assert.Null(new SvgArc(Point.Origin(), Point.Origin(), 1, 1, 0, false, true).BoundingBox());
    }

    [Fact]
    public void SvgArcConvertsEndpointForm()
    {
        var arc = new SvgArc(new Point(1, 0), new Point(0, 1), 1, 1, 0, false, true);
        var centerArc = arc.ToCenterArc();
        Assert.NotNull(centerArc);
        Assert.True(Close(centerArc.Value.Center.X, 0));
        Assert.True(Close(centerArc.Value.Center.Y, 0));
        Assert.True(centerArc.Value.SweepAngle > 0);

        var start = arc.Evaluate(0);
        Assert.NotNull(start);
        Assert.True(PointClose(start.Value, new Point(1, 0)));
        Assert.NotEmpty(arc.ToCubicBeziers());
        Assert.NotNull(arc.BoundingBox());
    }

    [Fact]
    public void SvgArcFlagsSelectDifferentSweeps()
    {
        var ccw = new SvgArc(new Point(1, 0), new Point(0, 1), 1, 1, 0, false, true).ToCenterArc();
        var cw = new SvgArc(new Point(1, 0), new Point(0, 1), 1, 1, 0, false, false).ToCenterArc();
        Assert.NotNull(ccw);
        Assert.NotNull(cw);
        Assert.True(ccw.Value.SweepAngle > 0);
        Assert.True(cw.Value.SweepAngle < 0);

        var large = new SvgArc(new Point(1, 0), new Point(-1, 0), 1, 1, 0, true, true).ToCenterArc();
        Assert.NotNull(large);
        Assert.True(Math.Abs(large.Value.SweepAngle) > TrigPackage.PI - 1e-6);
    }

    private static bool Close(double left, double right) => Math.Abs(left - right) < Epsilon;

    private static bool PointClose(Point left, Point right) => Close(left.X, right.X) && Close(left.Y, right.Y);
}
