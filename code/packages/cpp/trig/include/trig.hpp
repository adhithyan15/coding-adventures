// trig.hpp — trigonometric functions from first principles, header-only in pure
// ISO C++17 (namespace ca::trig). A faithful port of the Rust `trig` crate.
// ===========================================================================
//
// Every value is computed from BASIC ARITHMETIC — no <cmath>, no libm, no
// std::sin / std::sqrt. The point of the crate is to show *how* these functions
// are computed:
//
//   - sin / cos      Maclaurin (Taylor-at-zero) series, after reducing the
//                    argument into [-PI, PI]:  sin(x) = x - x^3/3! + x^5/5! - …
//   - sqrt           Newton's (Babylonian) method (quadratic convergence).
//   - tan            sin(x) / cos(x), guarding the cos(x)=0 poles.
//   - atan / atan2   Taylor series with two layers of range reduction.
//   - radians/degrees  the linear conversions (PI/180, 180/PI).
//
// DIVERGENCE FROM RUST. Rust's `sqrt` panics on a negative input; this port
// throws std::domain_error, the idiomatic C++ equivalent. Every other function
// is total and returns a double, exactly like the Rust crate.
//
// PORTABILITY. Pure ISO C++17 — no <cmath>, no compiler extensions. Compiles
// clean under GCC, Clang, and MSVC with -pedantic-errors / /permissive- and
// warnings-as-errors.
#ifndef CA_TRIG_HPP
#define CA_TRIG_HPP

#include <stdexcept>
#include <limits>

namespace ca {
namespace trig {

// The ratio of a circle's circumference to its diameter — hand-written to f64
// precision so the library is fully self-contained.
inline constexpr double PI = 3.141592653589793;
inline constexpr double TWO_PI = 2.0 * PI;
inline constexpr double HALF_PI = PI / 2.0;

namespace detail {

// |x| without std::fabs. (-0.0 stays -0.0, which is fine for the threshold
// comparisons below.)
inline double d_abs(double x) { return x < 0.0 ? -x : x; }

// Truncate toward zero without std::trunc. A double with |q| >= 2^53 has no
// fractional bits (already integral); below that it fits in a long long, whose
// float->int conversion truncates toward zero per the standard.
inline double d_trunc(double q) {
    constexpr double two53 = 9007199254740992.0;  // 2^53
    // Pass through anything NOT strictly inside (-2^53, 2^53). The negated
    // in-range test (vs. `q >= two53 || q <= -two53`) also catches NaN, which
    // fails every comparison — otherwise NaN would fall through to the cast,
    // and converting NaN to an integer is UB.
    if (!(q > -two53 && q < two53)) return q;
    return static_cast<double>(static_cast<long long>(q));
}

// Floating-point remainder x mod m (matches Rust's `%` on f64): result has the
// sign of x and magnitude < |m|.
inline double d_fmod(double x, double m) { return x - m * d_trunc(x / m); }

// Square root via Newton's method, no domain check (callers guarantee x >= 0).
inline double sqrt_unchecked(double x) {
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

// Reduce x into [-PI, PI], preserving any 2*PI-periodic function's value.
inline double range_reduce(double x) {
    double r = d_fmod(x, TWO_PI);
    if (r > PI) r -= TWO_PI;
    if (r < -PI) r += TWO_PI;
    return r;
}

// Inner atan for |x| <= 1: half-angle reduction then the Taylor series.
inline double atan_core(double x) {
    double reduced = x / (1.0 + sqrt_unchecked(1.0 + x * x));
    double t = reduced;
    double t_sq = t * t;
    double term = t;
    double result = t;
    for (int n = 1; n <= 30; n++) {
        term = term * (-t_sq) * static_cast<double>(2 * n - 1) /
               static_cast<double>(2 * n + 1);
        result += term;
        if (d_abs(term) < 1e-17) break;
    }
    return 2.0 * result;
}

}  // namespace detail

// ── sin / cos — Maclaurin series ─────────────────────────────────────────────

inline double sin(double x) {
    double rx = detail::range_reduce(x);
    double x_squared = rx * rx;
    double term = rx;
    double sum = term;
    for (int k = 1; k < 20; k++) {
        double denom =
            static_cast<double>(2 * k) * static_cast<double>(2 * k + 1);
        term *= -x_squared / denom;
        sum += term;
    }
    return sum;
}

inline double cos(double x) {
    double rx = detail::range_reduce(x);
    double x_squared = rx * rx;
    double term = 1.0;
    double sum = term;
    for (int k = 1; k < 20; k++) {
        double denom =
            static_cast<double>(2 * k - 1) * static_cast<double>(2 * k);
        term *= -x_squared / denom;
        sum += term;
    }
    return sum;
}

// ── angle conversion ─────────────────────────────────────────────────────────

inline double radians(double deg) { return deg * (PI / 180.0); }
inline double degrees(double rad) { return rad * (180.0 / PI); }

// ── sqrt — Newton's method; throws std::domain_error on x < 0 ────────────────

inline double sqrt(double x) {
    if (x < 0.0) throw std::domain_error("sqrt: input is negative");
    return detail::sqrt_unchecked(x);
}

// ── tan = sin / cos ──────────────────────────────────────────────────────────

inline double tan(double x) {
    double s = sin(x);
    double c = cos(x);
    if (detail::d_abs(c) < 1e-15) return s > 0.0 ? 1.0e308 : -1.0e308;
    return s / c;
}

// ── atan / atan2 ─────────────────────────────────────────────────────────────

inline double atan(double x) {
    // atan(x) rounds exactly to x here; avoid halving subnormals and retain -0.
    if (detail::d_abs(x) <= 0x1p-27) return x;
    if (x > 1.0) return HALF_PI - detail::atan_core(1.0 / x);
    if (x < -1.0) return -HALF_PI - detail::atan_core(1.0 / x);
    return detail::atan_core(x);
}

inline double atan2(double y, double x) {
    if (x > 0.0) return atan(y / x);
    if (x < 0.0 && y >= 0.0) return atan(y / x) + PI;
    if (x < 0.0 && y < 0.0) return atan(y / x) - PI;
    if (x == 0.0 && y > 0.0) return HALF_PI;
    if (x == 0.0 && y < 0.0) return -HALF_PI;
    return 0.0;  // (0,0): undefined; 0 by convention
}

}  // namespace trig
}  // namespace ca

#endif  // CA_TRIG_HPP
