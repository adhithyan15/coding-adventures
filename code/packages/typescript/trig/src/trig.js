"use strict";
// ============================================================================
// trig.ts — Trigonometric Functions from First Principles
// ============================================================================
//
// This module computes sin(x) and cos(x) using their Maclaurin series
// (Taylor series centered at zero). No built-in Math.sin or Math.cos
// is used — everything is derived from basic arithmetic.
//
// Why Taylor series?
// ------------------
// Brook Taylor (1715) and Colin Maclaurin (1742) showed that smooth
// functions can be expressed as infinite sums of polynomial terms.
// For trigonometry, these series converge for ALL real numbers,
// making them a universal computation method.
//
// The Maclaurin series for sine:
//
//   sin(x) = x - x³/3! + x⁵/5! - x⁷/7! + ...
//          = Σ (-1)^n · x^(2n+1) / (2n+1)!    for n = 0, 1, 2, ...
//
// The Maclaurin series for cosine:
//
//   cos(x) = 1 - x²/2! + x⁴/4! - x⁶/6! + ...
//          = Σ (-1)^n · x^(2n) / (2n)!        for n = 0, 1, 2, ...
//
// Each successive term is smaller than the last (for |x| < some bound),
// so the series converges quickly — especially after range reduction.
// ============================================================================
Object.defineProperty(exports, "__esModule", { value: true });
exports.PI = void 0;
exports.sin = sin;
exports.cos = cos;
exports.radians = radians;
exports.degrees = degrees;
// ----------------------------------------------------------------------------
// Constants
// ----------------------------------------------------------------------------
// PI to double-precision accuracy (same as Math.PI). This is the ratio of
// a circle's circumference to its diameter — the most fundamental constant
// in trigonometry.
exports.PI = 3.141592653589793;
// TWO_PI represents one full revolution (360°). We use it for range
// reduction: since sin and cos are periodic with period 2π, we can
// always reduce the input to a small range before computing the series.
const TWO_PI = 2.0 * exports.PI;
// ----------------------------------------------------------------------------
// Range Reduction
// ----------------------------------------------------------------------------
// The Taylor series converges fastest when |x| is small. Since sin and cos
// repeat every 2π, we can subtract multiples of 2π to bring x into [-π, π].
//
// Example: sin(1000π) = sin(0) = 0, because 1000π is an exact multiple of π.
//
// Without range reduction, computing sin(1000) would require many more terms
// to converge, and floating-point errors would accumulate badly.
function rangeReduce(x) {
    // Step 1: Reduce modulo 2π to get into [0, 2π) or (-2π, 0]
    x = x % TWO_PI;
    // Step 2: Shift into [-π, π] for optimal convergence
    // If x > π, subtract 2π to bring it to the negative side
    if (x > exports.PI) {
        x -= TWO_PI;
    }
    // If x < -π, add 2π to bring it to the positive side
    if (x < -exports.PI) {
        x += TWO_PI;
    }
    return x;
}
// ----------------------------------------------------------------------------
// sin(x) — Sine via Maclaurin Series
// ----------------------------------------------------------------------------
// Computes sin(x) using the iterative form of the Maclaurin series.
//
// Instead of computing x^n and n! separately (which overflow quickly),
// we compute each term from the previous one:
//
//   term_0 = x
//   term_n = term_{n-1} * (-x²) / ((2n)(2n+1))
//
// This is numerically stable and avoids large intermediate values.
//
// We use 20 terms, which gives approximately 15 digits of precision
// for inputs in [-π, π] — more than enough for double-precision floats.
function sin(x) {
    x = rangeReduce(x);
    let sum = 0.0;
    let term = x; // First term: x¹/1! = x
    for (let n = 1; n <= 20; n++) {
        // Accumulate this term into the running sum
        sum += term;
        // Derive the next term from the current one:
        //   next = current * (-x²) / ((2n)(2n+1))
        //
        // This works because:
        //   term_n   = (-1)^(n-1) · x^(2(n-1)+1) / (2(n-1)+1)!
        //   term_n+1 = (-1)^n     · x^(2n+1)     / (2n+1)!
        //   ratio    = -x² / ((2n)(2n+1))
        term *= (-x * x) / (2 * n * (2 * n + 1));
    }
    return sum;
}
// ----------------------------------------------------------------------------
// cos(x) — Cosine via Maclaurin Series
// ----------------------------------------------------------------------------
// Same approach as sin(x), but starting from term_0 = 1 (the constant term)
// and using even powers of x:
//
//   term_0 = 1
//   term_n = term_{n-1} * (-x²) / ((2n-1)(2n))
//
// The recurrence ratio differs slightly from sine because cosine uses
// even-indexed terms: x⁰/0!, x²/2!, x⁴/4!, etc.
function cos(x) {
    x = rangeReduce(x);
    let sum = 0.0;
    let term = 1.0; // First term: x⁰/0! = 1
    for (let n = 1; n <= 20; n++) {
        // Accumulate this term
        sum += term;
        // Derive next term:
        //   next = current * (-x²) / ((2n-1)(2n))
        //
        // The denominator uses (2n-1)(2n) because cosine terms are:
        //   1, -x²/2!, x⁴/4!, -x⁶/6!, ...
        // and 2!/0! = 2·1, 4!/2! = 4·3, 6!/4! = 6·5, etc.
        term *= (-x * x) / ((2 * n - 1) * (2 * n));
    }
    return sum;
}
// ----------------------------------------------------------------------------
// Unit Conversions
// ----------------------------------------------------------------------------
// Degrees and radians are two ways to measure angles:
//   - Degrees: a full circle = 360°  (convenient for humans)
//   - Radians: a full circle = 2π    (natural for mathematics)
//
// Conversion formulas:
//   radians = degrees × (π / 180)
//   degrees = radians × (180 / π)
function radians(deg) {
    return deg * (exports.PI / 180.0);
}
function degrees(rad) {
    return rad * (180.0 / exports.PI);
}
//# sourceMappingURL=trig.js.map