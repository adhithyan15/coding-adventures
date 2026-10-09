using CodingAdventures.Bezier2D;
using CodingAdventures.Point2D;

namespace CodingAdventures.Arc2D;

public readonly record struct CenterArc(
    Point Center,
    double Rx,
    double Ry,
    double StartAngle,
    double SweepAngle,
    double XRotation)
{
    public Point Evaluate(double t)
    {
        Validate();
        RequireFinite(t);
        var angle = StartAngle + t * SweepAngle;
        var xp = Rx * CodingAdventures.Trig.Trig.Cos(angle);
        var yp = Ry * CodingAdventures.Trig.Trig.Sin(angle);
        var cosineRotation = CodingAdventures.Trig.Trig.Cos(XRotation);
        var sineRotation = CodingAdventures.Trig.Trig.Sin(XRotation);

        return RequireFinite(new Point(
            cosineRotation * xp - sineRotation * yp + Center.X,
            sineRotation * xp + cosineRotation * yp + Center.Y));
    }

    public Point Tangent(double t)
    {
        Validate();
        RequireFinite(t);
        var angle = StartAngle + t * SweepAngle;
        var dxp = -Rx * CodingAdventures.Trig.Trig.Sin(angle) * SweepAngle;
        var dyp = Ry * CodingAdventures.Trig.Trig.Cos(angle) * SweepAngle;
        var cosineRotation = CodingAdventures.Trig.Trig.Cos(XRotation);
        var sineRotation = CodingAdventures.Trig.Trig.Sin(XRotation);

        return RequireFinite(new Point(
            cosineRotation * dxp - sineRotation * dyp,
            sineRotation * dxp + cosineRotation * dyp));
    }

    public Rect BoundingBox()
    {
        Validate();
        var cosineRotation = CodingAdventures.Trig.Trig.Cos(XRotation);
        var sineRotation = CodingAdventures.Trig.Trig.Sin(XRotation);
        var xExtremum = CodingAdventures.Trig.Trig.Atan2(-Ry * sineRotation, Rx * cosineRotation);
        var yExtremum = CodingAdventures.Trig.Trig.Atan2(Ry * cosineRotation, Rx * sineRotation);

        // Endpoints are always present. Each stationary angle has an opposite
        // partner, but only candidates inside the directed sweep can bound it.
        var first = AtAngle(StartAngle, cosineRotation, sineRotation);
        var last = AtAngle(StartAngle + SweepAngle, cosineRotation, sineRotation);
        var minX = Math.Min(first.X, last.X);
        var minY = Math.Min(first.Y, last.Y);
        var maxX = Math.Max(first.X, last.X);
        var maxY = Math.Max(first.Y, last.Y);
        foreach (var angle in new[]
            { xExtremum, xExtremum + CodingAdventures.Trig.Trig.PI,
              yExtremum, yExtremum + CodingAdventures.Trig.Trig.PI })
        {
            if (!ContainsAngle(angle))
            {
                continue;
            }

            var point = AtAngle(angle, cosineRotation, sineRotation);
            minX = Math.Min(minX, point.X);
            maxX = Math.Max(maxX, point.X);
            minY = Math.Min(minY, point.Y);
            maxY = Math.Max(maxY, point.Y);
        }

        var width = maxX - minX;
        var height = maxY - minY;
        RequireFinite(width);
        RequireFinite(height);
        return new Rect(minX, minY, width, height);

    }

    public IReadOnlyList<CubicBezier> ToCubicBeziers()
    {
        Validate();
        var maxSegment = CodingAdventures.Trig.Trig.PI / 2.0;
        var segmentCount = Math.Max(1, (int)Math.Ceiling(Math.Abs(SweepAngle) / maxSegment));
        var segmentSweep = SweepAngle / segmentCount;
        var cosineRotation = CodingAdventures.Trig.Trig.Cos(XRotation);
        var sineRotation = CodingAdventures.Trig.Trig.Sin(XRotation);
        var centerX = Center.X;
        var centerY = Center.Y;
        var k = (4.0 / 3.0) * CodingAdventures.Trig.Trig.Tan(segmentSweep / 4.0);
        var beziers = new List<CubicBezier>(segmentCount);

        for (var index = 0; index < segmentCount; index++)
        {
            var alpha = StartAngle + index * segmentSweep;
            var beta = alpha + segmentSweep;
            var cosAlpha = CodingAdventures.Trig.Trig.Cos(alpha);
            var sinAlpha = CodingAdventures.Trig.Trig.Sin(alpha);
            var cosBeta = CodingAdventures.Trig.Trig.Cos(beta);
            var sinBeta = CodingAdventures.Trig.Trig.Sin(beta);

            var p0Local = (X: Rx * cosAlpha, Y: Ry * sinAlpha);
            var p3Local = (X: Rx * cosBeta, Y: Ry * sinBeta);
            var p1Local = (X: p0Local.X + k * (-Rx * sinAlpha), Y: p0Local.Y + k * (Ry * cosAlpha));
            var p2Local = (X: p3Local.X - k * (-Rx * sinBeta), Y: p3Local.Y - k * (Ry * cosBeta));

            beziers.Add(new CubicBezier(
                RotateTranslate(p0Local.X, p0Local.Y),
                RotateTranslate(p1Local.X, p1Local.Y),
                RotateTranslate(p2Local.X, p2Local.Y),
                RotateTranslate(p3Local.X, p3Local.Y)));
        }

        return beziers;

        Point RotateTranslate(double localX, double localY) =>
            RequireFinite(new Point(
                cosineRotation * localX - sineRotation * localY + centerX,
                sineRotation * localX + cosineRotation * localY + centerY));
    }

    private void Validate()
    {
        if (!double.IsFinite(Center.X) || !double.IsFinite(Center.Y)
            || !double.IsFinite(Rx) || !double.IsFinite(Ry)
            || !double.IsFinite(StartAngle) || !double.IsFinite(SweepAngle)
            || !double.IsFinite(XRotation) || Rx <= 0 || Ry <= 0)
        {
            throw new ArgumentException("Center arc requires finite coordinates, angles and positive radii.");
        }

        if (Math.Abs(SweepAngle) > 2.0 * CodingAdventures.Trig.Trig.PI)
        {
            throw new ArgumentOutOfRangeException(nameof(SweepAngle), "Center arc sweep exceeds one turn.");
        }
    }

    private Point AtAngle(double angle, double cosineRotation, double sineRotation)
    {
        var localX = Rx * CodingAdventures.Trig.Trig.Cos(angle);
        var localY = Ry * CodingAdventures.Trig.Trig.Sin(angle);
        return RequireFinite(new Point(
            cosineRotation * localX - sineRotation * localY + Center.X,
            sineRotation * localX + cosineRotation * localY + Center.Y));
    }

    private bool ContainsAngle(double angle)
    {
        var turn = 2.0 * CodingAdventures.Trig.Trig.PI;
        if (SweepAngle > 0)
        {
            return PositiveMod(angle - StartAngle, turn) <= SweepAngle;
        }

        return SweepAngle < 0 && PositiveMod(StartAngle - angle, turn) <= -SweepAngle;
    }

    private static double PositiveMod(double value, double modulus)
    {
        var remainder = value % modulus;
        return remainder < 0 ? remainder + modulus : remainder;
    }

    private static void RequireFinite(double value)
    {
        if (!double.IsFinite(value))
        {
            throw new ArgumentException("Center arc produced a non-finite value.");
        }
    }

    private static Point RequireFinite(Point point)
    {
        RequireFinite(point.X);
        RequireFinite(point.Y);
        return point;
    }
}

public readonly record struct SvgArc(
    Point From,
    Point To,
    double Rx,
    double Ry,
    double XRotation,
    bool LargeArc,
    bool Sweep)
{
    public CenterArc? ToCenterArc()
    {
        if (From.DistanceSquared(To) < 1e-20)
        {
            return null;
        }

        if (Math.Abs(Rx) < 1e-10 || Math.Abs(Ry) < 1e-10)
        {
            return null;
        }

        var cosineRotation = CodingAdventures.Trig.Trig.Cos(XRotation);
        var sineRotation = CodingAdventures.Trig.Trig.Sin(XRotation);
        var dx = (From.X - To.X) / 2.0;
        var dy = (From.Y - To.Y) / 2.0;
        var x1Prime = cosineRotation * dx + sineRotation * dy;
        var y1Prime = -sineRotation * dx + cosineRotation * dy;
        var rx = Math.Abs(Rx);
        var ry = Math.Abs(Ry);

        var lambda = (x1Prime / rx) * (x1Prime / rx) + (y1Prime / ry) * (y1Prime / ry);
        if (lambda > 1.0)
        {
            var squareRootLambda = CodingAdventures.Trig.Trig.Sqrt(lambda);
            rx *= squareRootLambda;
            ry *= squareRootLambda;
        }

        var rxSquared = rx * rx;
        var rySquared = ry * ry;
        var x1PrimeSquared = x1Prime * x1Prime;
        var y1PrimeSquared = y1Prime * y1Prime;
        var numerator = rxSquared * rySquared - rxSquared * y1PrimeSquared - rySquared * x1PrimeSquared;
        var denominator = rxSquared * y1PrimeSquared + rySquared * x1PrimeSquared;
        var squareRoot = Math.Abs(denominator) < 1e-12
            ? 0.0
            : CodingAdventures.Trig.Trig.Sqrt(Math.Max(0.0, numerator / denominator));
        var sign = LargeArc == Sweep ? -1.0 : 1.0;
        var cxPrime = sign * squareRoot * (rx * y1Prime / ry);
        var cyPrime = sign * squareRoot * -(ry * x1Prime / rx);

        var midX = (From.X + To.X) / 2.0;
        var midY = (From.Y + To.Y) / 2.0;
        var cx = cosineRotation * cxPrime - sineRotation * cyPrime + midX;
        var cy = sineRotation * cxPrime + cosineRotation * cyPrime + midY;

        var ux = (x1Prime - cxPrime) / rx;
        var uy = (y1Prime - cyPrime) / ry;
        var vx = (-x1Prime - cxPrime) / rx;
        var vy = (-y1Prime - cyPrime) / ry;
        var startAngle = AngleBetween(1.0, 0.0, ux, uy);
        var sweepAngle = AngleBetween(ux, uy, vx, vy);

        if (!Sweep && sweepAngle > 0.0)
        {
            sweepAngle -= 2.0 * CodingAdventures.Trig.Trig.PI;
        }

        if (Sweep && sweepAngle < 0.0)
        {
            sweepAngle += 2.0 * CodingAdventures.Trig.Trig.PI;
        }

        return new CenterArc(new Point(cx, cy), rx, ry, startAngle, sweepAngle, XRotation);
    }

    public IReadOnlyList<CubicBezier> ToCubicBeziers() => ToCenterArc()?.ToCubicBeziers() ?? [];

    public Point? Evaluate(double t) => ToCenterArc()?.Evaluate(t) ?? From.Lerp(To, t);

    public Rect? BoundingBox() => ToCenterArc()?.BoundingBox() ?? Rect.FromPoints(
        new Point(Math.Min(From.X, To.X), Math.Min(From.Y, To.Y)),
        new Point(Math.Max(From.X, To.X), Math.Max(From.Y, To.Y)));

    private static double AngleBetween(double ux, double uy, double vx, double vy) =>
        CodingAdventures.Trig.Trig.Atan2(ux * vy - uy * vx, ux * vx + uy * vy);
}

public static class Arc2D
{
    public const string Version = "0.1.0";
}
