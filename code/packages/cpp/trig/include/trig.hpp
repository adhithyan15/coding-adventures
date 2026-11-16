// trig.hpp — trigonometric functions from first principles, header-only in pure
// ISO C++17 (namespace ca::trig). The C++ copy of the TRIG-REF-1 reference
// algorithm of PHY00a (code/specs/PHY00a-trig-convergence.md).
// ===========================================================================
//
// `sin`, `cos`, `radians` and `degrees` are built from addition, subtraction,
// multiplication and comparison ALONE: no <cmath>, no std::sin, no libm, no
// division. Every operation rounds once and nothing is fused, so the answer is
// BIT FOR BIT the same as in every other language port; the shared vectors in
// code/specs/fixtures/trig-reference-v1 (read by tests/conformance_test.cpp)
// prove it.
//
// THE BIG IDEA. A sine repeats, so an angle splits into whole quarter-turns and
// a small leftover:
//
//     x = k * (pi/2) + r,        |r| <= pi/4,        k a whole number
//
// sin(x) is one of  sin r, cos r, -sin r, -cos r  chosen by k mod 4, and cos(x)
// is one of  cos r, -sin r, -cos r, sin r:
//
//     k mod 4 |  0       1       2       3
//     --------+-------------------------------
//     sin(x)  |  sin r   cos r  -sin r  -cos r
//     cos(x)  |  cos r  -sin r  -cos r   sin r
//
// STEP 1, THE QUOTIENT. MAGIC = 1.5 * 2^52 has its units digit in the very last
// place of a double's significand, so adding it to t rounds t to a whole number
// (ties to even) and subtracting it again hands that whole number back:
//
//     k = (x * (2/pi) + MAGIC) - MAGIC            // nearest integer to x/(pi/2)
//
// It needs only `+` and `-`: no floor, no round, no float-to-integer cast, and
// so nothing language-dependent at NaN or infinity.
//
// STEP 2, THE REMAINDER (Cody-Waite). pi/2 is irrational, so one 53-bit pi/2
// times a big k loses exactly the digits we need. We store pi/2 as a SUM of five
// pieces, the first four with at most 24 significant bits, so that k * piece is
// EXACT whenever |k| < 2^29 (24 + 29 = 53 bits):
//
//   x ---------------------------------------------------------- the angle
//   |-- k*PIO2_1 --|                                              24 bits
//                  |-- k*PIO2_2 --|                               next 24
//                                 |-- k*PIO2_3 --|                next 24
//                                                |- k*PIO2_4 -|   next 24
//                                                             |k*PIO2_5| rounds
//   r = what is left over
//
// STEP 3, THE KERNELS. On |r| <= pi/4 the Maclaurin series converge so fast that
// ten terms are more than a double can resolve. Horner's rule, smallest term
// first, on STORED coefficients (so a reader can see the factorials):
//
//     sin r = r + r*z*(S1 + S2 z + ... + S9 z^8)           z = r*r
//     cos r = (1 - z/2) + z*z*(C2 + C3 z + ... + C9 z^7)
//
//     n    | S[n] = (-1)^n/(2n+1)!        C[n] = (-1)^n/(2n)!
//     -----+-------------------------------------------------
//     1    | -1/3!                        (-1/2!, applied as 0.5*z)
//     2    | +1/5!                        +1/4!
//     3    | -1/7!                        -1/6!
//     4    | +1/9!                        +1/8!
//     5    | -1/11!                       -1/10!
//     6    | +1/13!                       +1/12!
//     7    | -1/15!                       -1/14!
//     8    | +1/17!                       +1/16!
//     9    | -1/19!                       -1/18!
//
// DOMAIN AND ACCURACY. |x| <= 2^29 (about 5.4e8 radians). NaN, both infinities
// and every larger value return NaN: a finite answer there would be wrong but
// look right. Inside the domain sin and cos are within 3 ulp of the exact value
// (2.38 observed). sin(-0) is -0; |x| < 2^-27 returns x itself (and cos 1).
// radians and degrees are one multiply by a stored constant (within 1 ulp).
//
// tan, sqrt, atan and atan2 (PHY00a 3.8, 3.9) are the pre-existing algorithms,
// pinned to the bit and checked by the same vectors; no accuracy is claimed for
// them. tan keeps its pole sentinel (+-1e308 when |cos| < 1e-15); atan2 returns
// NaN for a NaN argument.
//
// NO FUSED MULTIPLY-ADD. A fused multiply-add rounds `a*b + c` once instead of
// twice, which changes last bits. Every product that feeds an addition is
// therefore written as its own statement, AND the header turns contraction off
// for its own code, because a header is compiled with the CONSUMER's flags, not
// the package's:
//   - clang:  _Pragma("STDC FP_CONTRACT OFF") at the top of each function
//             (clang honours it against its default, not against an explicit
//             -ffp-contract=fast: the consumer's build must not pass that);
//   - GCC:    #pragma GCC optimize ("fp-contract=off") around the code (GCC
//             IGNORES the STDC pragma and warns about it under -Wall, so it
//             gets its own branch; clang does not know the GCC pragma);
//   - MSVC:   #pragma fp_contract(off).
// Each pragma sits behind its own #if so -pedantic-errors -Wall -Wextra -Werror
// stays clean on every compiler. -ffast-math and friends, and any target that
// evaluates doubles in extended precision (x87, FLT_EVAL_METHOD != 0), are
// refused at compile time. Defining CA_TRIG_NO_FP_GUARDS compiles the pragmas
// out; it exists only so the build can run the fusion canary (tools/run.sh).
//
// DIVERGENCE FROM RUST. Rust's `sqrt` panics on a negative input; this port
// throws std::domain_error, the idiomatic C++ equivalent. Every other function
// is total and returns a double, exactly like the Rust crate.
//
// PORTABILITY. Pure ISO C++17 — no <cmath>, no compiler extensions beyond the
// guarded pragmas. Hex float literals (C++17) pin every constant by its bits.
#ifndef CA_TRIG_HPP
#define CA_TRIG_HPP

#include <cfloat>
#include <limits>
#include <stdexcept>

#if defined(__FAST_MATH__) || (defined(__FINITE_MATH_ONLY__) && __FINITE_MATH_ONLY__)
#error "trig.hpp needs strict IEEE arithmetic: do not build with -ffast-math or -ffinite-math-only"
#endif

// Doubles must be evaluated in binary64 (SSE2, ARM64); x87 extended precision
// rounds twice and breaks the add-MAGIC trick.
static_assert(FLT_EVAL_METHOD == 0,
              "trig.hpp needs FLT_EVAL_METHOD == 0 (no extended-precision intermediates)");

// ── Per-compiler "never fuse a*b+c" guard ────────────────────────────────────
//
// CA_TRIG_NO_CONTRACT is written as the first statement of every function that
// does arithmetic. Only clang needs it in function scope (its pragma is lexical
// and would leak into the includer's code at file scope); GCC and MSVC use a
// region guard below.
#if !defined(CA_TRIG_NO_FP_GUARDS) && defined(__clang__)
#define CA_TRIG_NO_CONTRACT _Pragma("STDC FP_CONTRACT OFF")
#else
#define CA_TRIG_NO_CONTRACT
#endif

#if !defined(CA_TRIG_NO_FP_GUARDS)
#if defined(__clang__)
// handled per function above
#elif defined(__GNUC__)
#pragma GCC push_options
#pragma GCC optimize("fp-contract=off")
#elif defined(_MSC_VER)
#pragma fp_contract(off)
#endif
#endif

namespace ca {
namespace trig {

// The ratio of a circle's circumference to its diameter — hand-written to f64
// precision so the library is fully self-contained.
inline constexpr double PI = 3.141592653589793;           // 0x400921FB54442D18
inline constexpr double TWO_PI = 2.0 * PI;                // 0x401921FB54442D18
inline constexpr double HALF_PI = PI / 2.0;               // 0x3FF921FB54442D18

namespace detail {

// ── The constants of PHY00a 3.2, as hex floats (their bits are authoritative;
// tests/conformance_test.cpp asserts every one) ────────────────────────────────

// 2/pi, the double nearest to it.
inline constexpr double INV_PIO2 = 0x1.45f306dc9c883p-1;  // 0x3FE45F306DC9C883
// pi/2 as five pieces: four of at most 24 significant bits (so k*piece is exact
// for |k| < 2^29), then the remainder rounded to 53 bits.
inline constexpr double PIO2_1 = 0x1.921fb4p+0;           // 0x3FF921FB40000000
inline constexpr double PIO2_2 = 0x1.4442dp-24;           // 0x3E74442D00000000
inline constexpr double PIO2_3 = 0x1.846988p-48;          // 0x3CF8469880000000
inline constexpr double PIO2_4 = 0x1.8cc516p-72;          // 0x3B78CC5160000000
inline constexpr double PIO2_5 = 0x1.01b839a25204ap-96;   // 0x39F01B839A25204A
// 1.5 * 2^52: add then subtract it to round to the nearest integer.
inline constexpr double MAGIC = 0x1.8p+52;                // 0x4338000000000000
// The edge of the domain, 2^29.
inline constexpr double LIMIT = 0x1p+29;                  // 0x41C0000000000000
// Below 2^-27, sin(x) == x exactly and cos(x) == 1.
inline constexpr double TINY = 0x1p-27;                   // 0x3E40000000000000
// pi/180 and 180/pi, pre-divided so a conversion is one multiply.
inline constexpr double RAD_PER_DEG = 0x1.1df46a2529d39p-6;  // 0x3F91DF46A2529D39
inline constexpr double DEG_PER_RAD = 0x1.ca5dc1a63c1f8p+5;  // 0x404CA5DC1A63C1F8

// Sine coefficients S[n] = (-1)^n/(2n+1)!, n = 1..9 (index 0 here is n = 1).
inline constexpr double S[9] = {
    -0x1.5555555555555p-3,   // -1/3!   0xBFC5555555555555
    0x1.1111111111111p-7,    // +1/5!   0x3F81111111111111
    -0x1.a01a01a01a01ap-13,  // -1/7!   0xBF2A01A01A01A01A
    0x1.71de3a556c734p-19,   // +1/9!   0x3EC71DE3A556C734
    -0x1.ae64567f544e4p-26,  // -1/11!  0xBE5AE64567F544E4
    0x1.6124613a86d09p-33,   // +1/13!  0x3DE6124613A86D09
    -0x1.ae7f3e733b81fp-41,  // -1/15!  0xBD6AE7F3E733B81F
    0x1.952c77030ad4ap-49,   // +1/17!  0x3CE952C77030AD4A
    -0x1.2f49b46814157p-57,  // -1/19!  0xBC62F49B46814157
};
// Cosine coefficients C[n] = (-1)^n/(2n)!, n = 2..9 (index 0 here is n = 2).
inline constexpr double C[8] = {
    0x1.5555555555555p-5,    // +1/4!   0x3FA5555555555555
    -0x1.6c16c16c16c17p-10,  // -1/6!   0xBF56C16C16C16C17
    0x1.a01a01a01a01ap-16,   // +1/8!   0x3EFA01A01A01A01A
    -0x1.27e4fb7789f5cp-22,  // -1/10!  0xBE927E4FB7789F5C
    0x1.1eed8eff8d898p-29,   // +1/12!  0x3E21EED8EFF8D898
    -0x1.93974a8c07c9dp-37,  // -1/14!  0xBDA93974A8C07C9D
    0x1.ae7f3e733b81fp-45,   // +1/16!  0x3D2AE7F3E733B81F
    -0x1.6827863b97d97p-53,  // -1/18!  0xBCA6827863B97D97
};

// |x| without std::fabs. (-0.0 stays -0.0, which is fine for the comparisons
// below; a NaN stays a NaN.)
inline double d_abs(double x) { return x < 0.0 ? -x : x; }

// ── Range reduction (PHY00a 3.4) ─────────────────────────────────────────────

// The quotient k and remainder r with x = k*(pi/2) + r, |r| <= pi/4 (about).
// Precondition: |x| <= 2^29, so k is a finite whole number below 2^29. Every
// product and the subtraction that uses it are SEPARATE statements: they must
// stay two roundings, never one fused multiply-add.
inline void reduce(double x, double &k, double &r) {
    CA_TRIG_NO_CONTRACT
    double t = x * INV_PIO2;  // x / (pi/2), approximately: one rounding
    double s = t + MAGIC;     // the fraction bits fall off the end here
    k = s - MAGIC;            // nearest integer, ties to even; exact
    r = x;
    double m = k * PIO2_1;    // product exact (24 + 29 bits); subtraction exact
    r = r - m;
    m = k * PIO2_2;           // product exact; subtraction exact
    r = r - m;
    m = k * PIO2_3;           // product exact; subtraction rounds
    r = r - m;
    m = k * PIO2_4;           // product exact; subtraction rounds
    r = r - m;
    m = k * PIO2_5;           // product rounds; subtraction rounds
    r = r - m;
}

// ── The kernels (PHY00a 3.5): Horner on the stored coefficients ──────────────

// sin(r) for |r| <= pi/4:  r + r*z*(S1 + S2 z + ... + S9 z^8).
inline double sin_k(double r) {
    CA_TRIG_NO_CONTRACT
    double z = r * r;
    double p = S[8];
    for (int n = 7; n >= 0; n--) {
        p = p * z;  // the product and the sum stay two statements
        p = p + S[n];
    }
    double w = r * z;
    w = w * p;
    return r + w;
}

// cos(r) for |r| <= pi/4:  (1 - z/2) + z*z*(C2 + C3 z + ... + C9 z^7).
inline double cos_k(double r) {
    CA_TRIG_NO_CONTRACT
    double z = r * r;
    double q = C[7];
    for (int n = 6; n >= 0; n--) {
        q = q * z;
        q = q + C[n];
    }
    double u = 0.5 * z;  // exact
    u = 1.0 - u;
    double v = z * z;
    v = v * q;
    return u + v;
}

// Square root via Newton's method, no domain check (callers guarantee x >= 0).
inline double sqrt_unchecked(double x) {
    CA_TRIG_NO_CONTRACT
    if (x == 0.0) return x;
    if (x > std::numeric_limits<double>::max()) return x;
    double scaled = x;
    double result_scale = 1.0;
    while (scaled < 0.25) {
        scaled *= 4.0;
        result_scale *= 0.5;
    }
    while (scaled >= 4.0) {
        scaled *= 0.25;
        result_scale *= 2.0;
    }
    double guess = scaled >= 1.0 ? scaled : 1.0;
    for (int i = 0; i < 60; i++) {
        double next = (guess + scaled / guess) / 2.0;
        if (d_abs(next - guess) < 1e-15 * guess + 1e-300) {
            return next * result_scale;
        }
        guess = next;
    }
    return guess * result_scale;
}

// Inner atan for |x| <= 1: half-angle reduction then the Taylor series.
inline double atan_core(double x) {
    CA_TRIG_NO_CONTRACT
    double xx = x * x;
    double reduced = x / (1.0 + sqrt_unchecked(1.0 + xx));
    double t = reduced;
    double t_sq = t * t;
    double term = t;
    double result = t;
    for (int n = 1; n <= 30; n++) {
        // Left to right: ((term * (-t_sq)) * (2n-1)) / (2n+1).
        term = term * (-t_sq) * static_cast<double>(2 * n - 1) /
               static_cast<double>(2 * n + 1);
        result += term;
        if (d_abs(term) < 1e-17) break;
    }
    return 2.0 * result;
}

}  // namespace detail

// ── sin / cos (PHY00a 3.6) ───────────────────────────────────────────────────
//
// The domain guard is a NEGATED `<=`: a NaN fails every comparison, so
// `!(|x| <= LIMIT)` is true for a NaN, for both infinities and for every finite
// value beyond 2^29, all in one test. The tempting `|x| > LIMIT` is wrong: it
// lets a NaN through to the integer cast, which is undefined behaviour.

inline double sin(double x) {
    CA_TRIG_NO_CONTRACT
    if (!(detail::d_abs(x) <= detail::LIMIT))
        return std::numeric_limits<double>::quiet_NaN();
    if (detail::d_abs(x) < detail::TINY) return x;  // keeps -0 and subnormals
    double k, r;
    detail::reduce(x, k, r);
    // k is a whole number below 2^29, so the cast is exact; `& 3` is k mod 4
    // (two's complement, so -1 & 3 == 3).
    switch (static_cast<long long>(k) & 3LL) {
        case 0: return detail::sin_k(r);
        case 1: return detail::cos_k(r);
        case 2: return -detail::sin_k(r);
        default: return -detail::cos_k(r);
    }
}

inline double cos(double x) {
    CA_TRIG_NO_CONTRACT
    if (!(detail::d_abs(x) <= detail::LIMIT))
        return std::numeric_limits<double>::quiet_NaN();
    if (detail::d_abs(x) < detail::TINY) return 1.0;
    double k, r;
    detail::reduce(x, k, r);
    switch (static_cast<long long>(k) & 3LL) {
        case 0: return detail::cos_k(r);
        case 1: return -detail::sin_k(r);
        case 2: return -detail::cos_k(r);
        default: return detail::sin_k(r);
    }
}

// ── angle conversion (PHY00a 3.7): one multiply by a bit-pinned constant ─────

inline double radians(double deg) { return deg * detail::RAD_PER_DEG; }
inline double degrees(double rad) { return rad * detail::DEG_PER_RAD; }

// ── sqrt — Newton's method; throws std::domain_error on x < 0 ────────────────

inline double sqrt(double x) {
    if (x < 0.0) throw std::domain_error("sqrt: input is negative");
    return detail::sqrt_unchecked(x);
}

// ── tan = sin / cos (PHY00a 3.8) ─────────────────────────────────────────────
//
// A pinned legacy contract, not an approximation: within 1e-15 of a pole
// (|cos| < 1e-15) the result is the sentinel +-1e308. For |x| > 2^29 both sin
// and cos are NaN, the sentinel test is false and NaN / NaN is NaN.

inline double tan(double x) {
    CA_TRIG_NO_CONTRACT
    double s = sin(x);
    double c = cos(x);
    if (detail::d_abs(c) < 1e-15) return s > 0.0 ? 1.0e308 : -1.0e308;
    return s / c;
}

// ── atan / atan2 (PHY00a 3.9) ────────────────────────────────────────────────

inline double atan(double x) {
    CA_TRIG_NO_CONTRACT
    // atan(x) rounds exactly to x here; avoid halving subnormals and retain -0.
    if (detail::d_abs(x) <= detail::TINY) return x;
    if (x > 1.0) return HALF_PI - detail::atan_core(1.0 / x);
    if (x < -1.0) return -HALF_PI - detail::atan_core(1.0 / x);
    return detail::atan_core(x);
}

inline double atan2(double y, double x) {
    CA_TRIG_NO_CONTRACT
    // A NaN fails every comparison below and would fall through to 0.0; a
    // finite answer to an undefined question is wrong (PHY00a 3.9).
    if (x != x || y != y) return std::numeric_limits<double>::quiet_NaN();
    if (x > 0.0) return atan(y / x);
    if (x < 0.0 && y >= 0.0) return atan(y / x) + PI;
    if (x < 0.0 && y < 0.0) return atan(y / x) - PI;
    if (x == 0.0 && y > 0.0) return HALF_PI;
    if (x == 0.0 && y < 0.0) return -HALF_PI;
    return 0.0;  // (0,0): undefined; 0 by convention
}

}  // namespace trig
}  // namespace ca

#if !defined(CA_TRIG_NO_FP_GUARDS)
#if defined(__clang__)
// nothing to restore: the clang pragma lives inside each function
#elif defined(__GNUC__)
#pragma GCC pop_options
#elif defined(_MSC_VER)
#pragma fp_contract(on)
#endif
#endif

#endif  // CA_TRIG_HPP
