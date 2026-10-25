using System.Text.Json;
using CodingAdventures.Point2D;

namespace CodingAdventures.Bezier2D.Tests;

public sealed class FlatteningConformanceTests
{
    private const double FixtureTolerance = 1e-12;

    [Fact]
    public void NeutralFlatteningCasesExceptBudgetConform()
    {
        using var document = JsonDocument.Parse(File.ReadAllText(FixturePath()));
        var root = document.RootElement;

        Assert.Equal(1, root.GetProperty("version").GetInt32());
        Assert.Equal(FixtureTolerance, root.GetProperty("absolute_tolerance").GetDouble());
        Assert.Equal(32, root.GetProperty("max_depth").GetInt32());
        Assert.Equal(65_535, root.GetProperty("max_subdivisions").GetInt32());

        foreach (var testCase in root.GetProperty("cases").EnumerateArray())
        {
            if (testCase.TryGetProperty("expected_termination", out _))
            {
                continue;
            }

            var id = testCase.GetProperty("id").GetString()!;
            var degree = testCase.GetProperty("degree").GetInt32();
            var tolerance = testCase.GetProperty("tolerance").GetDouble();
            var disposition = testCase.GetProperty("expected_disposition").GetString()!;
            var controls = ReadPoints(testCase.GetProperty("control_points"));

            if (disposition == "invalid-tolerance")
            {
                Assert.Throws<ArgumentOutOfRangeException>(() => Flatten(degree, controls, tolerance));
                continue;
            }

            var points = Flatten(degree, controls, tolerance);
            Assert.Equal(controls[0], points[0]);
            Assert.Equal(controls[^1], points[^1]);
            Assert.All(points, point => Assert.True(double.IsFinite(point.X) && double.IsFinite(point.Y), id));
            if (disposition == "chord")
            {
                Assert.Equal(2, points.Count);
            }
            else
            {
                Assert.True(points.Count > 2, id);
            }

            var witnessT = testCase.GetProperty("witness_t").GetDouble();
            var expectedWitness = ReadPoint(testCase.GetProperty("expected_witness"));
            var expectedDistance = testCase.GetProperty("expected_witness_distance").GetDouble();
            var actualWitness = Evaluate(degree, controls, witnessT);
            AssertPointClose(expectedWitness, actualWitness, id);
            Assert.True(Math.Abs(DistanceToSegment(actualWitness, controls[0], controls[^1]) - expectedDistance) <= FixtureTolerance, id);
        }
    }

    [Fact]
    public void BudgetExhaustionFailsDeterministically()
    {
        using var document = JsonDocument.Parse(File.ReadAllText(FixturePath()));
        var testCase = document.RootElement.GetProperty("cases").EnumerateArray()
            .Single(element => element.TryGetProperty("expected_termination", out _));
        var controls = ReadPoints(testCase.GetProperty("control_points"));

        Assert.Throws<InvalidOperationException>(() => Flatten(
            testCase.GetProperty("degree").GetInt32(),
            controls,
            testCase.GetProperty("tolerance").GetDouble()));
    }

    [Fact]
    public void NonfiniteControlsFailBeforeSubdivision()
    {
        var origin = new Point(0, 0);

        Assert.Throws<ArgumentOutOfRangeException>(() =>
            new QuadraticBezier(origin, new Point(double.NaN, 1), new Point(1, 0)).ToPolyline(0.1));
        Assert.Throws<ArgumentOutOfRangeException>(() =>
            new CubicBezier(origin, new Point(0, 1), new Point(double.PositiveInfinity, -1), origin).ToPolyline(0.1));
    }

    [Fact]
    public void FiniteInputsThatOverflowDerivedGeometryFailClosed()
    {
        var negative = new Point(-double.MaxValue / 2, 0);
        var positive = new Point(double.MaxValue / 2, 0);

        Assert.Throws<InvalidOperationException>(() =>
            new QuadraticBezier(negative, new Point(0, 1), positive).ToPolyline(0.1));
        Assert.Throws<InvalidOperationException>(() =>
            new CubicBezier(negative, new Point(0, 1), new Point(0, -1), positive).ToPolyline(0.1));
    }

    [Fact]
    public void DepthBudgetFailsBeforeReturningAPartialPolyline()
    {
        var quadratic = new QuadraticBezier(new Point(0, 0), new Point(0.5, 1), new Point(1, 0));
        var cubic = new CubicBezier(new Point(0, 0), new Point(0, 1), new Point(1, 1), new Point(1, 0));
        Assert.Throws<InvalidOperationException>(() => quadratic.ToPolyline(double.Epsilon));
        Assert.Throws<InvalidOperationException>(() => cubic.ToPolyline(double.Epsilon));
    }

    [Fact]
    public void BoundingBoxesIncludeInteriorXExtrema()
    {
        var quadratic = new QuadraticBezier(new Point(0, 0), new Point(10, 0), new Point(0, 1));
        var quadraticBounds = quadratic.BoundingBox();
        Assert.True(quadraticBounds.X + quadraticBounds.Width > 4.9);

        var cubic = new CubicBezier(new Point(0, 0), new Point(10, 0), new Point(-10, 0), new Point(0, 1));
        var cubicBounds = cubic.BoundingBox();
        Assert.True(cubicBounds.X < -2.8);
        Assert.True(cubicBounds.X + cubicBounds.Width > 2.8);
    }

    private static IReadOnlyList<Point> Flatten(int degree, IReadOnlyList<Point> controls, double tolerance) =>
        degree switch
        {
            2 => new QuadraticBezier(controls[0], controls[1], controls[2]).ToPolyline(tolerance),
            3 => new CubicBezier(controls[0], controls[1], controls[2], controls[3]).ToPolyline(tolerance),
            _ => throw new InvalidOperationException($"unsupported fixture degree {degree}"),
        };

    private static Point Evaluate(int degree, IReadOnlyList<Point> controls, double t) =>
        degree == 2
            ? new QuadraticBezier(controls[0], controls[1], controls[2]).Evaluate(t)
            : new CubicBezier(controls[0], controls[1], controls[2], controls[3]).Evaluate(t);

    private static List<Point> ReadPoints(JsonElement element) => element.EnumerateArray().Select(ReadPoint).ToList();

    private static Point ReadPoint(JsonElement element)
    {
        var coordinates = element.EnumerateArray().Select(value => value.GetDouble()).ToArray();
        return new Point(coordinates[0], coordinates[1]);
    }

    private static void AssertPointClose(Point expected, Point actual, string id)
    {
        Assert.True(Math.Abs(expected.X - actual.X) <= FixtureTolerance, id);
        Assert.True(Math.Abs(expected.Y - actual.Y) <= FixtureTolerance, id);
    }

    private static double DistanceToSegment(Point point, Point start, Point end)
    {
        var dx = end.X - start.X;
        var dy = end.Y - start.Y;
        var lengthSquared = dx * dx + dy * dy;
        if (lengthSquared == 0)
        {
            return Math.Sqrt(Math.Pow(point.X - start.X, 2) + Math.Pow(point.Y - start.Y, 2));
        }

        var projection = Math.Clamp(
            ((point.X - start.X) * dx + (point.Y - start.Y) * dy) / lengthSquared,
            0,
            1);
        var nearest = new Point(start.X + projection * dx, start.Y + projection * dy);
        return Math.Sqrt(Math.Pow(point.X - nearest.X, 2) + Math.Pow(point.Y - nearest.Y, 2));
    }

    private static string FixturePath()
    {
        for (var directory = new DirectoryInfo(Directory.GetCurrentDirectory()); directory is not null; directory = directory.Parent)
        {
            var candidate = Path.Combine(directory.FullName, "code", "specs", "fixtures", "bezier2d-flattening-v1", "cases.json");
            if (File.Exists(candidate))
            {
                return candidate;
            }
        }

        throw new FileNotFoundException("could not locate the checked-in G2D02 flattening corpus");
    }
}
