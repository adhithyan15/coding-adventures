/*
 * float_math.c — from-first-principles elementary functions (no <math.h>).
 *
 * Every routine uses only +, -, *, /, comparisons, and IEEE-754 bit access via
 * memcpy (which is well-defined, unlike a union or a pointer cast). The design
 * mirrors the classic reduce → approximate → reconstruct shape: bring the
 * argument into a small range where a short series or a few Newton steps reach
 * full double precision, then scale the result back with an exact power of two.
 */
#include "float_math.h"

#include <stdint.h>
#include <string.h>

/* Two-part natural log of 2 (ln2 = LN2HI + LN2LO to full precision), used for
 * accurate argument reduction in exp/log (the fdlibm split). */
static const double LN2HI = 6.93147180369123816490e-01;
static const double LN2LO = 1.90821492927058770002e-10;

/* ================================================================== */
/* Bit access helpers                                                 */
/* ================================================================== */

static uint64_t bits_of(double x) {
    uint64_t b;
    memcpy(&b, &x, sizeof b);
    return b;
}
static double double_of(uint64_t b) {
    double x;
    memcpy(&x, &b, sizeof x);
    return x;
}

/* ================================================================== */
/* Classification                                                     */
/* ================================================================== */

int fm_isnan(double x) { return x != x; }

int fm_isinf(double x) {
    uint64_t b = bits_of(x);
    if (((b >> 52) & 0x7FFu) == 0x7FFu && (b & 0xFFFFFFFFFFFFFu) == 0) {
        return (b >> 63) ? -1 : 1;
    }
    return 0;
}

int fm_isfinite(double x) { return ((bits_of(x) >> 52) & 0x7FFu) != 0x7FFu; }

double fm_inf(void) { return double_of((uint64_t)0x7FF0000000000000u); }
double fm_nan(void) { return double_of((uint64_t)0x7FF8000000000000u); }

/* ================================================================== */
/* Sign / rounding / remainder                                        */
/* ================================================================== */

double fm_fabs(double x) { return double_of(bits_of(x) & 0x7FFFFFFFFFFFFFFFu); }

double fm_copysign(double mag, double sgn) {
    uint64_t bm = bits_of(mag) & 0x7FFFFFFFFFFFFFFFu;
    uint64_t bs = bits_of(sgn) & 0x8000000000000000u;
    return double_of(bm | bs);
}

double fm_trunc(double x) {
    if (!fm_isfinite(x)) {
        return x;
    }
    /* |x| >= 2^52 has no fractional bits — already integral. Below that it fits
     * in a signed 64-bit integer, whose conversion truncates toward zero. */
    if (fm_fabs(x) >= 4503599627370496.0) {
        return x;
    }
    return (double)(long long)x;
}

double fm_floor(double x) {
    double t = fm_trunc(x);
    return (t > x) ? t - 1.0 : t;
}

double fm_ceil(double x) {
    double t = fm_trunc(x);
    return (t < x) ? t + 1.0 : t;
}

double fm_round(double x) {
    double t;
    double f;
    if (!fm_isfinite(x)) {
        return x;
    }
    t = fm_trunc(x);
    f = x - t;
    if (f >= 0.5) {
        return t + 1.0;
    }
    if (f <= -0.5) {
        return t - 1.0;
    }
    return t;
}

/* 2^n for n in [-512, 512], constructed exactly (always a normal double). */
static double two_pow(int n) {
    return double_of((uint64_t)(n + 1023) << 52);
}

double fm_ldexp(double x, int n) {
    if (x == 0.0 || !fm_isfinite(x)) {
        return x;
    }
    while (n > 512) {
        x *= two_pow(512);
        n -= 512;
        if (fm_isinf(x)) {
            return x;
        }
    }
    while (n < -512) {
        x *= two_pow(-512);
        n += 512;
        if (x == 0.0) {
            return x;
        }
    }
    return x * two_pow(n);
}

double fm_frexp(double x, int *exp) {
    uint64_t b;
    int e;
    if (x == 0.0 || !fm_isfinite(x)) {
        *exp = 0;
        return x;
    }
    b = bits_of(x);
    e = (int)((b >> 52) & 0x7FFu);
    if (e == 0) {
        /* Subnormal: scale up by 2^54 to normalise, then correct the exponent. */
        x *= 18014398509481984.0; /* 2^54 */
        b = bits_of(x);
        e = (int)((b >> 52) & 0x7FFu) - 54;
    }
    *exp = e - 1022;
    /* Force the biased exponent to 1022 so the fraction lands in [0.5, 1). */
    b = (b & ~((uint64_t)0x7FF << 52)) | ((uint64_t)1022 << 52);
    return double_of(b);
}

double fm_fmod(double x, double y) {
    double ax;
    double ay;
    int ex;
    int ey;
    double d;
    int i;
    if (fm_isnan(x) || fm_isnan(y) || fm_isinf(x) || y == 0.0) {
        return fm_nan();
    }
    if (fm_isinf(y)) {
        return x;
    }
    ax = fm_fabs(x);
    ay = fm_fabs(y);
    if (ax < ay) {
        return x;
    }
    (void)fm_frexp(ax, &ex);
    (void)fm_frexp(ay, &ey);
    /* Subtract ay*2^i for i = (ex-ey) down to 0; each scaling is exact, so the
     * running remainder stays exact and ends in [0, ay). */
    d = fm_ldexp(ay, ex - ey);
    for (i = ex - ey; i >= 0; --i) {
        if (ax >= d) {
            ax -= d;
        }
        d *= 0.5;
    }
    return fm_copysign(ax, x);
}

/* ================================================================== */
/* Roots                                                              */
/* ================================================================== */

double fm_sqrt(double x) {
    int e;
    double f;
    double y;
    int i;
    if (fm_isnan(x)) {
        return x;
    }
    if (x < 0.0) {
        return fm_nan();
    }
    if (x == 0.0 || fm_isinf(x) == 1) {
        return x;
    }
    f = fm_frexp(x, &e); /* x = f * 2^e, f in [0.5, 1) */
    if (e & 1) {
        f *= 2.0; /* make e even so 2^(e/2) is exact */
        e -= 1;
    }
    /* Linear seed for sqrt(f), f in [0.5, 2): within ~6% of the true root. */
    y = fm_ldexp(0.5 + 0.5 * f, e / 2);
    for (i = 0; i < 6; ++i) {
        y = 0.5 * (y + x / y); /* Newton: quadratic convergence */
    }
    return y;
}

double fm_cbrt(double x) {
    int sign;
    double ax;
    int e;
    double f;
    int q;
    int r;
    double m;
    double y;
    double res;
    int i;
    if (x == 0.0 || fm_isnan(x) || fm_isinf(x)) {
        return x;
    }
    sign = (x < 0.0);
    ax = fm_fabs(x);
    f = fm_frexp(ax, &e); /* ax = f * 2^e */
    q = e / 3;
    r = e - 3 * q; /* r in {-2..2}; normalise to [0,2] */
    if (r < 0) {
        r += 3;
        q -= 1;
    }
    m = fm_ldexp(f, r); /* m in [0.5, 4) */
    y = 1.0;
    for (i = 0; i < 8; ++i) {
        y = (2.0 * y + m / (y * y)) / 3.0; /* Newton for cube root */
    }
    res = fm_ldexp(y, q);
    return sign ? -res : res;
}

double fm_hypot(double x, double y) {
    double r;
    if (fm_isinf(x) || fm_isinf(y)) {
        return fm_inf();
    }
    x = fm_fabs(x);
    y = fm_fabs(y);
    if (x < y) {
        double t = x;
        x = y;
        y = t;
    }
    if (x == 0.0) {
        return 0.0;
    }
    r = y / x; /* in [0,1]; scaling by the larger magnitude avoids overflow */
    return x * fm_sqrt(1.0 + r * r);
}

/* ================================================================== */
/* Exponentials / logarithms                                          */
/* ================================================================== */

double fm_exp(double x) {
    double kd;
    int k;
    double r;
    double p;
    int i;
    if (fm_isnan(x)) {
        return x;
    }
    if (x > 709.782712893384) {
        return fm_inf(); /* overflows double */
    }
    if (x < -745.133219101941) {
        return 0.0; /* underflows to zero */
    }
    /* Reduce x = k*ln2 + r with |r| <= ln2/2, using the two-part ln2. */
    kd = fm_round(x * FM_LOG2E);
    k = (int)kd;
    r = (x - kd * LN2HI) - kd * LN2LO;
    /* exp(r) by its Taylor series, nested so each step is one add + one divide.
     * |r| <= 0.347, so 14 terms reach full double precision. */
    p = 1.0;
    for (i = 14; i >= 1; --i) {
        p = 1.0 + r * p / (double)i;
    }
    return fm_ldexp(p, k);
}

double fm_expm1(double x) {
    double p;
    int i;
    if (fm_isnan(x)) {
        return x;
    }
    /* Near 0, exp(x)-1 loses precision to cancellation; sum the series directly:
     *   expm1(x) = x*(1 + x/2*(1 + x/3*(...))). */
    if (fm_fabs(x) < 0.5) {
        p = 1.0;
        for (i = 14; i >= 2; --i) {
            p = 1.0 + x * p / (double)i;
        }
        return x * p;
    }
    return fm_exp(x) - 1.0;
}

double fm_log(double x) {
    int e;
    double f;
    double s;
    double z;
    double poly;
    double ed;
    int k;
    if (fm_isnan(x)) {
        return x;
    }
    if (x < 0.0) {
        return fm_nan();
    }
    if (x == 0.0) {
        return -fm_inf();
    }
    if (fm_isinf(x) == 1) {
        return x;
    }
    f = fm_frexp(x, &e); /* x = f * 2^e, f in [0.5, 1) */
    /* Shift the mantissa into [sqrt(1/2), sqrt(2)) so s = (f-1)/(f+1) is small. */
    if (f < 0.7071067811865476) {
        f *= 2.0;
        e -= 1;
    }
    s = (f - 1.0) / (f + 1.0); /* |s| <= 0.1716 */
    z = s * s;
    /* ln(f) = 2*(s + s^3/3 + s^5/5 + ...) = 2s * sum_k z^k/(2k+1). */
    poly = 0.0;
    for (k = 13; k >= 0; --k) {
        poly = poly * z + 1.0 / (2.0 * (double)k + 1.0);
    }
    ed = (double)e;
    /* ln(x) = e*ln2 + 2s*poly, keeping the two-part ln2 for accuracy. */
    return (ed * LN2HI + 2.0 * s * poly) + ed * LN2LO;
}

double fm_log2(double x) { return fm_log(x) * FM_LOG2E; }
double fm_log10(double x) { return fm_log(x) * FM_LOG10E; }

double fm_log_base(double x, double base) { return fm_log(x) / fm_log(base); }

/* ================================================================== */
/* Power                                                              */
/* ================================================================== */

double fm_pow(double x, double y) {
    double ry;
    int y_is_int;
    if (fm_isnan(x) || fm_isnan(y)) {
        return (y == 0.0) ? 1.0 : fm_nan();
    }
    if (y == 0.0 || x == 1.0) {
        return 1.0;
    }
    ry = fm_round(y);
    y_is_int = (ry == y) && (fm_fabs(y) <= 1e18);
    if (x == 0.0) {
        return (y > 0.0) ? 0.0 : fm_inf();
    }
    if (x < 0.0) {
        double mag;
        long long iy;
        if (!y_is_int) {
            return fm_nan(); /* a negative base to a fractional power is undefined */
        }
        mag = fm_pow(-x, y);
        iy = (long long)ry;
        return (iy & 1) ? -mag : mag;
    }
    /* x > 0. Small integer exponents by squaring — more accurate than exp/log. */
    if (y_is_int && fm_fabs(y) <= 64.0) {
        long long n = (long long)ry;
        int neg = (n < 0);
        unsigned long long m = neg ? (unsigned long long)(-n) : (unsigned long long)n;
        double base = x;
        double acc = 1.0;
        while (m != 0) {
            if (m & 1u) {
                acc *= base;
            }
            base *= base;
            m >>= 1;
        }
        return neg ? 1.0 / acc : acc;
    }
    return fm_exp(y * fm_log(x));
}

/* ================================================================== */
/* Hyperbolics                                                        */
/* ================================================================== */

double fm_sinh(double x) {
    if (fm_isnan(x) || fm_isinf(x)) {
        return x;
    }
    /* Near 0, (e^x - e^-x)/2 cancels; expm1 keeps it accurate:
     *   sinh(x) = (expm1(x) - expm1(-x)) / 2. */
    if (fm_fabs(x) < 1.0) {
        return 0.5 * (fm_expm1(x) - fm_expm1(-x));
    }
    {
        double ex = fm_exp(x);
        return 0.5 * (ex - 1.0 / ex);
    }
}

double fm_cosh(double x) {
    double ex;
    if (fm_isnan(x)) {
        return x;
    }
    if (fm_isinf(x)) {
        return fm_inf();
    }
    ex = fm_exp(fm_fabs(x));
    return 0.5 * (ex + 1.0 / ex);
}

double fm_tanh(double x) {
    double ax;
    double m;
    double t;
    if (fm_isnan(x)) {
        return x;
    }
    if (x == 0.0) {
        return x;
    }
    ax = fm_fabs(x);
    if (ax > 20.0) {
        return (x > 0.0) ? 1.0 : -1.0; /* saturates to +/-1 within double precision */
    }
    /* tanh(x) = (e^2x - 1)/(e^2x + 1) = expm1(2x)/(expm1(2x) + 2). Using expm1
     * avoids the catastrophic cancellation of (e^2x - 1) for small x. */
    m = fm_expm1(2.0 * ax);
    t = m / (m + 2.0);
    return (x < 0.0) ? -t : t;
}
