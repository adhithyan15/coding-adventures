/*
 * image_raw_pipeline.c — implementation of the pure-ISO C RAW colour pipeline.
 * ==========================================================================
 *
 * The only transcendental the crate needs is a fractional power (the sRGB
 * gamma exponent 1/2.4 and its inverse 2.4). The pure-ISO build links no libm,
 * so `pow` is built from a from-scratch `exp`/`ln` here; it reproduces the Rust
 * f64 `powf` to ~1e-12 relative, comfortably inside the round-trip tolerances.
 */
#include "image_raw_pipeline.h"

#include <stdlib.h> /* malloc, free */

/* ── From-scratch floating-point helpers (no libm) ────────────────────────*/

static double d_abs(double x) { return x < 0.0 ? -x : x; }

static double pow2i(int k) {
    double result = 1.0;
    double base = k < 0 ? 0.5 : 2.0;
    int n = k < 0 ? -k : k;
    while (n > 0) {
        if (n & 1) result *= base;
        base *= base;
        n >>= 1;
    }
    return result;
}

/* e^x via Cody-Waite range reduction: x = k*ln2 + r, e^x = 2^k * e^r. Guards
 * precede the (int) cast so an out-of-range argument cannot overflow it. */
static double d_exp(double x) {
    if (x != x) return x;
    if (x == 0.0) return 1.0;
    if (x > 709.782712893384) return 1.7976931348623157e308;
    if (x < -745.13321910194) return 0.0;
    const double INV_LN2 = 1.4426950408889634;
    const double C1 = 0.693359375;
    const double C2 = -2.1219444005469058277e-4;
    double kf = x * INV_LN2;
    int k = (int)(kf >= 0.0 ? kf + 0.5 : kf - 0.5);
    double r = (x - (double)k * C1) - (double)k * C2;
    double term = 1.0, sum = 1.0;
    for (int i = 1; i <= 17; i++) {
        term *= r / (double)i;
        sum += term;
    }
    return sum * pow2i(k);
}

/* ln(x): reduce x = m*2^e with m in [1,2), then ln = e*ln2 + 2*atanh(u),
 * u = (m-1)/(m+1). Top guards keep the reduction loops from spinning on
 * non-finite / non-positive input. */
static double d_ln(double x) {
    if (x != x) return x;
    if (x <= 0.0) return -1.7976931348623157e308;
    if (x > 1.7976931348623157e308) return 1.7976931348623157e308;
    int e = 0;
    double m = x;
    while (m < 1.0) { m *= 2.0; e--; }
    while (m >= 2.0) { m *= 0.5; e++; }
    double u = (m - 1.0) / (m + 1.0);
    double u2 = u * u;
    double term = u, sum = u;
    for (int n = 1; n <= 60; n++) {
        term *= u2;
        double add = term / (double)(2 * n + 1);
        sum += add;
        if (d_abs(add) < 1e-17) break;
    }
    const double LN2 = 0.6931471805599453;
    return (double)e * LN2 + 2.0 * sum;
}

/* x^y for x > 0 (the only case the sRGB transfer functions ever hit). */
static double d_pow_pos(double x, double y) { return d_exp(y * d_ln(x)); }

/* ── sRGB transfer functions ──────────────────────────────────────────────*/

double irp_srgb_gamma(double linear) {
    if (linear <= 0.0031308) return 12.92 * linear;
    return 1.055 * d_pow_pos(linear, 1.0 / 2.4) - 0.055;
}

double irp_srgb_decode(double encoded) {
    if (encoded <= 0.04045) return encoded / 12.92;
    return d_pow_pos((encoded + 0.055) / 1.055, 2.4);
}

/* ── 3x3 matrix ops ───────────────────────────────────────────────────────*/

void irp_mat3x3_mul(const double m[3][3], const double v[3], double out[3]) {
    out[0] = m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2];
    out[1] = m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2];
    out[2] = m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2];
}

int irp_invert_3x3(const double m[3][3], double out[3][3]) {
    /* Unpack (mirrors the Cramer's-rule derivation). */
    double a = m[0][0], b = m[0][1], c = m[0][2];
    double d = m[1][0], e = m[1][1], f = m[1][2];
    double g = m[2][0], h = m[2][1], k = m[2][2];

    /* 2x2 minors. */
    double ek_fh = e * k - f * h;
    double dk_fg = d * k - f * g;
    double dh_eg = d * h - e * g;
    double bk_ch = b * k - c * h;
    double ak_cg = a * k - c * g;
    double ah_bg = a * h - b * g;
    double bf_ce = b * f - c * e;
    double af_cd = a * f - c * d;
    double ae_bd = a * e - b * d;

    double det = a * ek_fh - b * dk_fg + c * dh_eg;
    if (d_abs(det) < 1e-12) return 0; /* singular / near-singular */

    double inv_det = 1.0 / det;

    /* inv = (1/det) * cofactor^T. */
    out[0][0] = ek_fh * inv_det;
    out[0][1] = -bk_ch * inv_det;
    out[0][2] = bf_ce * inv_det;
    out[1][0] = -dk_fg * inv_det;
    out[1][1] = ak_cg * inv_det;
    out[1][2] = -af_cd * inv_det;
    out[2][0] = dh_eg * inv_det;
    out[2][1] = -ah_bg * inv_det;
    out[2][2] = ae_bd * inv_det;
    return 1;
}

/* ── Full pipeline ────────────────────────────────────────────────────────*/

/* u32 saturating subtract (clamps at 0 instead of wrapping). */
static uint32_t sat_sub_u32(uint32_t a, uint32_t b) { return a > b ? a - b : 0; }

static double clamp01(double v) {
    if (v < 0.0) return 0.0;
    if (v > 1.0) return 1.0;
    return v;
}

/* srgb_gamma(v)*255 rounded half-away-from-zero and clamped to [0,255]. The
 * gamma input is always in [0,1] here (the matrix stage clamps first), so the
 * value is non-negative and round == floor(v+0.5). */
static uint8_t to_u8(double v) {
    double scaled = irp_srgb_gamma(v) * 255.0;
    double rounded = (double)(long long)(scaled + 0.5); /* scaled >= 0 */
    if (rounded < 0.0) rounded = 0.0;
    if (rounded > 255.0) rounded = 255.0;
    return (uint8_t)rounded;
}

IrpStatus irp_apply_color_pipeline(const IrpRaw16 *pixels, size_t n,
                                   uint32_t black_level, uint32_t white_level,
                                   const double wb[3],
                                   const double color_matrix[3][3],
                                   IrpRgb8 **out) {
    *out = NULL;
    if (n == 0) return IRP_OK; /* empty input -> empty (NULL) output */

    /* Guard the allocation size against size_t overflow. */
    if (n > ((size_t)-1) / sizeof(IrpRgb8)) return IRP_ERR_ALLOC;
    IrpRgb8 *dst = (IrpRgb8 *)malloc(n * sizeof(IrpRgb8));
    if (dst == NULL) return IRP_ERR_ALLOC;

    /* Effective white: black->saturation span, clamped to >= 1 to avoid ÷0. */
    double effective_white = (double)sat_sub_u32(white_level, black_level);
    if (effective_white < 1.0) effective_white = 1.0;

    for (size_t i = 0; i < n; i++) {
        /* Stage 1: subtract black level, normalise to [0,1]. */
        double norm[3];
        norm[0] = (double)sat_sub_u32(pixels[i].r, black_level) / effective_white;
        norm[1] = (double)sat_sub_u32(pixels[i].g, black_level) / effective_white;
        norm[2] = (double)sat_sub_u32(pixels[i].b, black_level) / effective_white;

        /* Stage 2: white balance. */
        double bal[3] = {norm[0] * wb[0], norm[1] * wb[1], norm[2] * wb[2]};

        /* Stage 3: camera->sRGB colour matrix, then clamp to [0,1]. */
        double mixed[3];
        irp_mat3x3_mul(color_matrix, bal, mixed);

        /* Stage 4: sRGB gamma + scale to u8. */
        dst[i].r = to_u8(clamp01(mixed[0]));
        dst[i].g = to_u8(clamp01(mixed[1]));
        dst[i].b = to_u8(clamp01(mixed[2]));
    }

    *out = dst;
    return IRP_OK;
}
