namespace CodingAdventures.Trig

open System

/// Trigonometric functions from first principles.
///
/// We deliberately build these operations from arithmetic and iterative
/// algorithms so the source teaches how a runtime approximates transcendental
/// functions under the hood.
[<RequireQualifiedAccess>]
module Trig =
    [<Literal>]
    let PI = 3.141592653589793

    let private halfPi = TrigConstants.halfPi

    /// A quiet NaN.  A NaN compares false with everything, which is exactly how
    /// the domain guard and the NaN rules below detect it.
    let private nan = BitConverter.Int64BitsToDouble 0x7FF8000000000000L

    // -----------------------------------------------------------------------
    // Range reduction
    // -----------------------------------------------------------------------

    /// Split x into whole quarter-turns and a leftover: x = k * (pi/2) + r with
    /// |r| <= pi/4 (plus a few ulps, because the quotient is rounded before k is
    /// chosen).  Returns (k, r) with k an integer-valued float, |k| < 2^29.
    ///
    /// Step 1, the quotient.  t = x * (2/pi) is approximately the number of
    /// quarter-turns.  (t + magic) - magic rounds it to the nearest integer (ties
    /// to even): magic = 1.5 * 2^52 has its units digit in the last place of the
    /// significand, so the addition rounds t to a whole number by the IEEE rule,
    /// and the subtraction is exact.  No floor, no round, no float-to-integer cast.
    ///
    /// Step 2, the remainder.  pi/2 is irrational, so k * (pi/2) cannot be formed
    /// exactly.  It is stored as five pieces (pio2_1 .. pio2_5), the first four so
    /// short (24 bits) that k * piece is exact, and removed from x one at a time:
    ///
    ///   x ------------------------------------------------------------ the angle
    ///   |-- k*pio2_1 --|                                                24 bits of k*pi/2
    ///                  |-- k*pio2_2 --|                                next 24
    ///                                 |-- k*pio2_3 --|                 next 24
    ///                                                |- k*pio2_4 -|    next 24
    ///                                                             |k*pio2_5|  last, rounds
    ///   r = what is left
    ///
    /// The first two subtractions are exact; the last three round, each by at most
    /// half an ulp of the current r.  Each product is bound to its own name before
    /// the subtraction, so there is nothing for a compiler to fuse (the .NET JIT
    /// never contracts a*b+c on its own; Math.FusedMultiplyAdd is never called).
    let private reduce (x: float) : struct (float * float) =
        let t = x * TrigConstants.invPio2
        let sum = t + TrigConstants.magic
        let k = sum - TrigConstants.magic
        let m1 = k * TrigConstants.pio2_1
        let r1 = x - m1
        let m2 = k * TrigConstants.pio2_2
        let r2 = r1 - m2
        let m3 = k * TrigConstants.pio2_3
        let r3 = r2 - m3
        let m4 = k * TrigConstants.pio2_4
        let r4 = r3 - m4
        let m5 = k * TrigConstants.pio2_5
        let r5 = r4 - m5
        struct (k, r5)

    /// The quadrant, k mod 4.  k is an integer-valued float with |k| < 2^29, so
    /// the conversion to int is exact and `&&& 3` on a two's-complement integer is
    /// the floor modulus (k = -1 gives 3, k = -5 gives 3).  It is the only integer
    /// operation in the algorithm.
    let private quadrant (k: float) : int = int k &&& 3

    // -----------------------------------------------------------------------
    // The kernels: Maclaurin series by Horner's rule on stored coefficients
    // -----------------------------------------------------------------------

    /// sin r for |r| <= pi/4, ten terms (r^1 .. r^19):
    ///
    ///   sin r = r + r*z*(S1 + S2*z + S3*z^2 + ... + S9*z^8),     z = r*r
    ///
    /// The coefficients are the Maclaurin series, S[n] = (-1)^n / (2n+1)!:
    ///
    ///   S1 = -1/3!   S2 = +1/5!   S3 = -1/7!   S4 = +1/9!   S5 = -1/11!
    ///   S6 = +1/13!  S7 = -1/15!  S8 = +1/17!  S9 = -1/19!
    ///
    /// Horner's rule starts with the innermost (smallest) coefficient, so small
    /// terms are added first, which is the accurate order.  There is no division,
    /// and one multiply and one add per step cannot be grouped two ways.  The
    /// leading r is added last, so the dominant part of the result carries a single
    /// rounding.  Every product is its own statement: two roundings, never one.
    let private sinKernel (r: float) : float =
        let z = r * r
        let coefficients = TrigConstants.sinCoefficients
        let mutable p = coefficients[8]

        for n in 7 .. -1 .. 0 do
            let scaled = p * z
            p <- scaled + coefficients[n]

        let w = r * z
        let wp = w * p
        r + wp

    /// cos r for |r| <= pi/4, ten terms (r^0 .. r^18):
    ///
    ///   cos r = (1 - z/2) + z*z*(C2 + C3*z + ... + C9*z^7),      z = r*r
    ///
    /// with C[n] = (-1)^n / (2n)!:
    ///
    ///   C2 = +1/4!   C3 = -1/6!   C4 = +1/8!   C5 = -1/10!
    ///   C6 = +1/12!  C7 = -1/14!  C8 = +1/16!  C9 = -1/18!
    ///
    /// 0.5 * z is exact.
    let private cosKernel (r: float) : float =
        let z = r * r
        let coefficients = TrigConstants.cosCoefficients
        let mutable q = coefficients[7]

        for n in 6 .. -1 .. 0 do
            let scaled = q * z
            q <- scaled + coefficients[n]

        let half = 0.5 * z
        let u = 1.0 - half
        let v = z * z
        let vq = v * q
        u + vq

    // -----------------------------------------------------------------------
    // sin, cos, tan
    // -----------------------------------------------------------------------

    /// Sine of an angle in radians, accurate to 3 ulp on |x| <= 2^29; NaN outside.
    ///
    ///   q = k mod 4:   0 -> sin r     1 -> cos r     2 -> -sin r     3 -> -cos r
    ///
    /// sin(-0) is -0 (the series would give +0, so the argument is returned
    /// itself), and 0 < |x| < 2^-27 returns x because the next term is below half
    /// an ulp.  The guard is a negated <= on purpose: NaN fails every comparison,
    /// so `not (|x| <= limit)` catches NaN, both infinities and every finite value
    /// beyond the domain in one test.  Writing `|x| > limit` would let a NaN
    /// through.
    let sin (x: float) : float =
        if not (abs x <= TrigConstants.limit) then
            nan
        elif abs x < TrigConstants.tiny then
            x
        else
            let struct (k, r) = reduce x

            match quadrant k with
            | 0 -> sinKernel r
            | 1 -> cosKernel r
            | 2 -> -(sinKernel r)
            | _ -> -(cosKernel r)

    /// Cosine of an angle in radians, accurate to 3 ulp on |x| <= 2^29; NaN outside.
    ///
    ///   q = k mod 4:   0 -> cos r     1 -> -sin r     2 -> -cos r     3 -> sin r
    let cos (x: float) : float =
        if not (abs x <= TrigConstants.limit) then
            nan
        elif abs x < TrigConstants.tiny then
            1.0
        else
            let struct (k, r) = reduce x

            match quadrant k with
            | 0 -> cosKernel r
            | 1 -> -(sinKernel r)
            | 2 -> -(cosKernel r)
            | _ -> sinKernel r

    /// Convert degrees to radians: one multiply by the bit-pinned pi/180 (one
    /// rounding, no overflowing intermediate).  Zero keeps its sign.
    let radians (degrees: float) = degrees * TrigConstants.radPerDeg

    /// Convert radians to degrees: one multiply by the bit-pinned 180/pi.
    let degrees (radians: float) = radians * TrigConstants.degPerRad

    let sqrt (x: float) =
        if x < 0.0 then
            raise (ArgumentOutOfRangeException("x", x, "sqrt: domain error -- input is negative"))

        elif x = 0.0 then
            x

        elif Double.IsPositiveInfinity x then
            x

        else
            let mutable scaled = x
            let mutable resultScale = 1.0
            while scaled < 0.25 do
                scaled <- scaled * 4.0
                resultScale <- resultScale * 0.5
            while scaled >= 4.0 do
                scaled <- scaled * 0.25
                resultScale <- resultScale * 2.0

            let mutable guess = if scaled >= 1.0 then scaled else 1.0
            let mutable converged = false

            for _ in 0 .. 59 do
                if not converged then
                    let next = (guess + scaled / guess) / 2.0
                    if abs (next - guess) < TrigConstants.sqrtRel * guess + TrigConstants.sqrtTiny then
                        converged <- true
                    guess <- next

            guess * resultScale

    /// Tangent as sin / cos, with a pinned legacy contract at the poles: where
    /// |cos| < 1e-15 the result is +-1e308 (the sign of the sine), not an
    /// approximation of the tangent and not infinity.  No accuracy claim is made
    /// for tan.  Outside the domain both sin and cos are NaN, the pole test is
    /// false, and NaN / NaN is NaN.
    let tan (x: float) =
        let sine = sin x
        let cosine = cos x

        if abs cosine < TrigConstants.tanPole then
            if sine > 0.0 then TrigConstants.tanBig else -TrigConstants.tanBig
        else
            sine / cosine

    /// The arctangent series on |x| <= 1 after a half-angle reduction
    /// t = x / (1 + sqrt(1 + x*x)), summed left to right until a term drops below
    /// 1e-17 (at most 30 terms):
    ///
    ///   term_n = ((term_(n-1) * (-t*t)) * (2n-1)) / (2n+1)
    let private atanCore (x: float) =
        let reduced = x / (1.0 + sqrt (1.0 + x * x))
        let squared = reduced * reduced
        let mutable term = reduced
        let mutable result = reduced
        let mutable n = 1
        let mutable running = true

        while running && n <= 30 do
            term <- term * (-squared) * float (2 * n - 1) / float (2 * n + 1)
            result <- result + term

            if abs term < TrigConstants.atanTerm then
                running <- false

            n <- n + 1

        2.0 * result

    /// Arctangent in the range (-pi/2, pi/2).
    let atan (x: float) =
        // atan(x) rounds exactly to x here; avoid halving subnormals and retain -0.
        if abs x <= TrigConstants.tiny then
            x
        elif x > 1.0 then
            halfPi - atanCore (1.0 / x)
        elif x < -1.0 then
            -halfPi - atanCore (1.0 / x)
        else
            atanCore x

    /// Four-quadrant arctangent in the range (-pi, pi].  A NaN in either position
    /// gives NaN (the old guards fell through to 0 or +-pi/2, a finite answer to
    /// an undefined question).
    let atan2 (y: float) (x: float) =
        if Double.IsNaN x || Double.IsNaN y then
            nan
        elif x > 0.0 then
            atan (y / x)
        elif x < 0.0 && y >= 0.0 then
            atan (y / x) + PI
        elif x < 0.0 && y < 0.0 then
            atan (y / x) - PI
        elif x = 0.0 && y > 0.0 then
            halfPi
        elif x = 0.0 && y < 0.0 then
            -halfPi
        else
            0.0
