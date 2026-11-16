namespace CodingAdventures.Trig;

/// <summary>
/// Trigonometric functions from first principles.
///
/// The key teaching idea in this package is that "advanced" math functions do
/// not need magic CPU instructions. Sine and cosine come from Maclaurin
/// series (stored as Horner coefficients), square root comes from Newton's
/// method, and tangent/arctangent are built on top of those primitives.
///
/// The arithmetic is the PHY00a reference algorithm TRIG-REF-1: every port in
/// every language performs the same IEEE operations in the same order, so the
/// results agree bit for bit (code/specs/PHY00a-trig-convergence.md).
/// </summary>
public static class Trig
{
    // ------------------------------------------------------------------
    // Constants (PHY00a 3.2).  Every constant is built from its IEEE bit
    // pattern, so no compiler's decimal parser is trusted; the tests assert
    // each one against the fixture manifest.
    // ------------------------------------------------------------------

    private static double F(long bits) => BitConverter.Int64BitsToDouble(bits);

    /// <summary>
    /// Pi to double-precision accuracy (0x400921FB54442D18).  Kept a
    /// <c>const</c> (it is public API); its bits are asserted by the tests.
    /// </summary>
    public const double PI = 3.141592653589793;

    internal const double TwoPi = 2.0 * PI;       // 0x401921FB54442D18
    internal const double HalfPi = PI / 2.0;      // 0x3FF921FB54442D18

    /// <summary>The double nearest 2/pi: turns an angle into a count of quarter-turns.</summary>
    internal static readonly double InvPio2 = F(0x3FE45F306DC9C883);

    /// <summary>
    /// pi/2 in five pieces (PHY00a 3.2).  The first four carry at most 24
    /// significant bits each, so k * piece is EXACT for |k| below 2^29
    /// (24 + 29 = 53 bits); the fifth is the remainder rounded to 53 bits.
    /// </summary>
    internal static readonly double[] Pio2 =
    [
        F(0x3FF921FB40000000),
        F(0x3E74442D00000000),
        F(0x3CF8469880000000),
        F(0x3B78CC5160000000),
        F(0x39F01B839A25204A),
    ];

    /// <summary>1.5 * 2^52: adding then subtracting it rounds to the nearest integer.</summary>
    internal static readonly double Magic = F(0x4338000000000000);

    /// <summary>2^29: the edge of the domain; beyond it sin, cos and tan are NaN.</summary>
    internal static readonly double Limit = F(0x41C0000000000000);

    /// <summary>2^-27: below it sin(x) = x and cos(x) = 1 exactly.</summary>
    internal static readonly double Tiny = F(0x3E40000000000000);

    /// <summary>
    /// S[n] = (-1)^n / (2n+1)!  for n = 1..9 (index 0 unused), each rounded once.
    ///
    ///   S1 = -1/3!   S2 = +1/5!   S3 = -1/7!   S4 = +1/9!   S5 = -1/11!
    ///   S6 = +1/13!  S7 = -1/15!  S8 = +1/17!  S9 = -1/19!
    /// </summary>
    internal static readonly double[] S =
    [
        0.0,
        F(unchecked((long)0xBFC5555555555555)),
        F(0x3F81111111111111),
        F(unchecked((long)0xBF2A01A01A01A01A)),
        F(0x3EC71DE3A556C734),
        F(unchecked((long)0xBE5AE64567F544E4)),
        F(0x3DE6124613A86D09),
        F(unchecked((long)0xBD6AE7F3E733B81F)),
        F(0x3CE952C77030AD4A),
        F(unchecked((long)0xBC62F49B46814157)),
    ];

    /// <summary>
    /// C[n] = (-1)^n / (2n)!  for n = 2..9 (indices 0 and 1 unused; the n = 1
    /// coefficient, -1/2, is applied as 0.5 * z, which is exact).
    ///
    ///   C2 = +1/4!   C3 = -1/6!   C4 = +1/8!   C5 = -1/10!
    ///   C6 = +1/12!  C7 = -1/14!  C8 = +1/16!  C9 = -1/18!
    /// </summary>
    internal static readonly double[] C =
    [
        0.0,
        0.0,
        F(0x3FA5555555555555),
        F(unchecked((long)0xBF56C16C16C16C17)),
        F(0x3EFA01A01A01A01A),
        F(unchecked((long)0xBE927E4FB7789F5C)),
        F(0x3E21EED8EFF8D898),
        F(unchecked((long)0xBDA93974A8C07C9D)),
        F(0x3D2AE7F3E733B81F),
        F(unchecked((long)0xBCA6827863B97D97)),
    ];

    /// <summary>pi/180, the double nearest the real value (also PI / 180.0).</summary>
    internal static readonly double RadPerDeg = F(0x3F91DF46A2529D39);

    /// <summary>180/pi, the double nearest the real value (also 180.0 / PI).</summary>
    internal static readonly double DegPerRad = F(0x404CA5DC1A63C1F8);

    // ------------------------------------------------------------------
    // Arithmetic model (PHY00a 3.1 and 5).  Every operation below is one
    // IEEE binary64 operation with one rounding.  RyuJIT never contracts
    // a * b + c into a fused multiply-add on its own; it does so only when
    // asked through Math.FusedMultiplyAdd, which this file never calls.
    // Math.Sin and Math.Cos are likewise never used.  Each product that
    // feeds an add or subtract is written as its own statement so the
    // reader can see every rounding.
    // ------------------------------------------------------------------

    /// <summary>
    /// Range reduction (PHY00a 3.4).  Returns k, the nearest integer to
    /// x / (pi/2) as a double, and r = x - k*pi/2 with |r| about pi/4.
    ///
    ///   x ---------------------------------------------------- the angle
    ///   |-- k*PIO2_1 --|                                        24 bits of k*pi/2
    ///                  |-- k*PIO2_2 --|                         next 24
    ///                                 |-- k*PIO2_3 --|          next 24
    ///                                                |- k*PIO2_4 -|
    ///                                                             |k*PIO2_5| rounds
    ///
    /// Step 1 rounds the quotient with the MAGIC trick: adding 1.5 * 2^52
    /// pushes the fraction bits off the end of the significand, so the
    /// hardware's own round-to-nearest-even makes it a whole number; the
    /// subtraction hands it back exactly.  No floor, round or cast involved.
    /// </summary>
    private static void Reduce(double x, out double k, out double r)
    {
        var t = x * InvPio2;
        k = (t + Magic) - Magic;

        r = x;
        foreach (var piece in Pio2)
        {
            var m = k * piece;   // exact for the first four pieces
            r = r - m;
        }
    }

    /// <summary>
    /// Sine on |r| &lt;= pi/4 by Horner's rule on stored coefficients:
    /// r + r*z*(S1 + S2*z + ... + S9*z^8), z = r*r (ten terms, x^1 .. x^19).
    /// </summary>
    private static double SinKernel(double r)
    {
        var z = r * r;
        var p = S[9];
        for (var n = 8; n >= 1; n--)
        {
            p = p * z;
            p = p + S[n];
        }

        var w = r * z;
        w = w * p;
        return r + w;
    }

    /// <summary>
    /// Cosine on |r| &lt;= pi/4: (1 - z/2) + z^2*(C2 + C3*z + ... + C9*z^7)
    /// (ten terms, x^0 .. x^18).  0.5 * z is exact.
    /// </summary>
    private static double CosKernel(double r)
    {
        var z = r * r;
        var q = C[9];
        for (var n = 8; n >= 2; n--)
        {
            q = q * z;
            q = q + C[n];
        }

        var u = 0.5 * z;
        u = 1.0 - u;
        var v = z * z;
        v = v * q;
        return u + v;
    }

    /// <summary>
    /// Compute sine (PHY00a 3.6).  Defined for |x| &lt;= 2^29; NaN, the
    /// infinities and larger arguments return NaN.  sin(-0) = -0.
    ///
    /// The quadrant table, with q = k mod 4:
    ///
    ///   q = 0:  sin r     q = 1:  cos r     q = 2:  -sin r     q = 3:  -cos r
    /// </summary>
    public static double Sin(double x)
    {
        // Negated <=, so that NaN (every comparison false) is caught too.
        if (!(Math.Abs(x) <= Limit))
        {
            return double.NaN;
        }

        if (Math.Abs(x) < Tiny)
        {
            return x;   // keeps -0 and subnormals exact
        }

        Reduce(x, out var k, out var r);
        switch ((long)k & 3L)   // two's complement: -1 & 3 = 3
        {
            case 0: return SinKernel(r);
            case 1: return CosKernel(r);
            case 2: return -SinKernel(r);
            default: return -CosKernel(r);
        }
    }

    /// <summary>
    /// Compute cosine (PHY00a 3.6), same domain as <see cref="Sin"/>.
    ///
    ///   q = 0:  cos r     q = 1:  -sin r    q = 2:  -cos r     q = 3:  sin r
    /// </summary>
    public static double Cos(double x)
    {
        if (!(Math.Abs(x) <= Limit))
        {
            return double.NaN;
        }

        if (Math.Abs(x) < Tiny)
        {
            return 1.0;
        }

        Reduce(x, out var k, out var r);
        switch ((long)k & 3L)
        {
            case 0: return CosKernel(r);
            case 1: return -SinKernel(r);
            case 2: return -CosKernel(r);
            default: return SinKernel(r);
        }
    }

    /// <summary>
    /// Convert degrees to radians: one multiply by the pinned constant
    /// (PHY00a 3.7), so one rounding.
    /// </summary>
    public static double Radians(double degrees) => degrees * RadPerDeg;

    /// <summary>
    /// Convert radians to degrees: one multiply by the pinned constant.
    /// </summary>
    public static double Degrees(double radians) => radians * DegPerRad;

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
            return x;
        }

        if (double.IsPositiveInfinity(x))
        {
            return x;
        }

        var scaled = x;
        var resultScale = 1.0;
        while (scaled < 0.25)
        {
            scaled *= 4.0;
            resultScale *= 0.5;
        }
        while (scaled >= 4.0)
        {
            scaled *= 0.25;
            resultScale *= 2.0;
        }

        var guess = scaled >= 1.0 ? scaled : 1.0;
        for (var i = 0; i < 60; i++)
        {
            var next = (guess + scaled / guess) / 2.0;
            if (Math.Abs(next - guess) < 1e-15 * guess + 1e-300)
            {
                return next * resultScale;
            }

            guess = next;
        }

        return guess * resultScale;
    }

    /// <summary>
    /// Compute tangent as the ratio of sine to cosine (PHY00a 3.8).  Within
    /// 1e-15 of a pole the legacy sentinel +-1e308 is returned (it is not an
    /// approximation of the tangent); there is no accuracy claim for tan.
    /// NaN beyond |x| = 2^29, as for sine and cosine.
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
        // atan(x) rounds exactly to x here; avoid halving subnormals and retain -0.
        if (Math.Abs(x) <= 7.450580596923828e-9)
        {
            return x;
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
        // Added by PHY00a 3.9: an undefined question gets NaN, not 0.0.
        if (double.IsNaN(x) || double.IsNaN(y))
        {
            return double.NaN;
        }

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
