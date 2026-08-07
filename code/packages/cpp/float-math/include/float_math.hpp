// float_math.hpp — elementary floating-point functions, from scratch
// (header-only, ISO C++17).
// ---------------------------------------------------------------------------
//
// A from-first-principles replacement for the parts of `<cmath>` the campaign's
// ports need — WITHOUT linking libm. Every function is computed from nothing but
// +, -, *, /, comparisons, and IEEE-754 bit manipulation (via std::memcpy, so no
// type-punning UB). Nothing here calls the platform math library; a math-using
// C++ port depends on THIS header instead, keeping the pure-ISO lane
// self-contained and identical across GCC, Clang, and MSVC.
//
// Companion to the `trig` crate (sin/cos/tan/atan): this covers roots,
// exponentials, logarithms, powers, and hyperbolics. Accuracy target: solid
// double precision (~1 ULP), verified against the platform libm as a local,
// non-shipped oracle.
#ifndef CA_FLOAT_MATH_HPP
#define CA_FLOAT_MATH_HPP

#include <cstdint>
#include <cstring>

namespace ca {
namespace float_math {

inline constexpr double PI = 3.141592653589793238462643383279503;
inline constexpr double E = 2.718281828459045235360287471352662;
inline constexpr double LN2 = 0.693147180559945309417232121458177;
inline constexpr double LN10 = 2.302585092994045684017991454684364;
inline constexpr double LOG2E = 1.442695040888963407359924681001892;
inline constexpr double LOG10E = 0.434294481903251827651128918916605;
inline constexpr double SQRT2 = 1.414213562373095048801688724209698;

namespace detail {
// Two-part ln2 for accurate argument reduction (the fdlibm split).
inline constexpr double LN2HI = 6.93147180369123816490e-01;
inline constexpr double LN2LO = 1.90821492927058770002e-10;

inline std::uint64_t bits_of(double x) {
    std::uint64_t b;
    std::memcpy(&b, &x, sizeof b);
    return b;
}
inline double double_of(std::uint64_t b) {
    double x;
    std::memcpy(&x, &b, sizeof x);
    return x;
}
} // namespace detail

// ── classification ─────────────────────────────────────────────────
inline bool isnan(double x) { return x != x; }
inline int isinf(double x) {
    std::uint64_t b = detail::bits_of(x);
    if (((b >> 52) & 0x7FFu) == 0x7FFu && (b & 0xFFFFFFFFFFFFFu) == 0) {
        return (b >> 63) ? -1 : 1;
    }
    return 0;
}
inline bool isfinite(double x) { return ((detail::bits_of(x) >> 52) & 0x7FFu) != 0x7FFu; }
inline double inf() { return detail::double_of(static_cast<std::uint64_t>(0x7FF0000000000000u)); }
inline double nan() { return detail::double_of(static_cast<std::uint64_t>(0x7FF8000000000000u)); }

// ── sign / rounding / remainder ────────────────────────────────────
inline double fabs(double x) { return detail::double_of(detail::bits_of(x) & 0x7FFFFFFFFFFFFFFFu); }
inline double copysign(double mag, double sgn) {
    std::uint64_t bm = detail::bits_of(mag) & 0x7FFFFFFFFFFFFFFFu;
    std::uint64_t bs = detail::bits_of(sgn) & 0x8000000000000000u;
    return detail::double_of(bm | bs);
}
inline double trunc(double x) {
    if (!isfinite(x)) return x;
    if (fabs(x) >= 4503599627370496.0) return x; // >= 2^52 is already integral
    return static_cast<double>(static_cast<long long>(x));
}
inline double floor(double x) {
    double t = trunc(x);
    return (t > x) ? t - 1.0 : t;
}
inline double ceil(double x) {
    double t = trunc(x);
    return (t < x) ? t + 1.0 : t;
}
inline double round(double x) {
    if (!isfinite(x)) return x;
    double t = trunc(x);
    double f = x - t;
    if (f >= 0.5) return t + 1.0;
    if (f <= -0.5) return t - 1.0;
    return t;
}

namespace detail {
inline double two_pow(int n) { return double_of(static_cast<std::uint64_t>(n + 1023) << 52); }
} // namespace detail

inline double ldexp(double x, int n) {
    if (x == 0.0 || !isfinite(x)) return x;
    while (n > 512) {
        x *= detail::two_pow(512);
        n -= 512;
        if (isinf(x)) return x;
    }
    while (n < -512) {
        x *= detail::two_pow(-512);
        n += 512;
        if (x == 0.0) return x;
    }
    return x * detail::two_pow(n);
}
inline double frexp(double x, int* exp) {
    if (x == 0.0 || !isfinite(x)) {
        *exp = 0;
        return x;
    }
    std::uint64_t b = detail::bits_of(x);
    int e = static_cast<int>((b >> 52) & 0x7FFu);
    if (e == 0) {
        x *= 18014398509481984.0; // 2^54
        b = detail::bits_of(x);
        e = static_cast<int>((b >> 52) & 0x7FFu) - 54;
    }
    *exp = e - 1022;
    b = (b & ~(static_cast<std::uint64_t>(0x7FF) << 52)) | (static_cast<std::uint64_t>(1022) << 52);
    return detail::double_of(b);
}
inline double fmod(double x, double y) {
    if (isnan(x) || isnan(y) || isinf(x) || y == 0.0) return nan();
    if (isinf(y)) return x;
    double ax = fabs(x), ay = fabs(y);
    if (ax < ay) return x;
    int ex, ey;
    (void)frexp(ax, &ex);
    (void)frexp(ay, &ey);
    double d = ldexp(ay, ex - ey);
    for (int i = ex - ey; i >= 0; --i) {
        if (ax >= d) ax -= d;
        d *= 0.5;
    }
    return copysign(ax, x);
}

// ── roots ──────────────────────────────────────────────────────────
inline double sqrt(double x) {
    if (isnan(x)) return x;
    if (x < 0.0) return nan();
    if (x == 0.0 || isinf(x) == 1) return x;
    int e;
    double f = frexp(x, &e);
    if (e & 1) {
        f *= 2.0;
        e -= 1;
    }
    double y = ldexp(0.5 + 0.5 * f, e / 2);
    for (int i = 0; i < 6; ++i) y = 0.5 * (y + x / y);
    return y;
}
inline double cbrt(double x) {
    if (x == 0.0 || isnan(x) || isinf(x)) return x;
    bool sign = (x < 0.0);
    double ax = fabs(x);
    int e;
    double f = frexp(ax, &e);
    int q = e / 3;
    int r = e - 3 * q;
    if (r < 0) {
        r += 3;
        q -= 1;
    }
    double m = ldexp(f, r);
    double y = 1.0;
    for (int i = 0; i < 8; ++i) y = (2.0 * y + m / (y * y)) / 3.0;
    double res = ldexp(y, q);
    return sign ? -res : res;
}
inline double hypot(double x, double y) {
    if (isinf(x) || isinf(y)) return inf();
    x = fabs(x);
    y = fabs(y);
    if (x < y) {
        double t = x;
        x = y;
        y = t;
    }
    if (x == 0.0) return 0.0;
    double r = y / x;
    return x * sqrt(1.0 + r * r);
}

// ── exp / log ──────────────────────────────────────────────────────
inline double exp(double x) {
    if (isnan(x)) return x;
    if (x > 709.782712893384) return inf();
    if (x < -745.133219101941) return 0.0;
    double kd = round(x * LOG2E);
    int k = static_cast<int>(kd);
    double r = (x - kd * detail::LN2HI) - kd * detail::LN2LO;
    double p = 1.0;
    for (int i = 14; i >= 1; --i) p = 1.0 + r * p / static_cast<double>(i);
    return ldexp(p, k);
}
inline double expm1(double x) {
    if (isnan(x)) return x;
    if (fabs(x) < 0.5) {
        double p = 1.0;
        for (int i = 14; i >= 2; --i) p = 1.0 + x * p / static_cast<double>(i);
        return x * p;
    }
    return exp(x) - 1.0;
}
inline double log(double x) {
    if (isnan(x)) return x;
    if (x < 0.0) return nan();
    if (x == 0.0) return -inf();
    if (isinf(x) == 1) return x;
    int e;
    double f = frexp(x, &e);
    if (f < 0.7071067811865476) {
        f *= 2.0;
        e -= 1;
    }
    double s = (f - 1.0) / (f + 1.0);
    double z = s * s;
    double poly = 0.0;
    for (int k = 13; k >= 0; --k) poly = poly * z + 1.0 / (2.0 * static_cast<double>(k) + 1.0);
    double ed = static_cast<double>(e);
    return (ed * detail::LN2HI + 2.0 * s * poly) + ed * detail::LN2LO;
}
inline double log2(double x) { return log(x) * LOG2E; }
inline double log10(double x) { return log(x) * LOG10E; }
inline double log_base(double x, double base) { return log(x) / log(base); }

// ── power ──────────────────────────────────────────────────────────
inline double pow(double x, double y) {
    if (isnan(x) || isnan(y)) return (y == 0.0) ? 1.0 : nan();
    if (y == 0.0 || x == 1.0) return 1.0;
    double ry = round(y);
    bool y_is_int = (ry == y) && (fabs(y) <= 1e18);
    if (x == 0.0) return (y > 0.0) ? 0.0 : inf();
    if (x < 0.0) {
        if (!y_is_int) return nan();
        double mag = pow(-x, y);
        long long iy = static_cast<long long>(ry);
        return (iy & 1) ? -mag : mag;
    }
    if (y_is_int && fabs(y) <= 64.0) {
        long long n = static_cast<long long>(ry);
        bool neg = (n < 0);
        unsigned long long m = neg ? static_cast<unsigned long long>(-n)
                                   : static_cast<unsigned long long>(n);
        double base = x, acc = 1.0;
        while (m != 0) {
            if (m & 1u) acc *= base;
            base *= base;
            m >>= 1;
        }
        return neg ? 1.0 / acc : acc;
    }
    return exp(y * log(x));
}

// ── hyperbolics ────────────────────────────────────────────────────
inline double sinh(double x) {
    if (isnan(x) || isinf(x)) return x;
    if (fabs(x) < 1.0) return 0.5 * (expm1(x) - expm1(-x));
    double ex = exp(x);
    return 0.5 * (ex - 1.0 / ex);
}
inline double cosh(double x) {
    if (isnan(x)) return x;
    if (isinf(x)) return inf();
    double ex = exp(fabs(x));
    return 0.5 * (ex + 1.0 / ex);
}
inline double tanh(double x) {
    if (isnan(x)) return x;
    if (x == 0.0) return x;
    double ax = fabs(x);
    if (ax > 20.0) return (x > 0.0) ? 1.0 : -1.0;
    double m = expm1(2.0 * ax);
    double t = m / (m + 2.0);
    return (x < 0.0) ? -t : t;
}

} // namespace float_math
} // namespace ca

#endif // CA_FLOAT_MATH_HPP
