namespace CodingAdventures.Point2D;

public readonly record struct Point(double X, double Y)
{
    public static Point Origin() => new(0.0, 0.0);

    public Point Add(Point other) => new(X + other.X, Y + other.Y);

    public Point Subtract(Point other) => new(X - other.X, Y - other.Y);

    public Point Scale(double scalar) => new(X * scalar, Y * scalar);

    public Point Negate() => new(-X, -Y);

    public double Dot(Point other) => X * other.X + Y * other.Y;

    public double Cross(Point other) => X * other.Y - Y * other.X;

    public double Magnitude() => Math.Sqrt(MagnitudeSquared());

    public double MagnitudeSquared() => X * X + Y * Y;

    public Point Normalize()
    {
        var magnitude = Magnitude();
        return magnitude < 1e-12 ? Origin() : new Point(X / magnitude, Y / magnitude);
    }

    public double Distance(Point other) => Subtract(other).Magnitude();

    public double DistanceSquared(Point other) => Subtract(other).MagnitudeSquared();

    public Point Lerp(Point other, double t) => new(X + (other.X - X) * t, Y + (other.Y - Y) * t);

    public Point Perpendicular() => new(-Y, X);

    public double Angle() => Math.Atan2(Y, X);
}

public readonly record struct Rect(double X, double Y, double Width, double Height)
{
    public static Rect Zero() => new(0.0, 0.0, 0.0, 0.0);

    public static Rect FromPoints(Point min, Point max) => new(min.X, min.Y, max.X - min.X, max.Y - min.Y);

    public Point MinPoint() => new(X, Y);

    public Point MaxPoint() => new(X + Width, Y + Height);

    public Point Center() => new(X + Width * 0.5, Y + Height * 0.5);

    public bool IsEmpty() => Width <= 0.0 || Height <= 0.0;

    public bool ContainsPoint(Point point) =>
        !IsEmpty()
        && point.X >= X
        && point.Y >= Y
        && point.X < X + Width
        && point.Y < Y + Height;

    public Rect Union(Rect other)
    {
        if (IsEmpty())
        {
            return other;
        }

        if (other.IsEmpty())
        {
            return this;
        }

        var minX = Math.Min(X, other.X);
        var minY = Math.Min(Y, other.Y);
        var maxX = Math.Max(X + Width, other.X + other.Width);
        var maxY = Math.Max(Y + Height, other.Y + other.Height);
        return new Rect(minX, minY, maxX - minX, maxY - minY);
    }

    public Rect? Intersection(Rect other)
    {
        var minX = Math.Max(X, other.X);
        var minY = Math.Max(Y, other.Y);
        var maxX = Math.Min(X + Width, other.X + other.Width);
        var maxY = Math.Min(Y + Height, other.Y + other.Height);
        var width = maxX - minX;
        var height = maxY - minY;
        return width <= 0.0 || height <= 0.0 ? null : new Rect(minX, minY, width, height);
    }

    public Rect ExpandBy(double amount) => new(X - amount, Y - amount, Width + 2.0 * amount, Height + 2.0 * amount);
}
