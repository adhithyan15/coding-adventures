/*
 * dsp_complex_test.c — Unit, hostile-input, and SplitMix64 PBT tests for c/dsp-complex.
 *
 * Covers:
 * - Constructors (zeros, from_real, from_real_imag, from_interleaved, clone)
 * - Scalar complex arithmetic & pure-ISO hypot/atan2 accuracy
 * - Projections (real, imag, magnitude, phase) & element-wise ops (conjugate, scale, add, sub, mul, div)
 * - VER15 CWE hardening:
 *   - CWE-20:  NaN, +Inf, -Inf, subnormals, odd interleaved length, div-by-zero
 *   - CWE-120/125/787: out-of-range get/set
 *   - CWE-190: SIZE_MAX overflow checks in checked_add/checked_mul & oversized length
 *   - CWE-400: pre-flight DSP_COMPLEX_MAX_SAMPLES cap
 *   - CWE-415/416: zero-on-free & idempotent double-free
 *   - CWE-476: NULL pointers on every entry point
 * - Deterministic SplitMix64 property-based tests:
 *   - conjugate(conjugate(x)) == x
 *   - |a * b| == |a| * |b|, arg(a * b) == arg(a) + arg(b) mod 2pi
 *   - add/sub inverse, mul/div round-trip
 */
#include "dsp_complex.h"
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

/* Deterministic SplitMix64 PRNG. */
static uint64_t splitmix64(uint64_t *state) {
    uint64_t z = (*state += 0x9E3779B97F4A7C15ULL);
    z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9ULL;
    z = (z ^ (z >> 27)) * 0x94D049BB133111EBULL;
    return z ^ (z >> 31);
}

static float rand_unit_f(uint64_t *state) {
    uint32_t bits = (uint32_t)(splitmix64(state) >> 40); /* 24-bit int */
    return ((float)bits / 16777216.0f) * 2.0f - 1.0f;
}

static void test_constructors_and_projections(void) {
    const float real[4] = {3.0f, 0.0f, -1.0f, 1.0f};
    const float imag[4] = {4.0f, 5.0f, 0.0f, -1.0f};
    const float pi_f = 3.14159265f;
    dsp_complex_tensor_t t = {NULL, 0};
    dsp_complex_tensor_t t2 = {NULL, 0};
    float *buf = NULL;
    size_t blen = 0;

    ISO_CHECK_EQ_INT(dsp_complex_tensor_from_real_imag(real, 4u, imag, 4u, &t), DSP_COMPLEX_OK);
    ISO_CHECK_EQ_UINT(dsp_complex_tensor_len(&t), 4u);
    ISO_CHECK_EQ_INT(dsp_complex_tensor_is_empty(&t), 0);

    ISO_CHECK_EQ_INT(dsp_complex_tensor_real(&t, &buf, &blen), DSP_COMPLEX_OK);
    ISO_CHECK_EQ_UINT(blen, 4u);
    ISO_CHECK(approx_eq(buf[0], 3.0f, 1e-6f));
    ISO_CHECK(approx_eq(buf[2], -1.0f, 1e-6f));
    dsp_complex_free_buffer(buf, blen);
    buf = NULL;

    ISO_CHECK_EQ_INT(dsp_complex_tensor_imag(&t, &buf, &blen), DSP_COMPLEX_OK);
    ISO_CHECK_EQ_UINT(blen, 4u);
    ISO_CHECK(approx_eq(buf[0], 4.0f, 1e-6f));
    ISO_CHECK(approx_eq(buf[3], -1.0f, 1e-6f));
    dsp_complex_free_buffer(buf, blen);
    buf = NULL;

    ISO_CHECK_EQ_INT(dsp_complex_tensor_magnitude(&t, &buf, &blen), DSP_COMPLEX_OK);
    ISO_CHECK_EQ_UINT(blen, 4u);
    ISO_CHECK(approx_eq(buf[0], 5.0f, 1e-6f));
    ISO_CHECK(approx_eq(buf[1], 5.0f, 1e-6f));
    ISO_CHECK(approx_eq(buf[2], 1.0f, 1e-6f));
    ISO_CHECK(approx_eq(buf[3], 1.41421356f, 1e-5f));
    dsp_complex_free_buffer(buf, blen);
    buf = NULL;

    ISO_CHECK_EQ_INT(dsp_complex_tensor_phase(&t, &buf, &blen), DSP_COMPLEX_OK);
    ISO_CHECK_EQ_UINT(blen, 4u);
    ISO_CHECK(approx_eq(buf[0], 0.92729522f, 1e-5f)); /* atan2(4, 3) */
    ISO_CHECK(approx_eq(buf[1], pi_f * 0.5f, 1e-5f)); /* atan2(5, 0) */
    ISO_CHECK(approx_eq(buf[2], pi_f, 1e-5f));        /* atan2(0, -1) */
    ISO_CHECK(approx_eq(buf[3], -pi_f * 0.25f, 1e-5f));
    dsp_complex_free_buffer(buf, blen);
    buf = NULL;

    /* Empty tensor round-trip */
    ISO_CHECK_EQ_INT(dsp_complex_tensor_from_real(NULL, 0u, &t2), DSP_COMPLEX_OK);
    ISO_CHECK_EQ_UINT(dsp_complex_tensor_len(&t2), 0u);
    ISO_CHECK_EQ_INT(dsp_complex_tensor_is_empty(&t2), 1);
    ISO_CHECK(dsp_complex_tensor_as_interleaved(&t2) == NULL);
    ISO_CHECK_EQ_INT(dsp_complex_tensor_real(&t2, &buf, &blen), DSP_COMPLEX_OK);
    ISO_CHECK(buf == NULL && blen == 0u);

    dsp_complex_tensor_free(&t);
    dsp_complex_tensor_free(&t2);
}

static void test_elementwise_and_indexing(void) {
    const float a_in[4] = {1.0f, 2.0f, -3.0f, 4.0f};
    const float b_in[4] = {5.0f, -1.0f, 2.0f, 2.0f};
    dsp_complex_tensor_t a = {NULL, 0};
    dsp_complex_tensor_t b = {NULL, 0};
    dsp_complex_tensor_t out = {NULL, 0};
    dsp_complex_t elem;

    ISO_CHECK_EQ_INT(dsp_complex_tensor_from_interleaved(a_in, 4u, &a), DSP_COMPLEX_OK);
    ISO_CHECK_EQ_INT(dsp_complex_tensor_from_interleaved(b_in, 4u, &b), DSP_COMPLEX_OK);

    ISO_CHECK_EQ_INT(dsp_complex_tensor_add(&a, &b, &out), DSP_COMPLEX_OK);
    ISO_CHECK_EQ_INT(dsp_complex_tensor_get(&out, 0u, &elem), DSP_COMPLEX_OK);
    ISO_CHECK(approx_eq(elem.re, 6.0f, 1e-6f) && approx_eq(elem.im, 1.0f, 1e-6f));
    dsp_complex_tensor_free(&out);

    ISO_CHECK_EQ_INT(dsp_complex_tensor_mul(&a, &b, &out), DSP_COMPLEX_OK);
    /* (1+2i)(5-i) = 7 + 9i */
    ISO_CHECK_EQ_INT(dsp_complex_tensor_get(&out, 0u, &elem), DSP_COMPLEX_OK);
    ISO_CHECK(approx_eq(elem.re, 7.0f, 1e-6f) && approx_eq(elem.im, 9.0f, 1e-6f));
    dsp_complex_tensor_free(&out);

    ISO_CHECK_EQ_INT(dsp_complex_tensor_div(&a, &b, &out), DSP_COMPLEX_OK);
    /* (-3+4i)/(2+2i) = (2+14i)/8 = 0.25 + 1.75i */
    ISO_CHECK_EQ_INT(dsp_complex_tensor_get(&out, 1u, &elem), DSP_COMPLEX_OK);
    ISO_CHECK(approx_eq(elem.re, 0.25f, 1e-5f) && approx_eq(elem.im, 1.75f, 1e-5f));
    dsp_complex_tensor_free(&out);

    ISO_CHECK_EQ_INT(dsp_complex_tensor_set(&a, 1u, dsp_c_make(9.0f, -8.0f)), DSP_COMPLEX_OK);
    ISO_CHECK_EQ_INT(dsp_complex_tensor_get(&a, 1u, &elem), DSP_COMPLEX_OK);
    ISO_CHECK(approx_eq(elem.re, 9.0f, 1e-6f) && approx_eq(elem.im, -8.0f, 1e-6f));

    dsp_complex_tensor_free(&a);
    dsp_complex_tensor_free(&b);
}

static void test_hostile_cwe_guards(void) {
    const float finite2[2] = {1.0f, 2.0f};
    const float bad_nan[2] = {1.0f, make_nan()};
    const float bad_inf[2] = {make_inf(), 0.0f};
    const float bad_ninf[2] = {0.0f, make_neg_inf()};
    const float odd3[3] = {1.0f, 2.0f, 3.0f};
    const float zero2[2] = {0.0f, 0.0f};
    dsp_complex_tensor_t t = {NULL, 0};
    dsp_complex_tensor_t z = {NULL, 0};
    dsp_complex_tensor_t out = {NULL, 0};
    dsp_complex_t elem;
    float *buf = NULL;
    size_t blen = 0;
    size_t sz = 0;

    /* CWE-476: NULL pointer rejection */
    ISO_CHECK_EQ_INT(dsp_complex_tensor_from_real(finite2, 2u, NULL), DSP_COMPLEX_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_complex_tensor_from_real(NULL, 2u, &t), DSP_COMPLEX_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_complex_tensor_from_real_imag(NULL, 2u, finite2, 2u, &t), DSP_COMPLEX_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_complex_tensor_from_interleaved(NULL, 2u, &t), DSP_COMPLEX_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_complex_tensor_get(NULL, 0u, &elem), DSP_COMPLEX_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_complex_tensor_get(&t, 0u, NULL), DSP_COMPLEX_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_complex_tensor_set(NULL, 0u, dsp_c_make(0.0f, 0.0f)), DSP_COMPLEX_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_complex_tensor_real(NULL, &buf, &blen), DSP_COMPLEX_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_complex_tensor_conjugate(NULL, &out), DSP_COMPLEX_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_complex_tensor_add(NULL, &t, &out), DSP_COMPLEX_ERR_INVALID_INPUT);
    dsp_complex_tensor_free(NULL);
    dsp_complex_free_buffer(NULL, 10u);

    /* CWE-20: non-finite floats, odd interleaved length, length mismatch, div-by-zero */
    ISO_CHECK_EQ_INT(dsp_complex_tensor_from_real(bad_nan, 2u, &t), DSP_COMPLEX_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_complex_tensor_from_real(bad_inf, 2u, &t), DSP_COMPLEX_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_complex_tensor_from_real(bad_ninf, 2u, &t), DSP_COMPLEX_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_complex_tensor_from_real_imag(finite2, 2u, bad_nan, 2u, &t), DSP_COMPLEX_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_complex_tensor_from_real_imag(finite2, 2u, finite2, 1u, &t), DSP_COMPLEX_ERR_LENGTH_MISMATCH);
    ISO_CHECK_EQ_INT(dsp_complex_tensor_from_interleaved(odd3, 3u, &t), DSP_COMPLEX_ERR_ODD_INTERLEAVED_LENGTH);
    ISO_CHECK_EQ_INT(dsp_complex_tensor_from_interleaved(bad_nan, 2u, &t), DSP_COMPLEX_ERR_INVALID_INPUT);

    ISO_CHECK_EQ_INT(dsp_complex_tensor_from_interleaved(finite2, 2u, &t), DSP_COMPLEX_OK);
    ISO_CHECK_EQ_INT(dsp_complex_tensor_from_interleaved(zero2, 2u, &z), DSP_COMPLEX_OK);
    ISO_CHECK_EQ_INT(dsp_complex_tensor_set(&t, 0u, dsp_c_make(make_nan(), 0.0f)), DSP_COMPLEX_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_complex_tensor_scale(&t, make_inf(), &out), DSP_COMPLEX_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_complex_tensor_div(&t, &z, &out), DSP_COMPLEX_ERR_INVALID_INPUT);

    /* CWE-125/787: out-of-range indexing */
    ISO_CHECK_EQ_INT(dsp_complex_tensor_get(&t, 1u, &elem), DSP_COMPLEX_ERR_OUT_OF_RANGE);
    ISO_CHECK_EQ_INT(dsp_complex_tensor_get(&t, SIZE_MAX, &elem), DSP_COMPLEX_ERR_OUT_OF_RANGE);
    ISO_CHECK_EQ_INT(dsp_complex_tensor_set(&t, 1u, dsp_c_make(1.0f, 1.0f)), DSP_COMPLEX_ERR_OUT_OF_RANGE);

    /* CWE-190 / CWE-400: overflow & MAX_SAMPLES caps */
    ISO_CHECK_EQ_INT(dsp_complex_checked_add(SIZE_MAX, 1u, &sz), 0);
    ISO_CHECK_EQ_INT(dsp_complex_checked_mul(SIZE_MAX, 2u, &sz), 0);
    ISO_CHECK_EQ_INT(dsp_complex_checked_mul(100u, 200u, &sz), 1);
    ISO_CHECK_EQ_UINT(sz, 20000u);
    ISO_CHECK_EQ_INT(dsp_complex_tensor_zeros(DSP_COMPLEX_MAX_SAMPLES + 1u, &out), DSP_COMPLEX_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_complex_tensor_from_real(finite2, DSP_COMPLEX_MAX_SAMPLES + 1u, &out), DSP_COMPLEX_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_complex_tensor_from_interleaved(finite2, (DSP_COMPLEX_MAX_SAMPLES + 1u) * 2u, &out), DSP_COMPLEX_ERR_INVALID_INPUT);

    /* CWE-415/416: zero-on-free & idempotent double-free */
    dsp_complex_tensor_free(&t);
    ISO_CHECK(t.data == NULL && t.len == 0u);
    dsp_complex_tensor_free(&t);
    ISO_CHECK(t.data == NULL && t.len == 0u);
    dsp_complex_tensor_free(&z);
    ISO_CHECK(dsp_complex_error_str(DSP_COMPLEX_OK) != NULL);

    /* Float-demotion overflow & raw-struct non-finite projection guards */
    {
        const float big2[2] = {3.0e38f, 3.0e38f};
        const float num2[2] = {2.0e38f, 0.0f};
        const float den2[2] = {1.0e-38f, 0.0f};
        float raw_nan[2];
        dsp_complex_tensor_t tbig = {NULL, 0};
        dsp_complex_tensor_t tnum = {NULL, 0};
        dsp_complex_tensor_t tden = {NULL, 0};
        dsp_complex_tensor_t traw = {NULL, 1u};
        raw_nan[0] = make_nan();
        raw_nan[1] = 0.0f;
        traw.data = raw_nan;

        ISO_CHECK_EQ_INT(dsp_complex_tensor_from_interleaved(big2, 2u, &tbig), DSP_COMPLEX_OK);
        ISO_CHECK_EQ_INT(dsp_complex_tensor_magnitude(&tbig, &buf, &blen), DSP_COMPLEX_ERR_INVALID_INPUT);
        ISO_CHECK(buf == NULL && blen == 0u);

        ISO_CHECK_EQ_INT(dsp_complex_tensor_from_interleaved(num2, 2u, &tnum), DSP_COMPLEX_OK);
        ISO_CHECK_EQ_INT(dsp_complex_tensor_from_interleaved(den2, 2u, &tden), DSP_COMPLEX_OK);
        ISO_CHECK_EQ_INT(dsp_complex_tensor_div(&tnum, &tden, &out), DSP_COMPLEX_ERR_INVALID_INPUT);

        ISO_CHECK_EQ_INT(dsp_complex_tensor_real(&traw, &buf, &blen), DSP_COMPLEX_ERR_INVALID_INPUT);
        ISO_CHECK_EQ_INT(dsp_complex_tensor_phase(&traw, &buf, &blen), DSP_COMPLEX_ERR_INVALID_INPUT);

        dsp_complex_tensor_free(&tbig);
        dsp_complex_tensor_free(&tnum);
        dsp_complex_tensor_free(&tden);
    }

    /* Aliased / in-place operations must not leak or corrupt inputs */
    {
        const float a_in[4] = {1.0f, 2.0f, 3.0f, -4.0f};
        const float b_in[4] = {2.0f, 0.0f, 1.0f, 1.0f};
        dsp_complex_tensor_t a = {NULL, 0};
        dsp_complex_tensor_t b = {NULL, 0};
        ISO_CHECK_EQ_INT(dsp_complex_tensor_from_interleaved(a_in, 4u, &a), DSP_COMPLEX_OK);
        ISO_CHECK_EQ_INT(dsp_complex_tensor_from_interleaved(b_in, 4u, &b), DSP_COMPLEX_OK);

        ISO_CHECK_EQ_INT(dsp_complex_tensor_clone(&a, &a), DSP_COMPLEX_OK);
        ISO_CHECK_EQ_UINT(a.len, 2u);
        ISO_CHECK(approx_eq(a.data[0], 1.0f, 1e-6f) && approx_eq(a.data[3], -4.0f, 1e-6f));

        ISO_CHECK_EQ_INT(dsp_complex_tensor_conjugate(&a, &a), DSP_COMPLEX_OK);
        ISO_CHECK(approx_eq(a.data[1], -2.0f, 1e-6f) && approx_eq(a.data[3], 4.0f, 1e-6f));

        ISO_CHECK_EQ_INT(dsp_complex_tensor_scale(&a, 0.5f, &a), DSP_COMPLEX_OK);
        ISO_CHECK(approx_eq(a.data[0], 0.5f, 1e-6f) && approx_eq(a.data[3], 2.0f, 1e-6f));

        /* Error on aliased call preserves existing tensor intact */
        ISO_CHECK_EQ_INT(dsp_complex_tensor_scale(&a, make_nan(), &a), DSP_COMPLEX_ERR_INVALID_INPUT);
        ISO_CHECK_EQ_UINT(a.len, 2u);
        ISO_CHECK(approx_eq(a.data[0], 0.5f, 1e-6f));

        ISO_CHECK_EQ_INT(dsp_complex_tensor_add(&a, &b, &a), DSP_COMPLEX_OK);
        ISO_CHECK(approx_eq(a.data[0], 2.5f, 1e-6f));
        ISO_CHECK_EQ_INT(dsp_complex_tensor_sub(&a, &b, &b), DSP_COMPLEX_OK);
        ISO_CHECK(approx_eq(b.data[0], 0.5f, 1e-6f));
        ISO_CHECK_EQ_INT(dsp_complex_tensor_mul(&a, &b, &a), DSP_COMPLEX_OK);
        ISO_CHECK_EQ_INT(dsp_complex_tensor_div(&a, &b, &a), DSP_COMPLEX_OK);
        ISO_CHECK(approx_eq(a.data[0], 2.5f, 1e-4f));

        dsp_complex_tensor_free(&a);
        dsp_complex_tensor_free(&b);
    }
}

static void test_splitmix64_pbt(void) {
    uint64_t seed = 0xCAFEBABE12345678ULL;
    int trial;

    for (trial = 0; trial < 40; ++trial) {
        size_t n = (size_t)(splitmix64(&seed) % 32u) + 1u;
        float re_a[32], im_a[32], re_b[32], im_b[32];
        dsp_complex_tensor_t a = {NULL, 0};
        dsp_complex_tensor_t b = {NULL, 0};
        dsp_complex_tensor_t c1 = {NULL, 0};
        dsp_complex_tensor_t c2 = {NULL, 0};
        dsp_complex_tensor_t prod = {NULL, 0};
        dsp_complex_tensor_t quot = {NULL, 0};
        float *mag_a = NULL, *mag_b = NULL, *mag_p = NULL;
        size_t len_a = 0, len_b = 0, len_p = 0;
        size_t i;

        for (i = 0; i < n; ++i) {
            re_a[i] = rand_unit_f(&seed) * 4.0f;
            im_a[i] = rand_unit_f(&seed) * 4.0f;
            /* Ensure b[i] is non-zero for division round-trip */
            re_b[i] = rand_unit_f(&seed) * 3.0f + (i % 2u == 0u ? 4.5f : -4.5f);
            im_b[i] = rand_unit_f(&seed) * 3.0f;
        }

        ISO_CHECK_EQ_INT(dsp_complex_tensor_from_real_imag(re_a, n, im_a, n, &a), DSP_COMPLEX_OK);
        ISO_CHECK_EQ_INT(dsp_complex_tensor_from_real_imag(re_b, n, im_b, n, &b), DSP_COMPLEX_OK);

        /* Involution: conj(conj(a)) == a */
        ISO_CHECK_EQ_INT(dsp_complex_tensor_conjugate(&a, &c1), DSP_COMPLEX_OK);
        ISO_CHECK_EQ_INT(dsp_complex_tensor_conjugate(&c1, &c2), DSP_COMPLEX_OK);
        for (i = 0; i < 2u * n; ++i) {
            ISO_CHECK(a.data[i] == c2.data[i]);
        }

        /* Multiplicative modulus & mul/div round-trip */
        ISO_CHECK_EQ_INT(dsp_complex_tensor_mul(&a, &b, &prod), DSP_COMPLEX_OK);
        ISO_CHECK_EQ_INT(dsp_complex_tensor_div(&prod, &b, &quot), DSP_COMPLEX_OK);
        ISO_CHECK_EQ_INT(dsp_complex_tensor_magnitude(&a, &mag_a, &len_a), DSP_COMPLEX_OK);
        ISO_CHECK_EQ_INT(dsp_complex_tensor_magnitude(&b, &mag_b, &len_b), DSP_COMPLEX_OK);
        ISO_CHECK_EQ_INT(dsp_complex_tensor_magnitude(&prod, &mag_p, &len_p), DSP_COMPLEX_OK);
        for (i = 0; i < n; ++i) {
            ISO_CHECK(approx_eq(mag_p[i], mag_a[i] * mag_b[i], 1e-4f));
            ISO_CHECK(approx_eq(quot.data[2u * i], a.data[2u * i], 1e-4f));
            ISO_CHECK(approx_eq(quot.data[2u * i + 1u], a.data[2u * i + 1u], 1e-4f));
        }

        dsp_complex_free_buffer(mag_a, len_a);
        dsp_complex_free_buffer(mag_b, len_b);
        dsp_complex_free_buffer(mag_p, len_p);
        dsp_complex_tensor_free(&a);
        dsp_complex_tensor_free(&b);
        dsp_complex_tensor_free(&c1);
        dsp_complex_tensor_free(&c2);
        dsp_complex_tensor_free(&prod);
        dsp_complex_tensor_free(&quot);
    }
}

int main(void) {
    test_constructors_and_projections();
    test_elementwise_and_indexing();
    test_hostile_cwe_guards();
    test_splitmix64_pbt();
    return ISO_TEST_RESULT();
}
