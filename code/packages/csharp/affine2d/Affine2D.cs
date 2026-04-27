using CodingAdventures.Point2D;

namespace CodingAdventures.Affine2D;

public readonly record struct Affine2D(double A, double B, double C, double D, double E, double F)
{
    public static Affine2D Identity() => new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);

    public static Affine2D Translate(double tx, double ty) => new(1.0, 0.0, 0.0, 1.0, tx, ty);

    public static Affine2D Rotate(double angle)
    {
        var cosine = CodingAdventures.Trig.Trig.Cos(angle);
        var sine = CodingAdventures.Trig.Trig.Sin(angle);
        return new Affine2D(cosine, sine, -sine, cosine, 0.0, 0.0);
    }

    public static Affine2D RotateAround(Point center, double angle) =>
        Translate(-center.X, -center.Y)
            .Then(Rotate(angle))
            .Then(Translate(center.X, center.Y));

    public static Affine2D Scale(double sx, double sy) => new(sx, 0.0, 0.0, sy, 0.0, 0.0);

    public static Affine2D ScaleUniform(double scale) => Scale(scale, scale);

    public static Affine2D SkewX(double angle) => new(1.0, 0.0, CodingAdventures.Trig.Trig.Tan(angle), 1.0, 0.0, 0.0);

    public static Affine2D SkewY(double angle) => new(1.0, CodingAdventures.Trig.Trig.Tan(angle), 0.0, 1.0, 0.0, 0.0);

    public Affine2D Then(Affine2D next) => next.Multiply(this);

    public Affine2D Multiply(Affine2D other) =>
        new(
            A * other.A + C * other.B,
            B * other.A + D * other.B,
            A * other.C + C * other.D,
            B * other.C + D * other.D,
            A * other.E + C * other.F + E,
            B * other.E + D * other.F + F);

    public Point ApplyToPoint(Point point) =>
        new(
            A * point.X + C * point.Y + E,
            B * point.X + D * point.Y + F);

    public Point ApplyToVector(Point vector) =>
        new(
            A * vector.X + C * vector.Y,
            B * vector.X + D * vector.Y);

    public double Determinant() => A * D - B * C;

    public Affine2D? Invert()
    {
        var determinant = Determinant();
        if (Math.Abs(determinant) < 1e-12)
        {
            return null;
        }

        return new Affine2D(
            D / determinant,
            -B / determinant,
            -C / determinant,
            A / determinant,
            (C * F - D * E) / determinant,
            (B * E - A * F) / determinant);
    }

    public bool IsIdentity()
    {
        const double epsilon = 1e-10;
        return Math.Abs(A - 1.0) < epsilon
            && Math.Abs(B) < epsilon
            && Math.Abs(C) < epsilon
            && Math.Abs(D - 1.0) < epsilon
            && Math.Abs(E) < epsilon
            && Math.Abs(F) < epsilon;
    }

    public bool IsTranslationOnly()
    {
        const double epsilon = 1e-10;
        return Math.Abs(A - 1.0) < epsilon
            && Math.Abs(B) < epsilon
            && Math.Abs(C) < epsilon
            && Math.Abs(D - 1.0) < epsilon;
    }

    public double[] ToArray() => [A, B, C, D, E, F];
}

public static class Affine2DPackage
{
    public const string Version = "0.1.0";
}
