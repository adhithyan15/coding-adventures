using System;
using System.Collections.Generic;
using System.Text;

namespace CodingAdventures.Polynomial;

// Polynomial.cs -- Coefficient-array polynomial arithmetic
// =========================================================
//
// A polynomial is a mathematical expression in one variable x built from a
// list of constant coefficients. For example, the polynomial 3 + 2x + 5x²
// has three terms: a constant 3, a linear term 2x, and a quadratic term 5x².
//
// We store polynomials as plain arrays of numbers where the *array index equals
// the degree* of that term:
//
//   [3, 2, 5]
//    ^  ^  ^
//    |  |  └── coefficient of x²  (index 2 → degree 2)
//    |  └───── coefficient of x¹  (index 1 → degree 1)
//    └──────── coefficient of x⁰  (index 0 → degree 0, the constant)
//
// This "little-endian" layout (lowest degree first) makes addition trivially
// position-aligned and keeps Horner's method natural to read.
//
// All operations normalize their output — trailing zero-coefficients are
// stripped. So [1, 0, 0] and [1] both represent the constant polynomial 1.
//
// GF(256) context
// ---------------
// Reed-Solomon error correction works with polynomials whose coefficients live
// in GF(2^8) (a finite field where addition is XOR and multiplication is
// modular). This library works over regular integers/floats, matching the
// TypeScript and Python reference implementations. The caller supplies the
// scalar multiply and add operations — here we just use regular arithmetic.
// The GF(256)-specific polynomial ring used by Reed-Solomon builds on top of
// this package by plugging in the field operations.

/// <summary>
/// Coefficient-array polynomial arithmetic.
///
/// A polynomial is stored as a <see cref="double"/> array where index i holds
/// the coefficient of x^i (little-endian, lowest degree first). The zero
/// polynomial is the empty array.
/// </summary>
public static class Polynomial
{
    /// <summary>Package version.</summary>
    public const string VERSION = "0.1.0";

    // =========================================================================
    // Fundamentals
    // =========================================================================

    /// <summary>
    /// Remove trailing zeros from a polynomial.
    ///
    /// Trailing zeros represent zero-coefficient high-degree terms. They do not
    /// change the mathematical value, but they affect degree comparisons and the
    /// stopping condition in polynomial long division.
    ///
    /// Examples:
    ///   normalize([1, 0, 0]) → [1]   (constant polynomial 1)
    ///   normalize([0])       → []    (zero polynomial)
    ///   normalize([1, 2, 3]) → [1, 2, 3]  (already normalized)
    /// </summary>
    public static double[] Normalize(double[] p)
    {
        if (p is null) throw new ArgumentNullException(nameof(p));

        var len = p.Length;
        // Walk backwards, skipping trailing zeros.
        while (len > 0 && p[len - 1] == 0.0)
        {
            len--;
        }

        if (len == p.Length) return p;
        var result = new double[len];
        Array.Copy(p, result, len);
        return result;
    }

    /// <summary>
    /// Return the degree of a polynomial.
    ///
    /// The degree is the index of the highest non-zero coefficient.
    ///
    /// By convention, the zero polynomial has degree -1. This sentinel lets
    /// polynomial long division terminate cleanly: the loop condition
    /// <c>degree(remainder) &gt;= degree(divisor)</c> is false when the
    /// remainder is zero.
    ///
    /// Examples:
    ///   degree([3, 0, 2]) → 2   (highest non-zero: index 2, the x² term)
    ///   degree([7])       → 0   (constant polynomial; degree 0)
    ///   degree([])        → -1  (zero polynomial; degree -1 by convention)
    ///   degree([0, 0])    → -1  (normalizes to []; same as zero polynomial)
    /// </summary>
    public static int Degree(double[] p)
    {
        if (p is null) throw new ArgumentNullException(nameof(p));
        var n = Normalize(p);
        // When n is empty (zero polynomial) this returns 0 - 1 = -1.
        return n.Length - 1;
    }

    /// <summary>
    /// Return the zero polynomial (empty array).
    ///
    /// Zero is the additive identity: Add(Zero(), p) = p for any p.
    /// </summary>
    public static double[] Zero() => [];

    /// <summary>
    /// Return the multiplicative identity polynomial [1].
    ///
    /// Multiplying any polynomial by One() returns that polynomial unchanged.
    /// </summary>
    public static double[] One() => [1.0];

    // =========================================================================
    // Addition and Subtraction
    // =========================================================================

    /// <summary>
    /// Add two polynomials term-by-term.
    ///
    /// Addition aligns matching coefficients by index. The shorter polynomial
    /// is implicitly padded with zeros.
    ///
    /// Visual example:
    ///   [1, 2, 3]   =  1 + 2x + 3x²
    /// + [4, 5]      =  4 + 5x
    /// ─────────────────────────────
    ///   [5, 7, 3]   =  5 + 7x + 3x²
    ///
    /// The degree-2 term 3x² had no partner in b, so it passes through intact.
    /// </summary>
    public static double[] Add(double[] a, double[] b)
    {
        if (a is null) throw new ArgumentNullException(nameof(a));
        if (b is null) throw new ArgumentNullException(nameof(b));

        var len = Math.Max(a.Length, b.Length);
        var result = new double[len];

        for (var i = 0; i < len; i++)
        {
            var ai = i < a.Length ? a[i] : 0.0;
            var bi = i < b.Length ? b[i] : 0.0;
            result[i] = ai + bi;
        }

        return Normalize(result);
    }

    /// <summary>
    /// Subtract polynomial b from polynomial a term-by-term.
    ///
    /// Equivalent to Add(a, Negate(b)), but computed directly.
    ///
    /// Visual example:
    ///   [5, 7, 3]   =  5 + 7x + 3x²
    /// - [1, 2, 3]   =  1 + 2x + 3x²
    /// ─────────────────────────────
    ///   [4, 5, 0]   →  normalize  →  [4, 5]   =  4 + 5x
    ///
    /// Note: 3x² − 3x² = 0; normalize strips the trailing zero.
    /// </summary>
    public static double[] Subtract(double[] a, double[] b)
    {
        if (a is null) throw new ArgumentNullException(nameof(a));
        if (b is null) throw new ArgumentNullException(nameof(b));

        var len = Math.Max(a.Length, b.Length);
        var result = new double[len];

        for (var i = 0; i < len; i++)
        {
            var ai = i < a.Length ? a[i] : 0.0;
            var bi = i < b.Length ? b[i] : 0.0;
            result[i] = ai - bi;
        }

        return Normalize(result);
    }

    // =========================================================================
    // Multiplication
    // =========================================================================

    /// <summary>
    /// Multiply two polynomials using polynomial convolution.
    ///
    /// Each term a[i]·xⁱ of a multiplies each term b[j]·xʲ of b, contributing
    /// a[i]·b[j] to the result at index i+j.
    ///
    /// If a has degree m and b has degree n, the result has degree m+n.
    ///
    /// Visual example:
    ///   [1, 2]  =  1 + 2x
    /// × [3, 4]  =  3 + 4x
    /// ─────────────────────────────────────────────
    /// result array of length 3, initialized to [0, 0, 0]:
    ///   i=0, j=0: result[0] += 1·3 = 3   → [3, 0, 0]
    ///   i=0, j=1: result[1] += 1·4 = 4   → [3, 4, 0]
    ///   i=1, j=0: result[1] += 2·3 = 6   → [3, 10, 0]
    ///   i=1, j=1: result[2] += 2·4 = 8   → [3, 10, 8]
    ///
    /// Result: [3, 10, 8]  =  3 + 10x + 8x²
    /// Verify: (1+2x)(3+4x) = 3+4x+6x+8x² = 3+10x+8x²  ✓
    /// </summary>
    public static double[] Multiply(double[] a, double[] b)
    {
        if (a is null) throw new ArgumentNullException(nameof(a));
        if (b is null) throw new ArgumentNullException(nameof(b));

        // Multiplying by the zero polynomial yields zero.
        if (a.Length == 0 || b.Length == 0) return [];

        // Result degree = deg(a) + deg(b), so result length = a.Length + b.Length - 1.
        var resultLen = a.Length + b.Length - 1;
        var result = new double[resultLen];

        for (var i = 0; i < a.Length; i++)
        {
            for (var j = 0; j < b.Length; j++)
            {
                result[i + j] += a[i] * b[j];
            }
        }

        return Normalize(result);
    }

    // =========================================================================
    // Division
    // =========================================================================

    /// <summary>
    /// Perform polynomial long division, returning (quotient, remainder).
    ///
    /// Given polynomials a and b (b ≠ zero), finds q and r such that:
    ///   a = b × q + r   and   degree(r) &lt; degree(b)
    ///
    /// Algorithm — analogous to school long division:
    ///   1. Find the leading term of the current remainder.
    ///   2. Divide it by the leading term of b → next quotient coefficient.
    ///   3. Subtract (quotient term) × b from the remainder.
    ///   4. Repeat until degree(remainder) &lt; degree(b).
    ///
    /// Detailed walkthrough: divide [5, 1, 3, 2] = 5 + x + 3x² + 2x³  by  [2, 1] = 2 + x
    ///
    ///   Step 1: remainder = [5, 1, 3, 2], deg=3.
    ///           Leading = 2x³, divisor leading = 1x.
    ///           Quotient term: 2x³/x = 2x²  → q[2] = 2
    ///           Subtract 2x² × (2+x) = 4x²+2x³ = [0,0,4,2] from remainder:
    ///           [5,1,3-4,2-2] = [5,1,-1,0] → normalize → [5,1,-1]
    ///
    ///   Step 2: remainder = [5,1,-1], deg=2.
    ///           Leading = -x², divisor leading = x.
    ///           Quotient term: -x²/x = -x  → q[1] = -1
    ///           Subtract -x × (2+x) = -2x-x² = [0,-2,-1] from [5,1,-1]:
    ///           [5,3,0] → [5,3]
    ///
    ///   Step 3: remainder = [5,3], deg=1.
    ///           Leading = 3x, divisor leading = x.
    ///           Quotient term: 3x/x = 3  → q[0] = 3
    ///           Subtract 3 × (2+x) = 6+3x = [6,3] from [5,3]:
    ///           [-1,0] → [-1]
    ///
    ///   Step 4: degree([-1]) = 0 &lt; 1 = degree(b). STOP.
    ///   Result: q = [3, -1, 2],  r = [-1]
    ///   Verify: (x+2)(3-x+2x²) + (-1) = 3x-x²+2x³+6-2x+4x² - 1 = 5+x+3x²+2x³  ✓
    /// </summary>
    /// <exception cref="InvalidOperationException">Thrown when b is the zero polynomial.</exception>
    public static (double[] Quotient, double[] Remainder) DivMod(double[] a, double[] b)
    {
        if (a is null) throw new ArgumentNullException(nameof(a));
        if (b is null) throw new ArgumentNullException(nameof(b));

        var nb = Normalize(b);
        if (nb.Length == 0)
        {
            throw new InvalidOperationException("polynomial division by zero");
        }

        var na = Normalize(a);
        var degA = na.Length - 1;
        var degB = nb.Length - 1;

        // If a has lower degree than b, quotient is zero, remainder is a.
        if (degA < degB)
        {
            return ([], na);
        }

        // Work on a mutable copy of the remainder.
        var rem = (double[])na.Clone();
        // Allocate the quotient with the right length.
        var quot = new double[degA - degB + 1];

        // Leading coefficient of the divisor.
        var leadB = nb[degB];

        // Current degree of the remainder — walks downward as we subtract.
        var degRem = degA;

        while (degRem >= degB)
        {
            // Quotient coefficient for this step.
            var coeff = rem[degRem] / leadB;
            var power = degRem - degB;
            quot[power] = coeff;

            // Subtract coeff * x^power * b from rem.
            for (var j = 0; j <= degB; j++)
            {
                rem[power + j] -= coeff * nb[j];
            }

            // The leading term is now zero by construction. Scan down past any
            // new trailing zeros to find the true new degree.
            degRem--;
            while (degRem >= 0 && rem[degRem] == 0.0)
            {
                degRem--;
            }
        }

        return (Normalize(quot), Normalize(rem));
    }

    /// <summary>
    /// Return the quotient of dividing a by b.
    /// </summary>
    /// <exception cref="InvalidOperationException">Thrown when b is the zero polynomial.</exception>
    public static double[] Divide(double[] a, double[] b) => DivMod(a, b).Quotient;

    /// <summary>
    /// Return the remainder of dividing a by b.
    ///
    /// This is the polynomial "modulo" operation. In GF(2^8) field construction,
    /// a high-degree polynomial is reduced modulo the primitive polynomial this way.
    /// </summary>
    /// <exception cref="InvalidOperationException">Thrown when b is the zero polynomial.</exception>
    public static double[] Mod(double[] a, double[] b) => DivMod(a, b).Remainder;

    // =========================================================================
    // Evaluation
    // =========================================================================

    /// <summary>
    /// Evaluate a polynomial at x using Horner's method.
    ///
    /// Horner's method rewrites the polynomial in nested form:
    ///   a₀ + x(a₁ + x(a₂ + ... + x·aₙ))
    ///
    /// This requires only n additions and n multiplications — no powers of x at
    /// all, compared to the naïve approach that would require n exponentiations.
    ///
    /// Algorithm (reading coefficients from high degree down to the constant):
    ///   acc = 0
    ///   for i from n downto 0:
    ///       acc = acc * x + p[i]
    ///   return acc
    ///
    /// Example: evaluate [3, 1, 2] = 3 + x + 2x² at x = 4:
    ///   Start: acc = 0
    ///   i=2: acc = 0*4 + 2 = 2
    ///   i=1: acc = 2*4 + 1 = 9
    ///   i=0: acc = 9*4 + 3 = 39
    ///   Verify: 3 + 4 + 2·16 = 3 + 4 + 32 = 39  ✓
    /// </summary>
    public static double Evaluate(double[] p, double x)
    {
        if (p is null) throw new ArgumentNullException(nameof(p));

        var n = Normalize(p);
        if (n.Length == 0) return 0.0;

        var acc = 0.0;
        // Iterate from high-degree term down to the constant.
        for (var i = n.Length - 1; i >= 0; i--)
        {
            acc = acc * x + n[i];
        }

        return acc;
    }

    // =========================================================================
    // Greatest Common Divisor
    // =========================================================================

    /// <summary>
    /// Compute the GCD of two polynomials using the Euclidean algorithm.
    ///
    /// The Euclidean algorithm for polynomials is identical to the integer
    /// version, with polynomial mod in place of integer mod:
    ///
    ///   while b ≠ zero:
    ///       a, b = b, a mod b
    ///   return normalize(a)
    ///
    /// The result is the highest-degree polynomial that divides both inputs with
    /// zero remainder.
    ///
    /// Use case: Reed-Solomon decoding uses the extended Euclidean algorithm on
    /// polynomials to recover error-locator and error-evaluator polynomials.
    ///
    /// Example: gcd([6, 7, 1], [6, 5, 1])
    ///   Round 1: [6,7,1] mod [6,5,1] → [2]  (constant 2)
    ///   Round 2: [6,5,1] mod [2]     → []   (any poly is divisible by a constant)
    ///   Round 3: b = [] → stop. Return normalize([2]) = [2].
    ///   The GCD is 2, confirming the two quadratics share no common factor.
    /// </summary>
    public static double[] Gcd(double[] a, double[] b)
    {
        if (a is null) throw new ArgumentNullException(nameof(a));
        if (b is null) throw new ArgumentNullException(nameof(b));

        var u = Normalize(a);
        var v = Normalize(b);

        while (v.Length > 0)
        {
            var r = Mod(u, v);
            u = v;
            v = r;
        }

        return Normalize(u);
    }

    // =========================================================================
    // Display helpers (useful for debugging and tests)
    // =========================================================================

    /// <summary>
    /// Format a polynomial as a human-readable string, e.g. "3 + 2x + 5x^2".
    /// </summary>
    public static string Format(double[] p)
    {
        if (p is null) throw new ArgumentNullException(nameof(p));

        var n = Normalize(p);
        if (n.Length == 0) return "0";

        var sb = new StringBuilder();
        var first = true;

        for (var i = 0; i < n.Length; i++)
        {
            if (n[i] == 0.0) continue;

            if (!first) sb.Append(" + ");
            first = false;

            if (i == 0)
            {
                sb.Append(n[i]);
            }
            else if (i == 1)
            {
                sb.Append($"{n[i]}x");
            }
            else
            {
                sb.Append($"{n[i]}x^{i}");
            }
        }

        return sb.Length == 0 ? "0" : sb.ToString();
    }
}
