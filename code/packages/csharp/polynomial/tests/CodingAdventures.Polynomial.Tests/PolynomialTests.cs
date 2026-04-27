namespace CodingAdventures.Polynomial.Tests;

// PolynomialTests.cs -- Comprehensive tests for the Polynomial library
// =====================================================================
//
// Each section matches a function in Polynomial.cs. Tests are arranged from
// simplest (constants, degree) through arithmetic to GCD, verifying both
// normal use and edge cases.

public sealed class PolynomialTests
{
    // =========================================================================
    // Helpers: small polynomials we reuse throughout
    // =========================================================================

    // p0 = [] (the zero polynomial)
    private static readonly double[] P0 = [];
    // p1 = [1]  (constant polynomial 1)
    private static readonly double[] P1 = [1.0];
    // p12 = [1, 2]  = 1 + 2x
    private static readonly double[] P12 = [1.0, 2.0];
    // p34 = [3, 4]  = 3 + 4x
    private static readonly double[] P34 = [3.0, 4.0];
    // p123 = [1, 2, 3] = 1 + 2x + 3x²
    private static readonly double[] P123 = [1.0, 2.0, 3.0];
    // p573 = [5, 7, 3] = 5 + 7x + 3x²  (= P12 + P34 shifted one term? No — used as test literal)
    private static readonly double[] P573 = [5.0, 7.0, 3.0];

    // =========================================================================
    // VERSION
    // =========================================================================

    [Fact]
    public void ExposesExpectedVersion()
    {
        Assert.Equal("0.1.0", Polynomial.VERSION);
    }

    // =========================================================================
    // Normalize
    // =========================================================================

    [Fact]
    public void NormalizeStripsTrailingZeros()
    {
        // [1, 0, 0] represents the same constant polynomial as [1].
        Assert.Equal(new double[] { 1.0 }, Polynomial.Normalize([1.0, 0.0, 0.0]));
    }

    [Fact]
    public void NormalizeHandlesAllZeros()
    {
        // [0], [0, 0], [0, 0, 0] are all the zero polynomial.
        Assert.Equal(Array.Empty<double>(), Polynomial.Normalize([0.0]));
        Assert.Equal(Array.Empty<double>(), Polynomial.Normalize([0.0, 0.0]));
    }

    [Fact]
    public void NormalizeHandlesEmptyArray()
    {
        Assert.Equal(Array.Empty<double>(), Polynomial.Normalize([]));
    }

    [Fact]
    public void NormalizeAlreadyNormalizedIsIdentity()
    {
        // A polynomial with no trailing zeros should be returned as-is.
        var result = Polynomial.Normalize([1.0, 2.0, 3.0]);
        Assert.Equal(new double[] { 1.0, 2.0, 3.0 }, result);
    }

    [Fact]
    public void NormalizeMiddleZerosAreKept()
    {
        // Interior zeros (non-trailing) must be kept — they represent missing terms.
        // [1, 0, 3] = 1 + 0x + 3x² is already normalized.
        Assert.Equal(new double[] { 1.0, 0.0, 3.0 }, Polynomial.Normalize([1.0, 0.0, 3.0]));
    }

    // =========================================================================
    // Degree
    // =========================================================================

    [Fact]
    public void DegreeOfZeroPolynomialIsMinusOne()
    {
        // The zero polynomial has degree -1 by convention (allows divmod to terminate).
        Assert.Equal(-1, Polynomial.Degree([]));
    }

    [Fact]
    public void DegreeOfAllZeroCoeffsIsMinusOne()
    {
        Assert.Equal(-1, Polynomial.Degree([0.0, 0.0]));
    }

    [Fact]
    public void DegreeOfConstantIsZero()
    {
        // Any non-zero constant has degree 0.
        Assert.Equal(0, Polynomial.Degree([7.0]));
    }

    [Fact]
    public void DegreeOfLinearIsOne()
    {
        Assert.Equal(1, Polynomial.Degree([3.0, 5.0]));
    }

    [Fact]
    public void DegreeOfQuadraticIsTwo()
    {
        Assert.Equal(2, Polynomial.Degree([1.0, 2.0, 3.0]));
    }

    [Fact]
    public void DegreeIgnoresTrailingZeros()
    {
        // [3, 0, 0] normalizes to [3] which has degree 0.
        Assert.Equal(0, Polynomial.Degree([3.0, 0.0, 0.0]));
    }

    // =========================================================================
    // Zero and One
    // =========================================================================

    [Fact]
    public void ZeroReturnsEmptyArray()
    {
        Assert.Equal(Array.Empty<double>(), Polynomial.Zero());
    }

    [Fact]
    public void OneReturnsArrayWithSingleOne()
    {
        Assert.Equal(new double[] { 1.0 }, Polynomial.One());
    }

    [Fact]
    public void ZeroIsAdditiveIdentity()
    {
        // p + 0 = p for any polynomial p.
        var sum = Polynomial.Add(P123, Polynomial.Zero());
        Assert.Equal(P123, sum);
    }

    [Fact]
    public void OneIsMultiplicativeIdentity()
    {
        // p * 1 = p for any polynomial p.
        var product = Polynomial.Multiply(P123, Polynomial.One());
        Assert.Equal(P123, product);
    }

    // =========================================================================
    // Add
    // =========================================================================

    [Fact]
    public void AddTwoConstantsGivesTheirSum()
    {
        // [3] + [4] = [7]
        Assert.Equal(new double[] { 7.0 }, Polynomial.Add([3.0], [4.0]));
    }

    [Fact]
    public void AddPolynomialsOfSameLength()
    {
        // [1, 2] + [3, 4] = [4, 6]  (1+3, 2+4)
        Assert.Equal(new double[] { 4.0, 6.0 }, Polynomial.Add(P12, P34));
    }

    [Fact]
    public void AddPolynomialsOfDifferentLengths()
    {
        // [1, 2, 3] + [4, 5] = [5, 7, 3]
        Assert.Equal(new double[] { 5.0, 7.0, 3.0 }, Polynomial.Add(P123, [4.0, 5.0]));
    }

    [Fact]
    public void AddWithZeroPolynomialIsIdentity()
    {
        Assert.Equal(P123, Polynomial.Add(P123, P0));
        Assert.Equal(P123, Polynomial.Add(P0, P123));
    }

    [Fact]
    public void AddCancellationNormalizes()
    {
        // [5, 7, 3] + [−5, −7, −3] = [0, 0, 0] → normalize → []
        Assert.Equal(Array.Empty<double>(), Polynomial.Add(P573, [-5.0, -7.0, -3.0]));
    }

    [Fact]
    public void AddIsCommutative()
    {
        Assert.Equal(Polynomial.Add(P12, P123), Polynomial.Add(P123, P12));
    }

    // =========================================================================
    // Subtract
    // =========================================================================

    [Fact]
    public void SubtractSamePolynomialGivesZero()
    {
        // p - p = 0 for any p.
        Assert.Equal(Array.Empty<double>(), Polynomial.Subtract(P123, P123));
    }

    [Fact]
    public void SubtractFromZeroNegates()
    {
        // 0 - [1, 2] = [-1, -2]
        Assert.Equal(new double[] { -1.0, -2.0 }, Polynomial.Subtract(P0, P12));
    }

    [Fact]
    public void SubtractStripsTrailingZero()
    {
        // [5, 7, 3] - [1, 2, 3] = [4, 5, 0] → normalize → [4, 5]
        Assert.Equal(new double[] { 4.0, 5.0 }, Polynomial.Subtract(P573, P123));
    }

    [Fact]
    public void SubtractPolynomialsOfDifferentLengths()
    {
        // [1, 2, 3] - [1, 2] = [0, 0, 3] → normalize → [0, 0, 3] (interior zeros kept)
        Assert.Equal(new double[] { 0.0, 0.0, 3.0 }, Polynomial.Subtract(P123, P12));
    }

    // =========================================================================
    // Multiply
    // =========================================================================

    [Fact]
    public void MultiplyByZeroPolynomialGivesZero()
    {
        Assert.Equal(Array.Empty<double>(), Polynomial.Multiply(P123, P0));
        Assert.Equal(Array.Empty<double>(), Polynomial.Multiply(P0, P123));
    }

    [Fact]
    public void MultiplyByOneIsIdentity()
    {
        Assert.Equal(P123, Polynomial.Multiply(P123, P1));
        Assert.Equal(P123, Polynomial.Multiply(P1, P123));
    }

    [Fact]
    public void MultiplyTwoLinearPolynomials()
    {
        // (1 + 2x)(3 + 4x) = 3 + 4x + 6x + 8x² = 3 + 10x + 8x²
        Assert.Equal(new double[] { 3.0, 10.0, 8.0 }, Polynomial.Multiply(P12, P34));
    }

    [Fact]
    public void MultiplyIsCommutative()
    {
        Assert.Equal(Polynomial.Multiply(P12, P123), Polynomial.Multiply(P123, P12));
    }

    [Fact]
    public void MultiplyDegreeIsSum()
    {
        // degree(a * b) = degree(a) + degree(b)
        var product = Polynomial.Multiply(P123, P34);
        Assert.Equal(Polynomial.Degree(P123) + Polynomial.Degree(P34), Polynomial.Degree(product));
    }

    [Fact]
    public void MultiplyConstantScalesCoefficients()
    {
        // [3] * [1, 2, 3] = [3, 6, 9]
        Assert.Equal(new double[] { 3.0, 6.0, 9.0 }, Polynomial.Multiply([3.0], P123));
    }

    [Fact]
    public void MultiplyTwoConstants()
    {
        // [5] * [4] = [20]
        Assert.Equal(new double[] { 20.0 }, Polynomial.Multiply([5.0], [4.0]));
    }

    // =========================================================================
    // DivMod
    // =========================================================================

    [Fact]
    public void DivModByZeroThrows()
    {
        Assert.Throws<InvalidOperationException>(() => Polynomial.DivMod(P12, P0));
    }

    [Fact]
    public void DivModWhenAHasLowerDegreeThanB()
    {
        // degree(a) < degree(b) → quotient = [], remainder = a.
        var (q, r) = Polynomial.DivMod([4.0, 5.0], [1.0, 2.0, 3.0]);
        Assert.Equal(Array.Empty<double>(), q);
        Assert.Equal(new double[] { 4.0, 5.0 }, r);
    }

    [Fact]
    public void DivModExactDivisionHasZeroRemainder()
    {
        // (3 + 10x + 8x²) / (1 + 2x) should divide exactly: (1+2x)(3+4x) = 3+10x+8x²
        var (q, r) = Polynomial.DivMod([3.0, 10.0, 8.0], P12);
        Assert.Equal(P34, q);
        Assert.Equal(Array.Empty<double>(), r);
    }

    [Fact]
    public void DivModSpecExampleFromSpec()
    {
        // From the spec: divide [5, 1, 3, 2] = 5 + x + 3x² + 2x³  by  [2, 1] = 2 + x
        // Expected: quotient = [3, -1, 2], remainder = [-1]
        var (q, r) = Polynomial.DivMod([5.0, 1.0, 3.0, 2.0], [2.0, 1.0]);
        Assert.Equal(new double[] { 3.0, -1.0, 2.0 }, q);
        Assert.Equal(new double[] { -1.0 }, r);
    }

    [Fact]
    public void DivModSatisfiesDividendEquality()
    {
        // Verify a = b*q + r for a general case.
        double[] a = [5.0, 1.0, 3.0, 2.0];
        double[] b = [2.0, 1.0];
        var (q, r) = Polynomial.DivMod(a, b);

        // Reconstruct b*q + r and compare to a.
        var reconstructed = Polynomial.Add(Polynomial.Multiply(b, q), r);
        Assert.Equal(Polynomial.Normalize(a), reconstructed);
    }

    [Fact]
    public void DivModDivideByItself()
    {
        // Any poly divided by itself has quotient [1] and remainder [].
        var (q, r) = Polynomial.DivMod(P123, P123);
        Assert.Equal(new double[] { 1.0 }, q);
        Assert.Equal(Array.Empty<double>(), r);
    }

    [Fact]
    public void DivModDivideByConstant()
    {
        // [6, 4, 2] / [2] = [3, 2, 1] with no remainder.
        var (q, r) = Polynomial.DivMod([6.0, 4.0, 2.0], [2.0]);
        Assert.Equal(new double[] { 3.0, 2.0, 1.0 }, q);
        Assert.Equal(Array.Empty<double>(), r);
    }

    [Fact]
    public void DivideReturnsQuotient()
    {
        var q = Polynomial.Divide([3.0, 10.0, 8.0], P12);
        Assert.Equal(P34, q);
    }

    [Fact]
    public void ModReturnsRemainder()
    {
        var r = Polynomial.Mod([5.0, 1.0, 3.0, 2.0], [2.0, 1.0]);
        Assert.Equal(new double[] { -1.0 }, r);
    }

    // =========================================================================
    // Evaluate
    // =========================================================================

    [Fact]
    public void EvaluateZeroPolynomialIsAlwaysZero()
    {
        Assert.Equal(0.0, Polynomial.Evaluate(P0, 0.0));
        Assert.Equal(0.0, Polynomial.Evaluate(P0, 42.0));
        Assert.Equal(0.0, Polynomial.Evaluate(P0, -99.0));
    }

    [Fact]
    public void EvaluateConstantPolynomialAtAnyPoint()
    {
        // [7] evaluates to 7 regardless of x.
        Assert.Equal(7.0, Polynomial.Evaluate([7.0], 0.0));
        Assert.Equal(7.0, Polynomial.Evaluate([7.0], 100.0));
    }

    [Fact]
    public void EvaluateLinearAtZeroIsConstantTerm()
    {
        // p(0) = a₀ for any polynomial (Horner step with x=0 kills all other terms).
        Assert.Equal(3.0, Polynomial.Evaluate([3.0, 5.0], 0.0));
    }

    [Fact]
    public void EvaluateSpecExampleFromSpec()
    {
        // From spec: [3, 1, 2] = 3 + x + 2x² at x = 4 should give 39.
        Assert.Equal(39.0, Polynomial.Evaluate([3.0, 1.0, 2.0], 4.0));
    }

    [Fact]
    public void EvaluateAtOneIsCoefficientsSum()
    {
        // p(1) = sum of all coefficients (every x^i = 1).
        var p = new double[] { 1.0, 2.0, 3.0 };
        Assert.Equal(6.0, Polynomial.Evaluate(p, 1.0));
    }

    [Fact]
    public void EvaluateAtMinusOne()
    {
        // [1, 2, 3] at x = -1: 1 - 2 + 3 = 2.
        Assert.Equal(2.0, Polynomial.Evaluate([1.0, 2.0, 3.0], -1.0));
    }

    [Fact]
    public void EvaluateMatchesNaiveFormula()
    {
        // Cross-check Horner vs direct calculation: [2, 0, 1] = 2 + x² at x = 3.
        // Naive: 2 + 0*3 + 1*9 = 11.
        Assert.Equal(11.0, Polynomial.Evaluate([2.0, 0.0, 1.0], 3.0));
    }

    // =========================================================================
    // Gcd
    // =========================================================================

    [Fact]
    public void GcdWithZeroPolynomialReturnsOther()
    {
        // gcd(p, []) = p for any p.
        Assert.Equal(P123, Polynomial.Gcd(P123, P0));
        // gcd([], p) = p for any p (after the loop, u = P0 then v = first(p)).
        // Actually gcd([], p): u=[], v=p. v.Length>0: r=mod([],p)=[]; u=p; v=[]. Return p.
        Assert.Equal(P123, Polynomial.Gcd(P0, P123));
    }

    [Fact]
    public void GcdOfPolynomialWithItselfIsItself()
    {
        // gcd(p, p) = p (normalized).
        Assert.Equal(Polynomial.Normalize(P123), Polynomial.Gcd(P123, P123));
    }

    [Fact]
    public void GcdConstantResultWhenNoCommonFactor()
    {
        // (x+1)(x+6) and (x+2)(x+3) share no common non-constant factor.
        // The GCD is some constant (a non-zero scalar), so it must have degree 0.
        // We don't assert an exact value because real-number Euclidean GCD is not
        // normalized to monic form; any non-zero constant divisor is correct.
        var g = Polynomial.Gcd([6.0, 7.0, 1.0], [6.0, 5.0, 1.0]);
        Assert.Equal(0, Polynomial.Degree(g));                           // degree 0 = constant
        Assert.Equal(Array.Empty<double>(), Polynomial.Mod([6.0, 7.0, 1.0], g)); // divides a
        Assert.Equal(Array.Empty<double>(), Polynomial.Mod([6.0, 5.0, 1.0], g)); // divides b
    }

    [Fact]
    public void GcdExtractsCommonLinearFactor()
    {
        // [2, 3] = 2 + 3x and [4, 6] = 2*(2 + 3x). GCD should be a scalar multiple of [2, 3].
        var g = Polynomial.Gcd([2.0, 3.0], [4.0, 6.0]);
        // The GCD must divide both inputs with zero remainder.
        Assert.Equal(Array.Empty<double>(), Polynomial.Mod([2.0, 3.0], g));
        Assert.Equal(Array.Empty<double>(), Polynomial.Mod([4.0, 6.0], g));
        // The degree must be 1 (linear common factor).
        Assert.Equal(1, Polynomial.Degree(g));
    }

    [Fact]
    public void GcdIsCommutative()
    {
        // gcd(a, b) and gcd(b, a) must be scalar multiples of each other (same degree,
        // same divisibility). We verify they share the same degree and each divides both inputs.
        var g1 = Polynomial.Gcd(P12, P34);
        var g2 = Polynomial.Gcd(P34, P12);
        Assert.Equal(Polynomial.Degree(g1), Polynomial.Degree(g2));
        // Both results divide both inputs.
        Assert.Equal(Array.Empty<double>(), Polynomial.Mod(P12, g1));
        Assert.Equal(Array.Empty<double>(), Polynomial.Mod(P12, g2));
    }

    // =========================================================================
    // Format (display helper)
    // =========================================================================

    [Fact]
    public void FormatZeroPolynomial()
    {
        Assert.Equal("0", Polynomial.Format(P0));
    }

    [Fact]
    public void FormatConstantPolynomial()
    {
        Assert.Equal("3", Polynomial.Format([3.0]));
    }

    [Fact]
    public void FormatLinearPolynomial()
    {
        Assert.Equal("1 + 2x", Polynomial.Format([1.0, 2.0]));
    }

    [Fact]
    public void FormatQuadraticPolynomial()
    {
        Assert.Equal("1 + 2x + 3x^2", Polynomial.Format([1.0, 2.0, 3.0]));
    }

    // =========================================================================
    // Null-guard checks
    // =========================================================================

    [Fact]
    public void NullInputsThrowArgumentNullException()
    {
        Assert.Throws<ArgumentNullException>(() => Polynomial.Normalize(null!));
        Assert.Throws<ArgumentNullException>(() => Polynomial.Degree(null!));
        Assert.Throws<ArgumentNullException>(() => Polynomial.Add(null!, P0));
        Assert.Throws<ArgumentNullException>(() => Polynomial.Add(P0, null!));
        Assert.Throws<ArgumentNullException>(() => Polynomial.Subtract(null!, P0));
        Assert.Throws<ArgumentNullException>(() => Polynomial.Subtract(P0, null!));
        Assert.Throws<ArgumentNullException>(() => Polynomial.Multiply(null!, P0));
        Assert.Throws<ArgumentNullException>(() => Polynomial.Multiply(P0, null!));
        Assert.Throws<ArgumentNullException>(() => Polynomial.DivMod(null!, P1));
        Assert.Throws<ArgumentNullException>(() => Polynomial.DivMod(P1, null!));
        Assert.Throws<ArgumentNullException>(() => Polynomial.Evaluate(null!, 0.0));
        Assert.Throws<ArgumentNullException>(() => Polynomial.Gcd(null!, P0));
        Assert.Throws<ArgumentNullException>(() => Polynomial.Gcd(P0, null!));
        Assert.Throws<ArgumentNullException>(() => Polynomial.Format(null!));
    }
}
