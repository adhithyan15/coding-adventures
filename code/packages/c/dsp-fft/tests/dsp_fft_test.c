/*
 * dsp_fft_test.c — Unit, hostile-input, invariant, and SplitMix64 PBT tests for c/dsp-fft.
 *
 * Covers:
 * - Closed-form known vectors: impulse, DC, single-bin sinusoid, N=1 identity
 * - Naive O(N^2) DFT reference oracle for power-of-two AND non-power-of-two sizes
 * - Radix-2 & Bluestein round-trips (ifft(fft(x)) == x) across N=1..32, 64, 128, 255
 * - RFFT / IRFFT half-spectrum length, conjugate symmetry, and round-trips (even & odd N)
 * - Family-1/4 invariants: linearity, Parseval's energy conservation
 * - VER15 CWE hardening:
 *   - CWE-476: NULL pointers on all entry points
 *   - CWE-20:  NaN, +Inf, -Inf, zero length, odd interleaved length, non-pow2 on *_scalar,
 *              irfft bin mismatch / output_length == 0
 *   - CWE-190 / CWE-400: MAX_FFT_LEN, MAX_SAMPLES, Bluestein oversized convolution cap
 *   - CWE-415 / CWE-416: zero-on-free & double-free safety
 * - Deterministic SplitMix64 PBT across random lengths & signals
 */
#include "dsp_fft.h"
#include "iso_test.h"

#include <stdint.h>
#include <stdlib.h>
#include <string.h>

static float af(float x) { return x < 0.0f ? -x : x; }

static int approx_eq(float a, float b, float tol) {
    float scale = af(a);
    if (af(b) > scale) scale = af(b);
    if (scale < 1.0f) scale = 1.0f;
    return af(a - b) <= scale * tol;
}

static float make_nan(void) {
    union { uint32_t u; float f; } v;
    v.u = 0x7FC00000u;
    return v.f;
}

static float make_inf(void) {
    union { uint32_t u; float f; } v;
    v.u = 0x7F800000u;
    return v.f;
}

static float make_neg_inf(void) {
    union { uint32_t u; float f; } v;
    v.u = 0xFF800000u;
    return v.f;
}

/* Pure ISO sin/cos for DFT test oracle (no <math.h> / -lm). */
static void test_sincos(double x, double *s_out, double *c_out) {
    static const double HALF_PI = 1.57079632679489661923;
    static const double TWO_OVER_PI = 0.63661977236758134308;
    double kf = x * TWO_OVER_PI;
    long long k = (long long)(kf >= 0.0 ? kf + 0.5 : kf - 0.5);
    double r = x - (double)k * HALF_PI;
    double r2, st, ss, ct, cs;
    int n, q;
    while (r > HALF_PI * 0.5) { r -= HALF_PI; k += 1; }
    while (r < -HALF_PI * 0.5) { r += HALF_PI; k -= 1; }
    r2 = r * r;
    st = r; ss = r; ct = 1.0; cs = 1.0;
    for (n = 1; n <= 10; ++n) {
        st *= -r2 / (double)((2 * n) * (2 * n + 1)); ss += st;
        ct *= -r2 / (double)((2 * n - 1) * (2 * n)); cs += ct;
    }
    q = (int)(k & 3LL);
    if (q == 0) { *s_out = ss; *c_out = cs; }
    else if (q == 1) { *s_out = cs; *c_out = -ss; }
    else if (q == 2) { *s_out = -ss; *c_out = -cs; }
    else { *s_out = -cs; *c_out = ss; }
}

/* Naive O(N^2) DFT oracle in double precision. */
static void naive_dft(const float *interleaved, size_t n, int inverse, float *out) {
    static const double TWO_PI = 6.28318530717958647692;
    double sign = inverse ? 1.0 : -1.0;
    double scale = inverse ? (1.0 / (double)n) : 1.0;
    size_t k, j;
    for (k = 0u; k < n; ++k) {
        double sum_re = 0.0;
        double sum_im = 0.0;
        for (j = 0u; j < n; ++j) {
            double theta = sign * TWO_PI * (double)k * (double)j / (double)n;
            double s = 0.0, c = 1.0;
            double xr = (double)interleaved[2u * j];
            double xi = (double)interleaved[2u * j + 1u];
            test_sincos(theta, &s, &c);
            sum_re += xr * c - xi * s;
            sum_im += xr * s + xi * c;
        }
        out[2u * k] = (float)(sum_re * scale);
        out[2u * k + 1u] = (float)(sum_im * scale);
    }
}

static uint64_t splitmix64(uint64_t *state) {
    uint64_t z = (*state += 0x9E3779B97F4A7C15ULL);
    z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9ULL;
    z = (z ^ (z >> 27)) * 0x94D049BB133111EBULL;
    return z ^ (z >> 31);
}

static float rand_unit_f(uint64_t *state) {
    uint32_t bits = (uint32_t)(splitmix64(state) >> 40);
    return ((float)bits / 16777216.0f) * 2.0f - 1.0f;
}

static void test_closed_form_vectors(void) {
    size_t sizes[4] = {1u, 5u, 8u, 12u};
    size_t si;

    for (si = 0u; si < 4u; ++si) {
        size_t n = sizes[si];
        float *sig = (float *)calloc(2u * n, sizeof(float));
        dsp_complex_tensor_t spec = {NULL, 0};
        size_t k;

        /* Impulse: [1, 0, ...] -> spectrum is all (1, 0) */
        sig[0] = 1.0f;
        ISO_CHECK_EQ_INT(dsp_fft(sig, 2u * n, 1, &spec), DSP_FFT_OK);
        ISO_CHECK_EQ_UINT(dsp_complex_tensor_len(&spec), n);
        for (k = 0u; k < n; ++k) {
            ISO_CHECK(approx_eq(spec.data[2u * k], 1.0f, 1e-4f));
            ISO_CHECK(approx_eq(spec.data[2u * k + 1u], 0.0f, 1e-4f));
        }
        dsp_complex_tensor_free(&spec);

        /* DC constant 1.0 -> [N, 0, 0, ...] */
        for (k = 0u; k < n; ++k) {
            sig[2u * k] = 1.0f;
            sig[2u * k + 1u] = 0.0f;
        }
        ISO_CHECK_EQ_INT(dsp_fft(sig, 2u * n, 1, &spec), DSP_FFT_OK);
        ISO_CHECK(approx_eq(spec.data[0], (float)n, 1e-4f));
        ISO_CHECK(approx_eq(spec.data[1], 0.0f, 1e-4f));
        for (k = 1u; k < n; ++k) {
            ISO_CHECK(approx_eq(spec.data[2u * k], 0.0f, 1e-3f));
            ISO_CHECK(approx_eq(spec.data[2u * k + 1u], 0.0f, 1e-3f));
        }
        dsp_complex_tensor_free(&spec);
        free(sig);
    }

    /* Single-bin cosine at bin 1, N=8: X[1] = 4 + 0i, X[7] = 4 + 0i */
    {
        static const double TWO_PI = 6.28318530717958647692;
        float cos_sig[8];
        dsp_complex_tensor_t spec = {NULL, 0};
        size_t i;
        for (i = 0u; i < 8u; ++i) {
            double s = 0.0, c = 1.0;
            test_sincos(TWO_PI * (double)i / 8.0, &s, &c);
            cos_sig[i] = (float)c;
        }
        ISO_CHECK_EQ_INT(dsp_fft(cos_sig, 8u, 0, &spec), DSP_FFT_OK);
        ISO_CHECK(approx_eq(spec.data[2], 4.0f, 1e-5f));
        ISO_CHECK(approx_eq(spec.data[3], 0.0f, 1e-5f));
        ISO_CHECK(approx_eq(spec.data[14], 4.0f, 1e-5f));
        ISO_CHECK(approx_eq(spec.data[15], 0.0f, 1e-5f));
        dsp_complex_tensor_free(&spec);
    }
}

static void test_dft_oracle_and_roundtrips(void) {
    size_t n;
    for (n = 1u; n <= 32u; ++n) {
        float *sig = (float *)calloc(2u * n, sizeof(float));
        float *oracle = (float *)calloc(2u * n, sizeof(float));
        dsp_complex_tensor_t spec = {NULL, 0};
        dsp_complex_tensor_t recon = {NULL, 0};
        float *real_sig = (float *)calloc(n, sizeof(float));
        dsp_complex_tensor_t half = {NULL, 0};
        float *real_rec = NULL;
        size_t real_rec_len = 0u;
        size_t i;

        for (i = 0u; i < n; ++i) {
            double s = 0.0, c = 1.0;
            test_sincos((double)i * 0.37, &s, &c);
            sig[2u * i] = (float)s;
            sig[2u * i + 1u] = (float)(c * 0.5);
            real_sig[i] = (float)s;
        }

        naive_dft(sig, n, 0, oracle);
        ISO_CHECK_EQ_INT(dsp_fft(sig, 2u * n, 1, &spec), DSP_FFT_OK);
        for (i = 0u; i < 2u * n; ++i) {
            ISO_CHECK(approx_eq(spec.data[i], oracle[i], 1e-3f));
        }

        ISO_CHECK_EQ_INT(dsp_ifft(&spec, &recon), DSP_FFT_OK);
        for (i = 0u; i < 2u * n; ++i) {
            ISO_CHECK(approx_eq(recon.data[i], sig[i], 1e-3f));
        }

        /* RFFT / IRFFT round-trip & conjugate symmetry check */
        ISO_CHECK_EQ_INT(dsp_rfft(real_sig, n, &half), DSP_FFT_OK);
        ISO_CHECK_EQ_UINT(dsp_complex_tensor_len(&half), n / 2u + 1u);
        ISO_CHECK_EQ_INT(dsp_irfft(&half, (uint32_t)n, &real_rec, &real_rec_len), DSP_FFT_OK);
        ISO_CHECK_EQ_UINT(real_rec_len, n);
        for (i = 0u; i < n; ++i) {
            ISO_CHECK(approx_eq(real_rec[i], real_sig[i], 1e-3f));
        }

        dsp_fft_free_buffer(real_rec, real_rec_len);
        dsp_complex_tensor_free(&half);
        dsp_complex_tensor_free(&recon);
        dsp_complex_tensor_free(&spec);
        free(real_sig);
        free(oracle);
        free(sig);
    }
}

static void test_family_invariants(void) {
    /* Linearity & Parseval energy conservation across pow2 (16) and non-pow2 (13). */
    size_t ns[2] = {16u, 13u};
    size_t ni;
    for (ni = 0u; ni < 2u; ++ni) {
        size_t n = ns[ni];
        float *x = (float *)calloc(2u * n, sizeof(float));
        float *y = (float *)calloc(2u * n, sizeof(float));
        float *xy = (float *)calloc(2u * n, sizeof(float));
        const float alpha = 1.5f;
        const float beta = -0.75f;
        dsp_complex_tensor_t fx = {NULL, 0};
        dsp_complex_tensor_t fy = {NULL, 0};
        dsp_complex_tensor_t fxy = {NULL, 0};
        double e_time = 0.0;
        double e_freq = 0.0;
        size_t i;

        for (i = 0u; i < n; ++i) {
            x[2u * i] = (float)(i + 1u) * 0.25f;
            x[2u * i + 1u] = -0.5f;
            y[2u * i] = 1.0f - (float)i * 0.1f;
            y[2u * i + 1u] = 0.3f * (float)i;
            xy[2u * i] = alpha * x[2u * i] + beta * y[2u * i];
            xy[2u * i + 1u] = alpha * x[2u * i + 1u] + beta * y[2u * i + 1u];
            e_time += (double)x[2u * i] * (double)x[2u * i] +
                      (double)x[2u * i + 1u] * (double)x[2u * i + 1u];
        }

        ISO_CHECK_EQ_INT(dsp_fft(x, 2u * n, 1, &fx), DSP_FFT_OK);
        ISO_CHECK_EQ_INT(dsp_fft(y, 2u * n, 1, &fy), DSP_FFT_OK);
        ISO_CHECK_EQ_INT(dsp_fft(xy, 2u * n, 1, &fxy), DSP_FFT_OK);

        for (i = 0u; i < n; ++i) {
            float lin_re = alpha * fx.data[2u * i] + beta * fy.data[2u * i];
            float lin_im = alpha * fx.data[2u * i + 1u] + beta * fy.data[2u * i + 1u];
            ISO_CHECK(approx_eq(fxy.data[2u * i], lin_re, 1e-3f));
            ISO_CHECK(approx_eq(fxy.data[2u * i + 1u], lin_im, 1e-3f));
            e_freq += (double)fx.data[2u * i] * (double)fx.data[2u * i] +
                      (double)fx.data[2u * i + 1u] * (double)fx.data[2u * i + 1u];
        }
        ISO_CHECK(approx_eq((float)e_time, (float)(e_freq / (double)n), 1e-3f));

        dsp_complex_tensor_free(&fx);
        dsp_complex_tensor_free(&fy);
        dsp_complex_tensor_free(&fxy);
        free(x);
        free(y);
        free(xy);
    }
}

static void test_hostile_cwe_guards(void) {
    const float valid4[4] = {1.0f, 0.0f, 2.0f, 0.0f};
    const float nonpow2_6[6] = {1.0f, 0.0f, 2.0f, 0.0f, 3.0f, 0.0f};
    const float odd3[3] = {1.0f, 2.0f, 3.0f};
    const float nan2[2] = {make_nan(), 0.0f};
    const float inf2[2] = {0.0f, make_inf()};
    const float ninf2[2] = {make_neg_inf(), 0.0f};
    float *out = NULL;
    size_t olen = 0u;
    dsp_complex_tensor_t t = {NULL, 0};

    /* CWE-476: NULL pointer checks */
    ISO_CHECK_EQ_INT(dsp_fft_scalar(NULL, 4u, &out, &olen), DSP_FFT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_fft_scalar(valid4, 4u, NULL, &olen), DSP_FFT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_fft_scalar(valid4, 4u, &out, NULL), DSP_FFT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_ifft_scalar(NULL, 4u, &out, &olen), DSP_FFT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_bluestein_scalar(NULL, 4u, DSP_FFT_FORWARD, &out, &olen), DSP_FFT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_rfft_scalar(NULL, 4u, &out, &olen), DSP_FFT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_irfft_scalar(NULL, 4u, 2u, &out, &olen), DSP_FFT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_fft(NULL, 4u, 0, &t), DSP_FFT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_fft(valid4, 4u, 0, NULL), DSP_FFT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_ifft(NULL, &t), DSP_FFT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_rfft(NULL, 4u, &t), DSP_FFT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_irfft(NULL, 2u, &out, &olen), DSP_FFT_ERR_INVALID_INPUT);
    dsp_fft_free_buffer(NULL, 8u);

    /* CWE-20: zero, odd, non-pow2, non-finite, irfft mismatch */
    ISO_CHECK_EQ_INT(dsp_fft_scalar(valid4, 0u, &out, &olen), DSP_FFT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_fft_scalar(odd3, 3u, &out, &olen), DSP_FFT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_fft_scalar(nonpow2_6, 6u, &out, &olen), DSP_FFT_ERR_NOT_POWER_OF_TWO);
    ISO_CHECK_EQ_INT(dsp_ifft_scalar(nonpow2_6, 6u, &out, &olen), DSP_FFT_ERR_NOT_POWER_OF_TWO);
    ISO_CHECK_EQ_INT(dsp_fft_scalar(nan2, 2u, &out, &olen), DSP_FFT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_fft_scalar(inf2, 2u, &out, &olen), DSP_FFT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_fft_scalar(ninf2, 2u, &out, &olen), DSP_FFT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_fft(odd3, 3u, 1, &t), DSP_FFT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_fft(nan2, 2u, 0, &t), DSP_FFT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_rfft(valid4, 0u, &t), DSP_FFT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_rfft(inf2, 2u, &t), DSP_FFT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_irfft_scalar(valid4, 4u, 0u, &out, &olen), DSP_FFT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_irfft_scalar(valid4, 4u, 8u, &out, &olen), DSP_FFT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_irfft_scalar(odd3, 3u, 2u, &out, &olen), DSP_FFT_ERR_INVALID_INPUT);

    /* CWE-190 / CWE-400: bounds exceeding MAX_FFT_LEN / MAX_SAMPLES / Bluestein 2N-1 */
    ISO_CHECK_EQ_INT(dsp_fft(valid4, DSP_FFT_MAX_LEN + 1u, 0, &t), DSP_FFT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_fft_scalar(valid4, (DSP_FFT_MAX_LEN + 1u) * 2u, &out, &olen), DSP_FFT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_fft_scalar(valid4, DSP_FFT_MAX_SAMPLES + 2u, &out, &olen), DSP_FFT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_bluestein_scalar(valid4, ((DSP_FFT_MAX_LEN / 2u) + 1u) * 2u,
                                          DSP_FFT_FORWARD, &out, &olen),
                     DSP_FFT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_irfft_scalar(valid4, 4u, (uint32_t)(DSP_FFT_MAX_LEN + 1u), &out, &olen),
                     DSP_FFT_ERR_INVALID_INPUT);
    ISO_CHECK(dsp_fft_error_str(DSP_FFT_OK) != NULL);

    /* CWE-20: invalid direction enum and near-FLT_MAX arithmetic overflow */
    {
        const float big4[4] = {2.0e38f, 0.0f, 2.0e38f, 0.0f};
        const float rbig2[2] = {2.0e38f, 2.0e38f};
        const float big6[6] = {2.0e38f, 0.0f, 2.0e38f, 0.0f, 2.0e38f, 0.0f};
        ISO_CHECK_EQ_INT(dsp_bluestein_scalar(nonpow2_6, 6u, (dsp_fft_direction_t)99, &out, &olen),
                         DSP_FFT_ERR_INVALID_INPUT);
        ISO_CHECK_EQ_INT(dsp_fft_scalar(big4, 4u, &out, &olen), DSP_FFT_ERR_INVALID_INPUT);
        ISO_CHECK(out == NULL && olen == 0u);
        ISO_CHECK_EQ_INT(dsp_bluestein_scalar(big6, 6u, DSP_FFT_FORWARD, &out, &olen),
                         DSP_FFT_ERR_INVALID_INPUT);
        ISO_CHECK(out == NULL && olen == 0u);
        ISO_CHECK_EQ_INT(dsp_fft(big4, 4u, 1, &t), DSP_FFT_ERR_INVALID_INPUT);
        ISO_CHECK_EQ_INT(dsp_rfft(rbig2, 2u, &t), DSP_FFT_ERR_INVALID_INPUT);
        ISO_CHECK_EQ_INT(dsp_irfft_scalar(big4, 4u, 2u, &out, &olen), DSP_FFT_ERR_INVALID_INPUT);
    }

    /* Aliased / in-place dsp_ifft(&spec, &spec) must not leak or wipe input */
    {
        const float sig4[4] = {1.0f, 0.0f, 2.0f, 0.0f};
        dsp_complex_tensor_t spec = {NULL, 0};
        ISO_CHECK_EQ_INT(dsp_fft(sig4, 4u, 0, &spec), DSP_FFT_OK);
        ISO_CHECK_EQ_INT(dsp_ifft(&spec, &spec), DSP_FFT_OK);
        ISO_CHECK_EQ_UINT(spec.len, 4u);
        ISO_CHECK(approx_eq(spec.data[0], 1.0f, 1e-4f));
        ISO_CHECK(approx_eq(spec.data[2], 0.0f, 1e-4f));
        ISO_CHECK(approx_eq(spec.data[4], 2.0f, 1e-4f));
        ISO_CHECK(approx_eq(spec.data[6], 0.0f, 1e-4f));
        dsp_complex_tensor_free(&spec);
    }
}

static void test_splitmix64_pbt(void) {
    uint64_t seed = 0x13579BDF2468ACE0ULL;
    int trial;

    for (trial = 0; trial < 32; ++trial) {
        size_t n = (size_t)(splitmix64(&seed) % 45u) + 1u;
        float sig[90];
        float real_sig[45];
        float oracle[90];
        dsp_complex_tensor_t spec = {NULL, 0};
        dsp_complex_tensor_t recon = {NULL, 0};
        dsp_complex_tensor_t half = {NULL, 0};
        float *r_rec = NULL;
        size_t r_len = 0u;
        double e_time = 0.0;
        double e_freq = 0.0;
        size_t i;

        for (i = 0u; i < n; ++i) {
            sig[2u * i] = rand_unit_f(&seed) * 2.0f;
            sig[2u * i + 1u] = rand_unit_f(&seed) * 2.0f;
            real_sig[i] = sig[2u * i];
            e_time += (double)sig[2u * i] * (double)sig[2u * i] +
                      (double)sig[2u * i + 1u] * (double)sig[2u * i + 1u];
        }

        naive_dft(sig, n, 0, oracle);
        ISO_CHECK_EQ_INT(dsp_fft(sig, 2u * n, 1, &spec), DSP_FFT_OK);
        ISO_CHECK_EQ_INT(dsp_ifft(&spec, &recon), DSP_FFT_OK);

        for (i = 0u; i < n; ++i) {
            ISO_CHECK(approx_eq(spec.data[2u * i], oracle[2u * i], 1e-3f));
            ISO_CHECK(approx_eq(spec.data[2u * i + 1u], oracle[2u * i + 1u], 1e-3f));
            ISO_CHECK(approx_eq(recon.data[2u * i], sig[2u * i], 1e-3f));
            ISO_CHECK(approx_eq(recon.data[2u * i + 1u], sig[2u * i + 1u], 1e-3f));
            e_freq += (double)spec.data[2u * i] * (double)spec.data[2u * i] +
                      (double)spec.data[2u * i + 1u] * (double)spec.data[2u * i + 1u];
        }
        ISO_CHECK(approx_eq((float)e_time, (float)(e_freq / (double)n), 1e-3f));

        ISO_CHECK_EQ_INT(dsp_rfft(real_sig, n, &half), DSP_FFT_OK);
        ISO_CHECK_EQ_INT(dsp_irfft(&half, (uint32_t)n, &r_rec, &r_len), DSP_FFT_OK);
        for (i = 0u; i < n; ++i) {
            ISO_CHECK(approx_eq(r_rec[i], real_sig[i], 1e-3f));
        }

        dsp_fft_free_buffer(r_rec, r_len);
        dsp_complex_tensor_free(&half);
        dsp_complex_tensor_free(&recon);
        dsp_complex_tensor_free(&spec);
    }
}

int main(void) {
    test_closed_form_vectors();
    test_dft_oracle_and_roundtrips();
    test_family_invariants();
    test_hostile_cwe_guards();
    test_splitmix64_pbt();
    return ISO_TEST_RESULT();
}
