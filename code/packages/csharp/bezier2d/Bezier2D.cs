using CodingAdventures.Point2D;

namespace CodingAdventures.Bezier2D;

internal static class Flattening
{
    internal const int MaxDepth = 32;
    internal const int MaxSubdivisions = 65_535;

    internal static void ValidateInputs(double tolerance, params Point[] controlPoints)
    {
        if (!double.IsFinite(tolerance) || tolerance <= 0.0)
        {
            throw new ArgumentOutOfRangeException(nameof(tolerance), tolerance, "tolerance must be finite and positive");
        }

        foreach (var point in controlPoints)
        {
            if (!double.IsFinite(point.X) || !double.IsFinite(point.Y))
            {
                throw new ArgumentOutOfRangeException(nameof(controlPoints), point, "control points must be finite");
            }
        }
    }

    internal static double DistanceToSegment(Point point, Point start, Point end)
    {
        var dx = Finite(end.X - start.X);
        var dy = Finite(end.Y - start.Y);
        var lengthSquared = Finite(dx * dx + dy * dy);

        if (lengthSquared == 0.0)
        {
            return Distance(point, start);
        }

        var offsetX = Finite(point.X - start.X);
        var offsetY = Finite(point.Y - start.Y);
        var projection = Finite((offsetX * dx + offsetY * dy) / lengthSquared);
        projection = Math.Clamp(projection, 0.0, 1.0);
        var nearest = new Point(
            Finite(start.X + projection * dx),
            Finite(start.Y + projection * dy));
        return Distance(point, nearest);
    }

    internal static InvalidOperationException BudgetExceeded() =>
        new("Bezier flattening exceeded the depth or subdivision budget");

    private static double Distance(Point left, Point right)
    {
        var dx = Finite(left.X - right.X);
        var dy = Finite(left.Y - right.Y);
        return Finite(Math.Sqrt(Finite(dx * dx + dy * dy)));
    }

    private static double Finite(double value) =>
        double.IsFinite(value)
            ? value
            : throw new InvalidOperationException("Bezier subdivision produced nonfinite geometry");
}

public readonly record struct QuadraticBezier(Point P0, Point P1, Point P2)
{
    public Point Evaluate(double t)
    {
        var q0 = P0.Lerp(P1, t);
        var q1 = P1.Lerp(P2, t);
        return q0.Lerp(q1, t);
    }

    public Point Derivative(double t)
    {
        var d0 = P1.Subtract(P0);
        var d1 = P2.Subtract(P1);
        return d0.Lerp(d1, t).Scale(2.0);
    }

    public (QuadraticBezier Left, QuadraticBezier Right) Split(double t)
    {
        var q0 = P0.Lerp(P1, t);
        var q1 = P1.Lerp(P2, t);
        var midpoint = q0.Lerp(q1, t);
        return (new QuadraticBezier(P0, q0, midpoint), new QuadraticBezier(midpoint, q1, P2));
    }

    public IReadOnlyList<Point> ToPolyline(double tolerance)
    {
        Flattening.ValidateInputs(tolerance, P0, P1, P2);
        var points = new List<Point> { P0 };
        var pending = new Stack<(QuadraticBezier Curve, int Depth)>();
        pending.Push((this, 0));
        var subdivisions = 0;

        while (pending.Count > 0)
        {
            var (curve, depth) = pending.Pop();
            var error = Flattening.DistanceToSegment(curve.P1, curve.P0, curve.P2);
            if (error <= tolerance)
            {
                points.Add(curve.P2);
                continue;
            }

            if (depth >= Flattening.MaxDepth || subdivisions >= Flattening.MaxSubdivisions)
            {
                throw Flattening.BudgetExceeded();
            }

            subdivisions++;
            var (left, right) = curve.Split(0.5);
            pending.Push((right, depth + 1));
            pending.Push((left, depth + 1));
        }

        return points;
    }

    public Rect BoundingBox()
    {
        var minX = Math.Min(P0.X, P2.X);
        var maxX = Math.Max(P0.X, P2.X);
        var minY = Math.Min(P0.Y, P2.Y);
        var maxY = Math.Max(P0.Y, P2.Y);

        var denomX = P0.X - 2.0 * P1.X + P2.X;
        if (Math.Abs(denomX) > 1e-12)
        {
            var tx = (P0.X - P1.X) / denomX;
            if (tx is > 0.0 and < 1.0)
            {
                var px = Evaluate(tx);
                minX = Math.Min(minX, px.X);
                maxX = Math.Max(maxX, px.X);
            }
        }

        var denomY = P0.Y - 2.0 * P1.Y + P2.Y;
        if (Math.Abs(denomY) > 1e-12)
        {
            var ty = (P0.Y - P1.Y) / denomY;
            if (ty is > 0.0 and < 1.0)
            {
                var py = Evaluate(ty);
                minY = Math.Min(minY, py.Y);
                maxY = Math.Max(maxY, py.Y);
            }
        }

        return new Rect(minX, minY, maxX - minX, maxY - minY);
    }

    public CubicBezier Elevate()
    {
        var q1 = P0.Scale(1.0 / 3.0).Add(P1.Scale(2.0 / 3.0));
        var q2 = P1.Scale(2.0 / 3.0).Add(P2.Scale(1.0 / 3.0));
        return new CubicBezier(P0, q1, q2, P2);
    }

}

public readonly record struct CubicBezier(Point P0, Point P1, Point P2, Point P3)
{
    public Point Evaluate(double t)
    {
        var p01 = P0.Lerp(P1, t);
        var p12 = P1.Lerp(P2, t);
        var p23 = P2.Lerp(P3, t);
        var p012 = p01.Lerp(p12, t);
        var p123 = p12.Lerp(p23, t);
        return p012.Lerp(p123, t);
    }

    public Point Derivative(double t)
    {
        var d0 = P1.Subtract(P0);
        var d1 = P2.Subtract(P1);
        var d2 = P3.Subtract(P2);
        var oneMinusT = 1.0 - t;
        return d0
            .Scale(oneMinusT * oneMinusT)
            .Add(d1.Scale(2.0 * oneMinusT * t))
            .Add(d2.Scale(t * t))
            .Scale(3.0);
    }

    public (CubicBezier Left, CubicBezier Right) Split(double t)
    {
        var p01 = P0.Lerp(P1, t);
        var p12 = P1.Lerp(P2, t);
        var p23 = P2.Lerp(P3, t);
        var p012 = p01.Lerp(p12, t);
        var p123 = p12.Lerp(p23, t);
        var p0123 = p012.Lerp(p123, t);

        return (
            new CubicBezier(P0, p01, p012, p0123),
            new CubicBezier(p0123, p123, p23, P3));
    }

    public IReadOnlyList<Point> ToPolyline(double tolerance)
    {
        Flattening.ValidateInputs(tolerance, P0, P1, P2, P3);
        var points = new List<Point> { P0 };
        var pending = new Stack<(CubicBezier Curve, int Depth)>();
        pending.Push((this, 0));
        var subdivisions = 0;

        while (pending.Count > 0)
        {
            var (curve, depth) = pending.Pop();
            var error = Math.Max(
                Flattening.DistanceToSegment(curve.P1, curve.P0, curve.P3),
                Flattening.DistanceToSegment(curve.P2, curve.P0, curve.P3));
            if (error <= tolerance)
            {
                points.Add(curve.P3);
                continue;
            }

            if (depth >= Flattening.MaxDepth || subdivisions >= Flattening.MaxSubdivisions)
            {
                throw Flattening.BudgetExceeded();
            }

            subdivisions++;
            var (left, right) = curve.Split(0.5);
            pending.Push((right, depth + 1));
            pending.Push((left, depth + 1));
        }

        return points;
    }

    public Rect BoundingBox()
    {
        var minX = Math.Min(P0.X, P3.X);
        var maxX = Math.Max(P0.X, P3.X);
        var minY = Math.Min(P0.Y, P3.Y);
        var maxY = Math.Max(P0.Y, P3.Y);

        foreach (var t in ExtremaOfCubicDerivative(P0.X, P1.X, P2.X, P3.X))
        {
            var px = Evaluate(t);
            minX = Math.Min(minX, px.X);
            maxX = Math.Max(maxX, px.X);
        }

        foreach (var t in ExtremaOfCubicDerivative(P0.Y, P1.Y, P2.Y, P3.Y))
        {
            var py = Evaluate(t);
            minY = Math.Min(minY, py.Y);
            maxY = Math.Max(maxY, py.Y);
        }

        return new Rect(minX, minY, maxX - minX, maxY - minY);
    }

    private static IReadOnlyList<double> ExtremaOfCubicDerivative(double v0, double v1, double v2, double v3)
    {
        var a = -3.0 * v0 + 9.0 * v1 - 9.0 * v2 + 3.0 * v3;
        var b = 6.0 * v0 - 12.0 * v1 + 6.0 * v2;
        var c = -3.0 * v0 + 3.0 * v1;
        var roots = new List<double>();

        if (Math.Abs(a) < 1e-12)
        {
            if (Math.Abs(b) > 1e-12)
            {
                var t = -c / b;
                if (t is > 0.0 and < 1.0)
                {
                    roots.Add(t);
                }
            }
        }
        else
        {
            var discriminant = b * b - 4.0 * a * c;
            if (discriminant >= 0.0)
            {
                var squareRoot = CodingAdventures.Trig.Trig.Sqrt(discriminant);
                var t1 = (-b + squareRoot) / (2.0 * a);
                var t2 = (-b - squareRoot) / (2.0 * a);

                if (t1 is > 0.0 and < 1.0)
                {
                    roots.Add(t1);
                }

                if (t2 is > 0.0 and < 1.0)
                {
                    roots.Add(t2);
                }
            }
        }

        return roots;
    }
}

public static class Bezier2D
{
    public const string Version = "0.1.0";
}
