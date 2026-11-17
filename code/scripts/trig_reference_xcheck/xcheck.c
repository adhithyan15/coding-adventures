/*
 * TRIG-REF-1 (PHY00a 3.3-3.9) in plain C, the first host cross-check of the
 * conformance-vector generator (PHY00a 6.4 step 2).  Reads lines on stdin:
 *
 *     <op> <16 hex digits>            sin cos tan radians degrees sqrt atan
 *     atan2 <16 hex>,<16 hex>         (y first, then x)
 *
 * and prints the result bits as 16 lowercase hex digits (any NaN is printed as
 * 7ff8000000000000; a sqrt of a negative prints "error").
 *
 * It is built with -ffp-contract=off and never calls libm: only + - * / and
 * comparisons, as the specification allows.  It shares no code with the
 * Python soft-float model or with xcheck.rs.
 */
#include <float.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#if FLT_EVAL_METHOD != 0
#error "TRIG-REF-1 needs FLT_EVAL_METHOD == 0 (no extended precision)"
#endif

static double F(uint64_t b) { double d; memcpy(&d, &b, 8); return d; }
static uint64_t B(double d) { uint64_t b; memcpy(&b, &d, 8); return b; }
static int isnan_(double x) { return x != x; }
static double fabs_(double x) { return B(x) & 0x8000000000000000ull ? -x : x; }

/* Every constant is built from its bit pattern (PHY00a 3.2 says the bits are
 * authoritative), so a mistyped hex-float or decimal literal cannot enter. */
static double INV_PIO2, PIO2_1, PIO2_2, PIO2_3, PIO2_4, PIO2_5, MAGIC, LIMIT, TINY;
static double RAD_PER_DEG, DEG_PER_RAD, PI, HALF_PI;

static double S[10], C[10];

static void init_coeffs(void) {
    INV_PIO2 = F(0x3FE45F306DC9C883ull);
    PIO2_1 = F(0x3FF921FB40000000ull);
    PIO2_2 = F(0x3E74442D00000000ull);
    PIO2_3 = F(0x3CF8469880000000ull);
    PIO2_4 = F(0x3B78CC5160000000ull);
    PIO2_5 = F(0x39F01B839A25204Aull);
    MAGIC = F(0x4338000000000000ull);
    LIMIT = F(0x41C0000000000000ull);
    TINY = F(0x3E40000000000000ull);
    RAD_PER_DEG = F(0x3F91DF46A2529D39ull);
    DEG_PER_RAD = F(0x404CA5DC1A63C1F8ull);
    PI = F(0x400921FB54442D18ull);
    HALF_PI = F(0x3FF921FB54442D18ull);
    static const uint64_t s[9] = {0xBFC5555555555555ull, 0x3F81111111111111ull,
        0xBF2A01A01A01A01Aull, 0x3EC71DE3A556C734ull, 0xBE5AE64567F544E4ull,
        0x3DE6124613A86D09ull, 0xBD6AE7F3E733B81Full, 0x3CE952C77030AD4Aull,
        0xBC62F49B46814157ull};
    static const uint64_t c[8] = {0x3FA5555555555555ull, 0xBF56C16C16C16C17ull,
        0x3EFA01A01A01A01Aull, 0xBE927E4FB7789F5Cull, 0x3E21EED8EFF8D898ull,
        0xBDA93974A8C07C9Dull, 0x3D2AE7F3E733B81Full, 0xBCA6827863B97D97ull};
    for (int n = 1; n <= 9; n++) S[n] = F(s[n - 1]);
    for (int n = 2; n <= 9; n++) C[n] = F(c[n - 2]);
}

static double sin_k(double r) {
    double z = r * r;
    double p = S[9];
    for (int n = 8; n >= 1; n--) { p = p * z; p = p + S[n]; }
    double w = r * z;
    w = w * p;
    return r + w;
}

static double cos_k(double r) {
    double z = r * r;
    double q = C[9];
    for (int n = 8; n >= 2; n--) { q = q * z; q = q + C[n]; }
    double u = 0.5 * z;
    u = 1.0 - u;
    double v = z * z;
    v = v * q;
    return u + v;
}

static void reduce(double x, double *kk, double *rr) {
    double t = x * INV_PIO2;
    double k = (t + MAGIC) - MAGIC;
    double m, r = x;
    m = k * PIO2_1; r = r - m;
    m = k * PIO2_2; r = r - m;
    m = k * PIO2_3; r = r - m;
    m = k * PIO2_4; r = r - m;
    m = k * PIO2_5; r = r - m;
    *kk = k; *rr = r;
}

static double nan_(void) { return F(0x7ff8000000000000ull); }

static double ref_sin(double x) {
    if (!(fabs_(x) <= LIMIT)) return nan_();
    if (fabs_(x) < TINY) return x;
    double k, r;
    reduce(x, &k, &r);
    int q = (int)k & 3;
    switch (q) {
    case 0: return sin_k(r);
    case 1: return cos_k(r);
    case 2: return -sin_k(r);
    default: return -cos_k(r);
    }
}

static double ref_cos(double x) {
    if (!(fabs_(x) <= LIMIT)) return nan_();
    if (fabs_(x) < TINY) return 1.0;
    double k, r;
    reduce(x, &k, &r);
    int q = (int)k & 3;
    switch (q) {
    case 0: return cos_k(r);
    case 1: return -sin_k(r);
    case 2: return -cos_k(r);
    default: return sin_k(r);
    }
}

static double ref_tan(double x) {
    double s = ref_sin(x), c = ref_cos(x);
    if (fabs_(c) < 1e-15) return s > 0 ? 1e308 : -1e308;
    return s / c;
}

static double ref_sqrt(double x, int *err) {
    *err = 0;
    if (x < 0) { *err = 1; return 0; }
    if (x == 0) return x;
    if (x == F(0x7ff0000000000000ull)) return x;
    double scaled = x, scale = 1.0;
    while (scaled < 0.25) { scaled = scaled * 4.0; scale = scale * 0.5; }
    while (scaled >= 4.0) { scaled = scaled * 0.25; scale = scale * 2.0; }
    double guess = scaled >= 1.0 ? scaled : 1.0;
    for (int i = 0; i < 60; i++) {
        double next = (guess + scaled / guess) / 2.0;
        if (fabs_(next - guess) < 1e-15 * guess + 1e-300) return next * scale;
        guess = next;
    }
    return guess * scale;
}

static double sq(double x) { int e; return ref_sqrt(x, &e); }

static double atan_core(double x) {
    double reduced = x / (1.0 + sq(1.0 + x * x));
    double t = reduced, tsq = t * t, term = t, result = t;
    for (int n = 1; n <= 30; n++) {
        term = term * (-tsq);
        term = term * (double)(2 * n - 1);
        term = term / (double)(2 * n + 1);
        result = result + term;
        if (fabs_(term) < 1e-17) break;
    }
    return 2.0 * result;
}

static double ref_atan(double x) {
    if (fabs_(x) <= TINY) return x;
    if (x > 1.0) return HALF_PI - atan_core(1.0 / x);
    if (x < -1.0) return -HALF_PI - atan_core(1.0 / x);
    return atan_core(x);
}

static double ref_atan2(double y, double x) {
    if (isnan_(x) || isnan_(y)) return nan_();
    if (x > 0) return ref_atan(y / x);
    if (x < 0 && y >= 0) return ref_atan(y / x) + PI;
    if (x < 0 && y < 0) return ref_atan(y / x) - PI;
    if (x == 0 && y > 0) return HALF_PI;
    if (x == 0 && y < 0) return -HALF_PI;
    return 0.0;
}

int main(void) {
    init_coeffs();
    char op[32], arg[64];
    while (scanf("%31s %63s", op, arg) == 2) {
        double r = 0;
        int err = 0;
        if (!strcmp(op, "atan2")) {
            uint64_t yb, xb;
            sscanf(arg, "%16llx,%16llx", (unsigned long long *)&yb, (unsigned long long *)&xb);
            r = ref_atan2(F(yb), F(xb));
        } else {
            unsigned long long xb;
            sscanf(arg, "%llx", &xb);
            double x = F(xb);
            if (!strcmp(op, "sin")) r = ref_sin(x);
            else if (!strcmp(op, "cos")) r = ref_cos(x);
            else if (!strcmp(op, "tan")) r = ref_tan(x);
            else if (!strcmp(op, "radians")) r = x * RAD_PER_DEG;
            else if (!strcmp(op, "degrees")) r = x * DEG_PER_RAD;
            else if (!strcmp(op, "sqrt")) r = ref_sqrt(x, &err);
            else if (!strcmp(op, "atan")) r = ref_atan(x);
            else { fprintf(stderr, "bad op %s\n", op); return 2; }
        }
        if (err) puts("error");
        else if (isnan_(r)) puts("7ff8000000000000");
        else printf("%016llx\n", (unsigned long long)B(r));
    }
    return 0;
}
