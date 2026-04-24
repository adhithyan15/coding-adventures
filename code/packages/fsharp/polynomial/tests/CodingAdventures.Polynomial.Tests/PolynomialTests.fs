namespace CodingAdventures.Polynomial.Tests

// PolynomialTests.fs -- Comprehensive tests for the F# Polynomial module
// =======================================================================
//
// Each section mirrors a function in Polynomial.fs. Tests are organized from
// simplest (constants, degree) through arithmetic to GCD, verifying both
// normal use and edge cases.

open System
open Xunit
open CodingAdventures.Polynomial

// =========================================================================
// Helpers: small polynomials we reuse throughout
// =========================================================================

// p0 = [||]       the zero polynomial
// p1 = [|1|]      constant polynomial 1
// p12 = [|1;2|]   1 + 2x
// p34 = [|3;4|]   3 + 4x
// p123 = [|1;2;3|] 1 + 2x + 3x²
// p573 = [|5;7;3|] 5 + 7x + 3x²

type PolynomialTests() =

    // =========================================================================
    // VERSION
    // =========================================================================

    [<Fact>]
    member _.``exposes expected version``() =
        Assert.Equal("0.1.0", Polynomial.VERSION)

    // =========================================================================
    // Normalize
    // =========================================================================

    [<Fact>]
    member _.``normalize strips trailing zeros``() =
        // [|1; 0; 0|] represents the same constant polynomial as [|1|].
        Assert.Equal<float array>([| 1.0 |], Polynomial.normalize [| 1.0; 0.0; 0.0 |])

    [<Fact>]
    member _.``normalize handles all zeros``() =
        Assert.Equal<float array>([||], Polynomial.normalize [| 0.0 |])
        Assert.Equal<float array>([||], Polynomial.normalize [| 0.0; 0.0 |])

    [<Fact>]
    member _.``normalize handles empty array``() =
        Assert.Equal<float array>([||], Polynomial.normalize [||])

    [<Fact>]
    member _.``normalize already normalized is identity``() =
        let result = Polynomial.normalize [| 1.0; 2.0; 3.0 |]
        Assert.Equal<float array>([| 1.0; 2.0; 3.0 |], result)

    [<Fact>]
    member _.``normalize keeps interior zeros``() =
        // Interior zeros (non-trailing) represent missing terms — they must be kept.
        // [|1; 0; 3|] = 1 + 0x + 3x² is already normalized.
        Assert.Equal<float array>([| 1.0; 0.0; 3.0 |], Polynomial.normalize [| 1.0; 0.0; 3.0 |])

    // =========================================================================
    // Degree
    // =========================================================================

    [<Fact>]
    member _.``degree of zero polynomial is minus one``() =
        // The zero polynomial has degree -1 by convention.
        Assert.Equal(-1, Polynomial.degree [||])

    [<Fact>]
    member _.``degree of all-zero coefficients is minus one``() =
        Assert.Equal(-1, Polynomial.degree [| 0.0; 0.0 |])

    [<Fact>]
    member _.``degree of constant is zero``() =
        Assert.Equal(0, Polynomial.degree [| 7.0 |])

    [<Fact>]
    member _.``degree of linear is one``() =
        Assert.Equal(1, Polynomial.degree [| 3.0; 5.0 |])

    [<Fact>]
    member _.``degree of quadratic is two``() =
        Assert.Equal(2, Polynomial.degree [| 1.0; 2.0; 3.0 |])

    [<Fact>]
    member _.``degree ignores trailing zeros``() =
        // [|3; 0; 0|] normalizes to [|3|] which has degree 0.
        Assert.Equal(0, Polynomial.degree [| 3.0; 0.0; 0.0 |])

    // =========================================================================
    // Zero and One
    // =========================================================================

    [<Fact>]
    member _.``zero returns empty array``() =
        Assert.Equal<float array>([||], Polynomial.zero ())

    [<Fact>]
    member _.``one returns array with single one``() =
        Assert.Equal<float array>([| 1.0 |], Polynomial.one ())

    [<Fact>]
    member _.``zero is additive identity``() =
        let p = [| 1.0; 2.0; 3.0 |]
        // p + 0 = p for any polynomial p.
        Assert.Equal<float array>(p, Polynomial.add p (Polynomial.zero ()))

    [<Fact>]
    member _.``one is multiplicative identity``() =
        let p = [| 1.0; 2.0; 3.0 |]
        // p * 1 = p for any polynomial p.
        Assert.Equal<float array>(p, Polynomial.multiply p (Polynomial.one ()))

    // =========================================================================
    // Add
    // =========================================================================

    [<Fact>]
    member _.``add two constants gives their sum``() =
        // [|3|] + [|4|] = [|7|]
        Assert.Equal<float array>([| 7.0 |], Polynomial.add [| 3.0 |] [| 4.0 |])

    [<Fact>]
    member _.``add polynomials of same length``() =
        // [|1;2|] + [|3;4|] = [|4;6|]  (1+3, 2+4)
        Assert.Equal<float array>([| 4.0; 6.0 |], Polynomial.add [| 1.0; 2.0 |] [| 3.0; 4.0 |])

    [<Fact>]
    member _.``add polynomials of different lengths``() =
        // [|1;2;3|] + [|4;5|] = [|5;7;3|]
        Assert.Equal<float array>([| 5.0; 7.0; 3.0 |], Polynomial.add [| 1.0; 2.0; 3.0 |] [| 4.0; 5.0 |])

    [<Fact>]
    member _.``add with zero polynomial is identity``() =
        let p = [| 1.0; 2.0; 3.0 |]
        Assert.Equal<float array>(p, Polynomial.add p [||])
        Assert.Equal<float array>(p, Polynomial.add [||] p)

    [<Fact>]
    member _.``add cancellation normalizes``() =
        // [|5;7;3|] + [|-5;-7;-3|] = [|0;0;0|] → normalize → [||]
        Assert.Equal<float array>([||], Polynomial.add [| 5.0; 7.0; 3.0 |] [| -5.0; -7.0; -3.0 |])

    [<Fact>]
    member _.``add is commutative``() =
        let a = [| 1.0; 2.0 |]
        let b = [| 1.0; 2.0; 3.0 |]
        Assert.Equal<float array>(Polynomial.add a b, Polynomial.add b a)

    // =========================================================================
    // Subtract
    // =========================================================================

    [<Fact>]
    member _.``subtract same polynomial gives zero``() =
        let p = [| 1.0; 2.0; 3.0 |]
        Assert.Equal<float array>([||], Polynomial.subtract p p)

    [<Fact>]
    member _.``subtract from zero negates``() =
        // [||] - [|1;2|] = [|-1;-2|]
        Assert.Equal<float array>([| -1.0; -2.0 |], Polynomial.subtract [||] [| 1.0; 2.0 |])

    [<Fact>]
    member _.``subtract strips trailing zero``() =
        // [|5;7;3|] - [|1;2;3|] = [|4;5;0|] → normalize → [|4;5|]
        Assert.Equal<float array>([| 4.0; 5.0 |], Polynomial.subtract [| 5.0; 7.0; 3.0 |] [| 1.0; 2.0; 3.0 |])

    [<Fact>]
    member _.``subtract polynomials of different lengths``() =
        // [|1;2;3|] - [|1;2|] = [|0;0;3|] — interior zeros are kept.
        Assert.Equal<float array>([| 0.0; 0.0; 3.0 |], Polynomial.subtract [| 1.0; 2.0; 3.0 |] [| 1.0; 2.0 |])

    // =========================================================================
    // Multiply
    // =========================================================================

    [<Fact>]
    member _.``multiply by zero polynomial gives zero``() =
        Assert.Equal<float array>([||], Polynomial.multiply [| 1.0; 2.0; 3.0 |] [||])
        Assert.Equal<float array>([||], Polynomial.multiply [||] [| 1.0; 2.0; 3.0 |])

    [<Fact>]
    member _.``multiply by one is identity``() =
        let p = [| 1.0; 2.0; 3.0 |]
        Assert.Equal<float array>(p, Polynomial.multiply p [| 1.0 |])
        Assert.Equal<float array>(p, Polynomial.multiply [| 1.0 |] p)

    [<Fact>]
    member _.``multiply two linear polynomials``() =
        // (1 + 2x)(3 + 4x) = 3 + 4x + 6x + 8x² = 3 + 10x + 8x²
        Assert.Equal<float array>([| 3.0; 10.0; 8.0 |], Polynomial.multiply [| 1.0; 2.0 |] [| 3.0; 4.0 |])

    [<Fact>]
    member _.``multiply is commutative``() =
        let a = [| 1.0; 2.0 |]
        let b = [| 1.0; 2.0; 3.0 |]
        Assert.Equal<float array>(Polynomial.multiply a b, Polynomial.multiply b a)

    [<Fact>]
    member _.``multiply degree is sum of degrees``() =
        // degree(a * b) = degree(a) + degree(b)
        let a = [| 1.0; 2.0; 3.0 |]
        let b = [| 3.0; 4.0 |]
        let product = Polynomial.multiply a b
        Assert.Equal(Polynomial.degree a + Polynomial.degree b, Polynomial.degree product)

    [<Fact>]
    member _.``multiply constant scales coefficients``() =
        // [|3|] * [|1;2;3|] = [|3;6;9|]
        Assert.Equal<float array>([| 3.0; 6.0; 9.0 |], Polynomial.multiply [| 3.0 |] [| 1.0; 2.0; 3.0 |])

    [<Fact>]
    member _.``multiply two constants``() =
        Assert.Equal<float array>([| 20.0 |], Polynomial.multiply [| 5.0 |] [| 4.0 |])

    // =========================================================================
    // DivMod
    // =========================================================================

    [<Fact>]
    member _.``divmod by zero throws``() =
        Assert.Throws<InvalidOperationException>(fun () -> Polynomial.divmod [| 1.0; 2.0 |] [||] |> ignore)
        |> ignore

    [<Fact>]
    member _.``divmod when a has lower degree than b``() =
        // degree(a) < degree(b) → quotient = [||], remainder = a.
        let q, r = Polynomial.divmod [| 4.0; 5.0 |] [| 1.0; 2.0; 3.0 |]
        Assert.Equal<float array>([||], q)
        Assert.Equal<float array>([| 4.0; 5.0 |], r)

    [<Fact>]
    member _.``divmod exact division has zero remainder``() =
        // (3 + 10x + 8x²) / (1 + 2x) should divide exactly: (1+2x)(3+4x) = 3+10x+8x²
        let q, r = Polynomial.divmod [| 3.0; 10.0; 8.0 |] [| 1.0; 2.0 |]
        Assert.Equal<float array>([| 3.0; 4.0 |], q)
        Assert.Equal<float array>([||], r)

    [<Fact>]
    member _.``divmod spec example from spec``() =
        // From spec: divide [|5;1;3;2|] = 5 + x + 3x² + 2x³  by  [|2;1|] = 2 + x
        // Expected: quotient = [|3;-1;2|], remainder = [|-1|]
        let q, r = Polynomial.divmod [| 5.0; 1.0; 3.0; 2.0 |] [| 2.0; 1.0 |]
        Assert.Equal<float array>([| 3.0; -1.0; 2.0 |], q)
        Assert.Equal<float array>([| -1.0 |], r)

    [<Fact>]
    member _.``divmod satisfies dividend equality``() =
        // Verify a = b*q + r for a general case.
        let a = [| 5.0; 1.0; 3.0; 2.0 |]
        let b = [| 2.0; 1.0 |]
        let q, r = Polynomial.divmod a b
        // Reconstruct b*q + r and compare to a.
        let reconstructed = Polynomial.add (Polynomial.multiply b q) r
        Assert.Equal<float array>(Polynomial.normalize a, reconstructed)

    [<Fact>]
    member _.``divmod divide by itself gives one and zero``() =
        let p = [| 1.0; 2.0; 3.0 |]
        let q, r = Polynomial.divmod p p
        Assert.Equal<float array>([| 1.0 |], q)
        Assert.Equal<float array>([||], r)

    [<Fact>]
    member _.``divmod divide by constant``() =
        // [|6;4;2|] / [|2|] = [|3;2;1|] with no remainder.
        let q, r = Polynomial.divmod [| 6.0; 4.0; 2.0 |] [| 2.0 |]
        Assert.Equal<float array>([| 3.0; 2.0; 1.0 |], q)
        Assert.Equal<float array>([||], r)

    [<Fact>]
    member _.``divide returns quotient``() =
        let q = Polynomial.divide [| 3.0; 10.0; 8.0 |] [| 1.0; 2.0 |]
        Assert.Equal<float array>([| 3.0; 4.0 |], q)

    [<Fact>]
    member _.``pmod returns remainder``() =
        let r = Polynomial.pmod [| 5.0; 1.0; 3.0; 2.0 |] [| 2.0; 1.0 |]
        Assert.Equal<float array>([| -1.0 |], r)

    // =========================================================================
    // Evaluate
    // =========================================================================

    [<Fact>]
    member _.``evaluate zero polynomial is always zero``() =
        Assert.Equal(0.0, Polynomial.evaluate [||] 0.0)
        Assert.Equal(0.0, Polynomial.evaluate [||] 42.0)
        Assert.Equal(0.0, Polynomial.evaluate [||] -99.0)

    [<Fact>]
    member _.``evaluate constant polynomial at any point``() =
        // [|7|] evaluates to 7 regardless of x.
        Assert.Equal(7.0, Polynomial.evaluate [| 7.0 |] 0.0)
        Assert.Equal(7.0, Polynomial.evaluate [| 7.0 |] 100.0)

    [<Fact>]
    member _.``evaluate linear at zero is constant term``() =
        // p(0) = a₀ for any polynomial (x=0 kills all other terms).
        Assert.Equal(3.0, Polynomial.evaluate [| 3.0; 5.0 |] 0.0)

    [<Fact>]
    member _.``evaluate spec example from spec``() =
        // From spec: [|3; 1; 2|] = 3 + x + 2x² at x = 4 should give 39.
        Assert.Equal(39.0, Polynomial.evaluate [| 3.0; 1.0; 2.0 |] 4.0)

    [<Fact>]
    member _.``evaluate at one is coefficients sum``() =
        // p(1) = sum of all coefficients (every x^i = 1).
        Assert.Equal(6.0, Polynomial.evaluate [| 1.0; 2.0; 3.0 |] 1.0)

    [<Fact>]
    member _.``evaluate at minus one``() =
        // [|1;2;3|] at x = -1: 1 - 2 + 3 = 2.
        Assert.Equal(2.0, Polynomial.evaluate [| 1.0; 2.0; 3.0 |] -1.0)

    [<Fact>]
    member _.``evaluate matches naive formula``() =
        // Cross-check Horner vs direct calculation: [|2; 0; 1|] = 2 + x² at x = 3.
        // Naive: 2 + 0*3 + 1*9 = 11.
        Assert.Equal(11.0, Polynomial.evaluate [| 2.0; 0.0; 1.0 |] 3.0)

    // =========================================================================
    // Gcd
    // =========================================================================

    [<Fact>]
    member _.``gcd with zero polynomial returns other``() =
        // gcd(p, [||]) = p for any p.
        let p = [| 1.0; 2.0; 3.0 |]
        Assert.Equal<float array>(p, Polynomial.gcd p [||])
        Assert.Equal<float array>(p, Polynomial.gcd [||] p)

    [<Fact>]
    member _.``gcd of polynomial with itself is itself``() =
        let p = [| 1.0; 2.0; 3.0 |]
        Assert.Equal<float array>(Polynomial.normalize p, Polynomial.gcd p p)

    [<Fact>]
    member _.``gcd constant result when no common factor``() =
        // (x+1)(x+6) and (x+2)(x+3) share no common non-constant factor.
        // The GCD is a non-zero constant (degree 0).
        let g = Polynomial.gcd [| 6.0; 7.0; 1.0 |] [| 6.0; 5.0; 1.0 |]
        Assert.Equal(0, Polynomial.degree g)                                            // constant
        Assert.Equal<float array>([||], Polynomial.pmod [| 6.0; 7.0; 1.0 |] g)        // divides a
        Assert.Equal<float array>([||], Polynomial.pmod [| 6.0; 5.0; 1.0 |] g)        // divides b

    [<Fact>]
    member _.``gcd extracts common linear factor``() =
        // [|2;3|] = 2+3x and [|4;6|] = 2*(2+3x). GCD is a scalar multiple of [|2;3|].
        let g = Polynomial.gcd [| 2.0; 3.0 |] [| 4.0; 6.0 |]
        // The GCD must divide both inputs with zero remainder.
        Assert.Equal<float array>([||], Polynomial.pmod [| 2.0; 3.0 |] g)
        Assert.Equal<float array>([||], Polynomial.pmod [| 4.0; 6.0 |] g)
        // The degree must be 1 (linear common factor).
        Assert.Equal(1, Polynomial.degree g)

    [<Fact>]
    member _.``gcd is commutative by degree``() =
        // gcd(a, b) and gcd(b, a) must have the same degree and both divide both inputs.
        let a = [| 1.0; 2.0 |]
        let b = [| 3.0; 4.0 |]
        let g1 = Polynomial.gcd a b
        let g2 = Polynomial.gcd b a
        Assert.Equal(Polynomial.degree g1, Polynomial.degree g2)
        Assert.Equal<float array>([||], Polynomial.pmod a g1)
        Assert.Equal<float array>([||], Polynomial.pmod a g2)
