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
        var angle = StartAngle + t * SweepAngle;
        var xp = Rx * CodingAdventures.Trig.Trig.Cos(angle);
        var yp = Ry * CodingAdventures.Trig.Trig.Sin(angle);
        var cosineRotation = CodingAdventures.Trig.Trig.Cos(XRotation);
        var sineRotation = CodingAdventures.Trig.Trig.Sin(XRotation);

        return new Point(
            cosineRotation * xp - sineRotation * yp + Center.X,
            sineRotation * xp + cosineRotation * yp + Center.Y);
    }

    public Point Tangent(double t)
    {
        var angle = StartAngle + t * SweepAngle;
        var dxp = -Rx * CodingAdventures.Trig.Trig.Sin(angle) * SweepAngle;
        var dyp = Ry * CodingAdventures.Trig.Trig.Cos(angle) * SweepAngle;
        var cosineRotation = CodingAdventures.Trig.Trig.Cos(XRotation);
        var sineRotation = CodingAdventures.Trig.Trig.Sin(XRotation);

        return new Point(
            cosineRotation * dxp - sineRotation * dyp,
            sineRotation * dxp + cosineRotation * dyp);
    }

    public Rect BoundingBox()
    {
        const int samples = 100;
        var minX = double.PositiveInfinity;
        var minY = double.PositiveInfinity;
        var maxX = double.NegativeInfinity;
        var maxY = double.NegativeInfinity;

        for (var index = 0; index <= samples; index++)
        {
            var point = Evaluate(index / (double)samples);
            minX = Math.Min(minX, point.X);
            maxX = Math.Max(maxX, point.X);
            minY = Math.Min(minY, point.Y);
            maxY = Math.Max(maxY, point.Y);
        }

        return new Rect(minX, minY, maxX - minX, maxY - minY);
    }

    public IReadOnlyList<CubicBezier> ToCubicBeziers()
    {
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
            new(
                cosineRotation * localX - sineRotation * localY + centerX,
                sineRotation * localX + cosineRotation * localY + centerY);
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
        if (Math.Abs(From.X - To.X) < 1e-12 && Math.Abs(From.Y - To.Y) < 1e-12)
        {
            return null;
        }

        if (Math.Abs(Rx) < 1e-12 || Math.Abs(Ry) < 1e-12)
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

    public Point? Evaluate(double t) => ToCenterArc()?.Evaluate(t);

    public Rect? BoundingBox() => ToCenterArc()?.BoundingBox();

    private static double AngleBetween(double ux, double uy, double vx, double vy) =>
        CodingAdventures.Trig.Trig.Atan2(ux * vy - uy * vx, ux * vx + uy * vy);
}

public static class Arc2D
{
    public const string Version = "0.1.0";
}
