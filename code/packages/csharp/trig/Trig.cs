namespace CodingAdventures.Trig;

/// <summary>
/// Trigonometric functions from first principles.
///
/// The key teaching idea in this package is that "advanced" math functions do
/// not need magic CPU instructions. Sine and cosine come from Maclaurin
/// series, square root comes from Newton's method, and tangent/arctangent are
/// built on top of those primitives.
/// </summary>
public static class Trig
{
    /// <summary>
    /// Pi to double-precision accuracy.
    /// </summary>
    public const double PI = 3.141592653589793;

    private const double TwoPi = 2.0 * PI;
    private const double HalfPi = PI / 2.0;

    private static double RangeReduce(double x)
    {
        var reduced = x % TwoPi;
        if (reduced > PI)
        {
            reduced -= TwoPi;
        }

        if (reduced < -PI)
        {
            reduced += TwoPi;
        }

        return reduced;
    }

    /// <summary>
    /// Compute sine by summing the Maclaurin series after reducing the input
    /// into the range [-pi, pi].
    /// </summary>
    public static double Sin(double x)
    {
        var reduced = RangeReduce(x);
        var squared = reduced * reduced;
        var term = reduced;
        var sum = term;

        for (var k = 1; k < 20; k++)
        {
            var denominator = (2.0 * k) * (2.0 * k + 1.0);
            term *= -squared / denominator;
            sum += term;
        }

        return sum;
    }

    /// <summary>
    /// Compute cosine by summing the even-powered Maclaurin series.
    /// </summary>
    public static double Cos(double x)
    {
        var reduced = RangeReduce(x);
        var squared = reduced * reduced;
        var term = 1.0;
        var sum = term;

        for (var k = 1; k < 20; k++)
        {
            var denominator = (2.0 * k - 1.0) * (2.0 * k);
            term *= -squared / denominator;
            sum += term;
        }

        return sum;
    }

    /// <summary>
    /// Convert degrees to radians.
    /// </summary>
    public static double Radians(double degrees) => degrees * (PI / 180.0);

    /// <summary>
    /// Convert radians to degrees.
    /// </summary>
    public static double Degrees(double radians) => radians * (180.0 / PI);

    /// <summary>
    /// Compute a square root using Newton's method.
    /// </summary>
    public static double Sqrt(double x)
    {
        if (x < 0.0)
        {
            throw new ArgumentOutOfRangeException(nameof(x), x, "sqrt: domain error -- input is negative");
        }

        if (x == 0.0)
        {
            return 0.0;
        }

        var guess = x >= 1.0 ? x : 1.0;
        for (var i = 0; i < 60; i++)
        {
            var next = (guess + x / guess) / 2.0;
            if (Math.Abs(next - guess) < 1e-15 * guess + 1e-300)
            {
                return next;
            }

            guess = next;
        }

        return guess;
    }

    /// <summary>
    /// Compute tangent as the ratio of sine to cosine.
    /// </summary>
    public static double Tan(double x)
    {
        var sine = Sin(x);
        var cosine = Cos(x);
        if (Math.Abs(cosine) < 1e-15)
        {
            return sine > 0.0 ? 1.0e308 : -1.0e308;
        }

        return sine / cosine;
    }

    private static double AtanCore(double x)
    {
        // Half-angle reduction shrinks the series input so the alternating
        // arctangent series converges quickly.
        var reduced = x / (1.0 + Sqrt(1.0 + x * x));
        var squared = reduced * reduced;
        var term = reduced;
        var result = reduced;

        for (var n = 1; n <= 30; n++)
        {
            term = term * (-squared) * (2.0 * n - 1.0) / (2.0 * n + 1.0);
            result += term;

            if (Math.Abs(term) < 1e-17)
            {
                break;
            }
        }

        return 2.0 * result;
    }

    /// <summary>
    /// Compute arctangent with range reduction for large magnitudes.
    /// </summary>
    public static double Atan(double x)
    {
        if (x == 0.0)
        {
            return 0.0;
        }

        if (x > 1.0)
        {
            return HalfPi - AtanCore(1.0 / x);
        }

        if (x < -1.0)
        {
            return -HalfPi - AtanCore(1.0 / x);
        }

        return AtanCore(x);
    }

    /// <summary>
    /// Compute the four-quadrant arctangent in the range (-pi, pi].
    /// </summary>
    public static double Atan2(double y, double x)
    {
        if (x > 0.0)
        {
            return Atan(y / x);
        }

        if (x < 0.0 && y >= 0.0)
        {
            return Atan(y / x) + PI;
        }

        if (x < 0.0 && y < 0.0)
        {
            return Atan(y / x) - PI;
        }

        if (x == 0.0 && y > 0.0)
        {
            return HalfPi;
        }

        if (x == 0.0 && y < 0.0)
        {
            return -HalfPi;
        }

        return 0.0;
    }
}
