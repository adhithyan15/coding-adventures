using CodingAdventures.Point2D;
using System.Text.Json;
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
        Assert.NotNull(new SvgArc(Point.Origin(), Point.Origin(), 1, 1, 0, false, true).Evaluate(0));
        Assert.NotNull(new SvgArc(Point.Origin(), Point.Origin(), 1, 1, 0, false, true).BoundingBox());
    }

    [Fact]
    public void SvgArcConsumesNeutralDegenerateCases()
    {
        using var fixture = JsonDocument.Parse(File.ReadAllText(Path.Combine(AppContext.BaseDirectory, "geometry2d-v1-cases.json")));
        var root = fixture.RootElement;
        var tolerance = root.GetProperty("absolute_tolerance").GetDouble();
        var count = 0;
        foreach (var entry in root.GetProperty("cases").EnumerateArray())
        {
            if (entry.GetProperty("operation").GetString() != "svg-arc-degenerate")
            {
                continue;
            }

            count++;
            static Point ReadPoint(JsonElement element, string key)
            {
                var coordinates = element.GetProperty(key);
                return new Point(coordinates[0].GetDouble(), coordinates[1].GetDouble());
            }

            var arc = new SvgArc(ReadPoint(entry, "from"), ReadPoint(entry, "to"),
                entry.GetProperty("rx").GetDouble(), entry.GetProperty("ry").GetDouble(), 0, false, true);
            var id = entry.GetProperty("id").GetString();
            Assert.Null(arc.ToCenterArc());
            Assert.Empty(arc.ToCubicBeziers());
            var point = arc.Evaluate(entry.GetProperty("sample_t").GetDouble());
            Assert.NotNull(point);
            var expectedPoint = ReadPoint(entry, "expected_point");
            Assert.True(Math.Abs(point.Value.X - expectedPoint.X) <= tolerance, id);
            Assert.True(Math.Abs(point.Value.Y - expectedPoint.Y) <= tolerance, id);
            var bounds = arc.BoundingBox();
            Assert.NotNull(bounds);
            var expectedBounds = entry.GetProperty("expected_bounds");
            var actual = bounds.Value;
            Assert.True(Math.Abs(actual.X - expectedBounds[0].GetDouble()) <= tolerance, id);
            Assert.True(Math.Abs(actual.Y - expectedBounds[1].GetDouble()) <= tolerance, id);
            Assert.True(Math.Abs(actual.Width - expectedBounds[2].GetDouble()) <= tolerance, id);
            Assert.True(Math.Abs(actual.Height - expectedBounds[3].GetDouble()) <= tolerance, id);
        }

        Assert.Equal(4, count);
    }

    [Fact]
    public void SvgArcKeepsStrictDegenerateBoundariesAndSignedRadius()
    {
        var origin = Point.Origin();
        foreach (var arc in new[]
        {
            new SvgArc(origin, new Point(1, 0), 1e-10, 1, 0, false, true),
            new SvgArc(origin, new Point(1e-10, 0), 1, 1, 0, false, true),
            new SvgArc(origin, new Point(8e-11, 8e-11), 1, 1, 0, false, true),
            new SvgArc(origin, new Point(1, 0), -1e-10, 1, 0, false, true),
        })
        {
            var center = arc.ToCenterArc();
            Assert.NotNull(center);
            Assert.All(new[] { center.Value.Center.X, center.Value.Center.Y, center.Value.Rx,
                center.Value.Ry, center.Value.StartAngle, center.Value.SweepAngle,
                center.Value.XRotation }, value => Assert.True(double.IsFinite(value)));
        }
        var positive = new SvgArc(new Point(1, 0), new Point(0, 1), 1, 1, 0, false, true);
        var negative = positive with { Rx = -1 };
        var p = positive.Evaluate(0.25);
        var n = negative.Evaluate(0.25);
        Assert.NotNull(p);
        Assert.NotNull(n);
        Assert.True(Math.Abs(p.Value.X - n.Value.X) < 1e-12);
        Assert.True(Math.Abs(p.Value.Y - n.Value.Y) < 1e-12);
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
