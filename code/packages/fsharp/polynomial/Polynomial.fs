namespace CodingAdventures.Polynomial

open System

// Polynomial.fs -- Coefficient-array polynomial arithmetic
// =========================================================
//
// A polynomial is a mathematical expression in one variable x built from a
// list of constant coefficients. For example, 3 + 2x + 5x² has three terms:
// a constant 3, a linear term 2x, and a quadratic term 5x².
//
// We store polynomials as plain float arrays where the *array index equals the
// degree* of that term (little-endian — lowest degree first):
//
//   [| 3.0; 2.0; 5.0 |]
//      ^     ^     ^
//      |     |     └── coefficient of x²  (index 2 → degree 2)
//      |     └──────── coefficient of x¹  (index 1 → degree 1)
//      └────────────── coefficient of x⁰  (index 0 → constant)
//
// All operations normalize their output — trailing zero-coefficients are
// always stripped. So [| 1.0; 0.0; 0.0 |] and [| 1.0 |] both represent the
// constant polynomial 1.
//
// Functional style
// ----------------
// F# encourages immutable values and pipeline operators. Every function here
// takes its inputs, builds a fresh array for the result, and normalizes before
// returning. Nothing is mutated after creation (except the one place in DivMod
// where we use a local mutable variable for the loop accumulator, which is
// idiomatic for imperative inner loops in F#).

/// Polynomial type alias: a float array where index i holds the coefficient of xⁱ.
/// The zero polynomial is the empty array [||].
type Polynomial = float array

[<RequireQualifiedAccess>]
module Polynomial =

    /// Package version.
    [<Literal>]
    let VERSION = "0.1.0"

    // =========================================================================
    // Fundamentals
    // =========================================================================

    /// Remove trailing zeros from a polynomial.
    ///
    /// Trailing zeros represent zero-coefficient high-degree terms. They do not
    /// change the mathematical value but do affect degree comparisons and the
    /// stopping condition in polynomial long division.
    ///
    /// Examples:
    ///   normalize [|1; 0; 0|] → [|1|]   constant polynomial 1
    ///   normalize [|0|]       → [||]    zero polynomial
    ///   normalize [|1; 2; 3|] → [|1; 2; 3|]  already normalized
    let normalize (p: Polynomial) : Polynomial =
        let mutable len = p.Length
        while len > 0 && p[len - 1] = 0.0 do
            len <- len - 1
        if len = p.Length then p
        else p[.. len - 1]

    /// Return the degree of a polynomial.
    ///
    /// The degree is the index of the highest non-zero coefficient.
    ///
    /// By convention, the zero polynomial has degree -1. This sentinel lets
    /// polynomial long division terminate cleanly: the loop condition
    /// `degree(remainder) >= degree(divisor)` is false when the remainder is zero.
    ///
    /// Examples:
    ///   degree [|3; 0; 2|] → 2   highest non-zero: index 2, the x² term
    ///   degree [|7|]       → 0   constant polynomial; degree 0
    ///   degree [||]        → -1  zero polynomial; degree -1 by convention
    ///   degree [|0; 0|]    → -1  normalizes to [||]; same as zero polynomial
    let degree (p: Polynomial) : int =
        let n = normalize p
        // When n is empty (zero polynomial) this returns 0 - 1 = -1.
        n.Length - 1

    /// Return the zero polynomial (empty array).
    ///
    /// Zero is the additive identity: add zero p = p for any p.
    let zero () : Polynomial = [||]

    /// Return the multiplicative identity polynomial [|1|].
    ///
    /// Multiplying any polynomial by one returns that polynomial unchanged.
    let one () : Polynomial = [| 1.0 |]

    // =========================================================================
    // Addition and Subtraction
    // =========================================================================

    /// Add two polynomials term-by-term.
    ///
    /// Addition aligns matching coefficients by index. The shorter polynomial
    /// is implicitly padded with zeros.
    ///
    /// Visual example:
    ///   [|1; 2; 3|]   =  1 + 2x + 3x²
    /// + [|4; 5|]      =  4 + 5x
    /// ─────────────────────────────────
    ///   [|5; 7; 3|]   =  5 + 7x + 3x²
    let add (a: Polynomial) (b: Polynomial) : Polynomial =
        let len = max a.Length b.Length
        let result =
            Array.init len (fun i ->
                let ai = if i < a.Length then a[i] else 0.0
                let bi = if i < b.Length then b[i] else 0.0
                ai + bi)
        normalize result

    /// Subtract polynomial b from polynomial a term-by-term.
    ///
    /// Equivalent to add a (negate b), but computed directly.
    ///
    /// Visual example:
    ///   [|5; 7; 3|]   =  5 + 7x + 3x²
    /// - [|1; 2; 3|]   =  1 + 2x + 3x²
    /// ─────────────────────────────────
    ///   [|4; 5; 0|]   →  normalize  →  [|4; 5|]   =  4 + 5x
    ///
    /// Note: 3x² − 3x² = 0; normalize strips the trailing zero.
    let subtract (a: Polynomial) (b: Polynomial) : Polynomial =
        let len = max a.Length b.Length
        let result =
            Array.init len (fun i ->
                let ai = if i < a.Length then a[i] else 0.0
                let bi = if i < b.Length then b[i] else 0.0
                ai - bi)
        normalize result

    // =========================================================================
    // Multiplication
    // =========================================================================

    /// Multiply two polynomials using polynomial convolution.
    ///
    /// Each term a[i]·xⁱ of a multiplies each term b[j]·xʲ of b, contributing
    /// a[i]·b[j] to the result at index i+j.
    ///
    /// If a has degree m and b has degree n, the result has degree m+n.
    ///
    /// Visual example:
    ///   [|1; 2|]  =  1 + 2x
    /// × [|3; 4|]  =  3 + 4x
    /// ─────────────────────────────────────────────────────
    /// result array length 3, initialized to [|0; 0; 0|]:
    ///   i=0, j=0: result[0] += 1·3 = 3   → [|3; 0; 0|]
    ///   i=0, j=1: result[1] += 1·4 = 4   → [|3; 4; 0|]
    ///   i=1, j=0: result[1] += 2·3 = 6   → [|3; 10; 0|]
    ///   i=1, j=1: result[2] += 2·4 = 8   → [|3; 10; 8|]
    ///
    /// Result: [|3; 10; 8|]  =  3 + 10x + 8x²
    /// Verify: (1+2x)(3+4x) = 3+4x+6x+8x² = 3+10x+8x²  ✓
    let multiply (a: Polynomial) (b: Polynomial) : Polynomial =
        // Multiplying by the zero polynomial yields zero.
        if a.Length = 0 || b.Length = 0 then [||]
        else
            // Result degree = deg(a) + deg(b), so result length = a.Length + b.Length - 1.
            let resultLen = a.Length + b.Length - 1
            let result = Array.zeroCreate<float> resultLen

            for i in 0 .. a.Length - 1 do
                for j in 0 .. b.Length - 1 do
                    result[i + j] <- result[i + j] + a[i] * b[j]

            normalize result

    // =========================================================================
    // Division
    // =========================================================================

    /// Perform polynomial long division, returning (quotient, remainder).
    ///
    /// Given polynomials a and b (b ≠ zero), finds q and r such that:
    ///   a = b × q + r   and   degree(r) < degree(b)
    ///
    /// Algorithm — analogous to school long division:
    ///   1. Find the leading term of the current remainder.
    ///   2. Divide it by the leading term of b → next quotient coefficient.
    ///   3. Subtract (quotient term) × b from the remainder.
    ///   4. Repeat until degree(remainder) < degree(b).
    ///
    /// Detailed walkthrough: divide [|5;1;3;2|] = 5+x+3x²+2x³  by  [|2;1|] = 2+x
    ///
    ///   Step 1: remainder = [|5;1;3;2|], deg=3.
    ///           Leading = 2x³, divisor leading = 1x.
    ///           Quotient term: 2x³/x = 2x²  → q[2] = 2
    ///           Subtract 2x² × (2+x) = 4x²+2x³ = [|0;0;4;2|]:
    ///           [|5;1;3-4;2-2|] = [|5;1;-1;0|] → normalize → [|5;1;-1|]
    ///
    ///   Step 2: remainder = [|5;1;-1|], deg=2.
    ///           Leading = -x², divisor leading = x.
    ///           Quotient term: -x²/x = -x  → q[1] = -1
    ///           Subtract -x × (2+x) = [|0;-2;-1|]:
    ///           [|5;3;0|] → [|5;3|]
    ///
    ///   Step 3: remainder = [|5;3|], deg=1.
    ///           Leading = 3x, divisor leading = x.
    ///           Quotient term: 3x/x = 3  → q[0] = 3
    ///           Subtract 3 × [|2;1|] = [|6;3|]:
    ///           [|-1;0|] → [|-1|]
    ///
    ///   Step 4: degree([|-1|]) = 0 < 1 = degree(b). STOP.
    ///   Result: q = [|3;-1;2|],  r = [|-1|]
    ///   Verify: (x+2)(3-x+2x²) + (-1) = 3x-x²+2x³+6-2x+4x² - 1 = 5+x+3x²+2x³  ✓
    let divmod (a: Polynomial) (b: Polynomial) : Polynomial * Polynomial =
        let nb = normalize b
        if nb.Length = 0 then
            raise (InvalidOperationException("polynomial division by zero"))

        let na = normalize a
        let degA = na.Length - 1
        let degB = nb.Length - 1

        // If a has lower degree than b, quotient is zero, remainder is a.
        if degA < degB then
            [||], na
        else
            // Work on a mutable copy of the remainder.
            let rem = Array.copy na
            // Allocate the quotient with the right length.
            let quot = Array.zeroCreate<float> (degA - degB + 1)

            // Leading coefficient of the divisor.
            let leadB = nb[degB]

            // Current degree of the remainder — walks downward as we subtract.
            let mutable degRem = degA

            while degRem >= degB do
                // Quotient coefficient for this step.
                let coeff = rem[degRem] / leadB
                let power = degRem - degB
                quot[power] <- coeff

                // Subtract coeff * xᵖᵒʷᵉʳ * b from rem.
                for j in 0 .. degB do
                    rem[power + j] <- rem[power + j] - coeff * nb[j]

                // The leading term is now zero by construction. Scan down past
                // any new trailing zeros to find the true new degree.
                degRem <- degRem - 1
                while degRem >= 0 && rem[degRem] = 0.0 do
                    degRem <- degRem - 1

            normalize quot, normalize rem

    /// Return the quotient of dividing a by b.
    ///
    /// Raises InvalidOperationException if b is the zero polynomial.
    let divide (a: Polynomial) (b: Polynomial) : Polynomial = fst (divmod a b)

    /// Return the remainder of dividing a by b.
    ///
    /// This is the polynomial "modulo" operation. In GF(2^8) field construction,
    /// a high-degree polynomial is reduced modulo the primitive polynomial this way.
    ///
    /// Raises InvalidOperationException if b is the zero polynomial.
    let pmod (a: Polynomial) (b: Polynomial) : Polynomial = snd (divmod a b)

    // =========================================================================
    // Evaluation
    // =========================================================================

    /// Evaluate a polynomial at x using Horner's method.
    ///
    /// Horner's method rewrites the polynomial in nested form:
    ///   a₀ + x(a₁ + x(a₂ + ... + x·aₙ))
    ///
    /// This requires only n additions and n multiplications — no powers of x at
    /// all, compared to the naïve approach that requires n exponentiations.
    ///
    /// Algorithm (reading coefficients from high degree down to the constant):
    ///   acc = 0
    ///   for i from n downto 0:
    ///       acc = acc * x + p[i]
    ///   return acc
    ///
    /// Example: evaluate [|3; 1; 2|] = 3 + x + 2x² at x = 4:
    ///   Start: acc = 0
    ///   i=2: acc = 0*4 + 2 = 2
    ///   i=1: acc = 2*4 + 1 = 9
    ///   i=0: acc = 9*4 + 3 = 39
    ///   Verify: 3 + 4 + 2·16 = 3 + 4 + 32 = 39  ✓
    let evaluate (p: Polynomial) (x: float) : float =
        let n = normalize p
        if n.Length = 0 then 0.0
        else
            // Array.foldBack processes from the right (high degree) to the left (constant).
            // acc starts at 0; each step: new_acc = old_acc * x + n[i].
            Array.foldBack (fun coeff acc -> acc * x + coeff) n 0.0

    // =========================================================================
    // Greatest Common Divisor
    // =========================================================================

    /// Compute the GCD of two polynomials using the Euclidean algorithm.
    ///
    /// The Euclidean algorithm for polynomials is identical to the integer
    /// version, with polynomial mod in place of integer mod:
    ///
    ///   while b ≠ zero:
    ///       a, b = b, a mod b
    ///   return normalize a
    ///
    /// The result is the highest-degree polynomial that divides both inputs with
    /// zero remainder.
    ///
    /// Use case: Reed-Solomon decoding uses the extended Euclidean algorithm on
    /// polynomials to recover error-locator and error-evaluator polynomials.
    let gcd (a: Polynomial) (b: Polynomial) : Polynomial =
        let mutable u = normalize a
        let mutable v = normalize b

        while v.Length > 0 do
            let r = pmod u v
            u <- v
            v <- r

        normalize u
