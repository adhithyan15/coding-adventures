namespace CodingAdventures.Trig

open System

/// Every constant of TRIG-REF-1 (PHY00a section 3.2), built from its bit pattern.
///
/// A decimal literal is only a request: "give me the double nearest this
/// decimal".  Whether the answer is the double the specification means depends on
/// the compiler.  Building each constant from the 64 bits the specification pins
/// (BitConverter.Int64BitsToDouble, a reinterpretation with no arithmetic at all)
/// removes that question: the bits in the source are the bits in the machine.
/// The test suite compares every one with the constant table of the shared
/// fixture's manifest.json, through `table`.
///
/// Nothing here is clever.  The comments say what each number is for, because the
/// bits alone say nothing.
module TrigConstants =

    /// Reinterpret 64 bits as an IEEE binary64 number.  No rounding happens.
    let private bits (pattern: int64) : float = BitConverter.Int64BitsToDouble pattern

    // ---------------------------------------------------------------------
    // Circle constants
    // ---------------------------------------------------------------------

    /// pi, 3.141592653589793.
    let pi = bits 0x400921FB54442D18L

    /// One complete rotation, 2 * pi (exact: doubling only changes the exponent).
    let twoPi = bits 0x401921FB54442D18L

    /// A quarter rotation, pi / 2 (exact: halving only changes the exponent).
    let halfPi = bits 0x3FF921FB54442D18L

    // ---------------------------------------------------------------------
    // Range reduction (Cody-Waite)
    // ---------------------------------------------------------------------

    /// The double nearest 2 / pi: x * invPio2 is roughly the number of
    /// quarter-turns in x.
    let invPio2 = bits 0x3FE45F306DC9C883L

    // pi / 2 is stored as a sum of five pieces.  The first four have at most 24
    // significant bits each, so multiplying one of them by an integer k with
    // |k| < 2^29 needs at most 24 + 29 = 53 bits: the product is exact.  That is
    // the Cody-Waite device (Cody and Waite, Software Manual for the Elementary
    // Functions, 1980).  The fifth piece is the remainder, rounded to 53 bits.
    // Together the five carry pi / 2 to about 149 bits.

    /// The first 24 bits of pi / 2.
    let pio2_1 = bits 0x3FF921FB40000000L
    /// The next 24 bits.
    let pio2_2 = bits 0x3E74442D00000000L
    /// The next 24 bits.
    let pio2_3 = bits 0x3CF8469880000000L
    /// The next 24 bits.
    let pio2_4 = bits 0x3B78CC5160000000L
    /// The remainder, rounded to 53 bits.
    let pio2_5 = bits 0x39F01B839A25204AL

    /// 1.5 * 2^52.  Adding it to t (with |t| < 2^51) pushes the fraction bits off
    /// the end of the significand, so (t + magic) - magic is t rounded to the
    /// nearest integer, ties to even, using only an add and a subtract.
    let magic = bits 0x4338000000000000L

    /// 2^29: the edge of the domain of sin, cos and tan.
    let limit = bits 0x41C0000000000000L

    /// 2^-27: below this sin x = x and cos x = 1 to the last bit.
    let tiny = bits 0x3E40000000000000L

    // ---------------------------------------------------------------------
    // Series coefficients
    // ---------------------------------------------------------------------

    /// S[n] = (-1)^n / (2n+1)! for n = 1 .. 9, each rounded once, in order
    /// (the array starts at -1/3! and ends at -1/19!).
    let sinCoefficients =
        [| bits 0xBFC5555555555555L // S1 = -1/3!
           bits 0x3F81111111111111L // S2 = +1/5!
           bits 0xBF2A01A01A01A01AL // S3 = -1/7!
           bits 0x3EC71DE3A556C734L // S4 = +1/9!
           bits 0xBE5AE64567F544E4L // S5 = -1/11!
           bits 0x3DE6124613A86D09L // S6 = +1/13!
           bits 0xBD6AE7F3E733B81FL // S7 = -1/15!
           bits 0x3CE952C77030AD4AL // S8 = +1/17!
           bits 0xBC62F49B46814157L |] // S9 = -1/19!

    /// C[n] = (-1)^n / (2n)! for n = 2 .. 9, in order (+1/4! to -1/18!).  The
    /// n = 1 coefficient, -1/2, is applied as 0.5 * z, which is exact.
    let cosCoefficients =
        [| bits 0x3FA5555555555555L // C2 = +1/4!
           bits 0xBF56C16C16C16C17L // C3 = -1/6!
           bits 0x3EFA01A01A01A01AL // C4 = +1/8!
           bits 0xBE927E4FB7789F5CL // C5 = -1/10!
           bits 0x3E21EED8EFF8D898L // C6 = +1/12!
           bits 0xBDA93974A8C07C9DL // C7 = -1/14!
           bits 0x3D2AE7F3E733B81FL // C8 = +1/16!
           bits 0xBCA6827863B97D97L |] // C9 = -1/18!

    // ---------------------------------------------------------------------
    // Degree conversion
    // ---------------------------------------------------------------------

    /// pi / 180, rounded once.
    let radPerDeg = bits 0x3F91DF46A2529D39L

    /// 180 / pi, rounded once.
    let degPerRad = bits 0x404CA5DC1A63C1F8L

    // ---------------------------------------------------------------------
    // Legacy contract constants (tan, sqrt, atan)
    // ---------------------------------------------------------------------

    /// 1e-15: tan treats |cos| < tanPole as a pole.
    let tanPole = bits 0x3CD203AF9EE75616L

    /// 1e308: the finite value tan returns at a pole (a pinned legacy contract,
    /// not an approximation of the tangent).
    let tanBig = bits 0x7FE1CCF385EBC8A0L

    /// 1e-17: the arctangent series stops once a term is smaller than this.
    let atanTerm = bits 0x3C670EF54646D497L

    /// 1e-15: the relative tolerance of the Newton square-root iteration.
    let sqrtRel = bits 0x3CD203AF9EE75616L

    /// 1e-300: the absolute floor of the same tolerance.
    let sqrtTiny = bits 0x01A56E1FC2F8F359L

    /// Every constant under the name manifest.json gives it.
    let table: (string * float) list =
        [ "PI", pi
          "TWO_PI", twoPi
          "HALF_PI", halfPi
          "INV_PIO2", invPio2
          "PIO2_1", pio2_1
          "PIO2_2", pio2_2
          "PIO2_3", pio2_3
          "PIO2_4", pio2_4
          "PIO2_5", pio2_5
          "MAGIC", magic
          "LIMIT", limit
          "TINY", tiny
          "RAD_PER_DEG", radPerDeg
          "DEG_PER_RAD", degPerRad
          "TAN_POLE", tanPole
          "TAN_BIG", tanBig
          "ATAN_TERM", atanTerm
          "SQRT_REL", sqrtRel
          "SQRT_TINY", sqrtTiny ]
        @ [ for n in 1 .. 9 -> sprintf "S%d" n, sinCoefficients[n - 1] ]
        @ [ for n in 2 .. 9 -> sprintf "C%d" n, cosCoefficients[n - 2] ]
