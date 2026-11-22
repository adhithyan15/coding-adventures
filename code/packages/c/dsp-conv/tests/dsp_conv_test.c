/*
 * dsp_conv_test.c — unit, Family-1/4 invariant, hostile-input, and SplitMix64 PBT
 * tests for c/dsp-conv (pure ISO C17).
 */
#include "dsp_conv.h"
#include "iso_test.h"

#include <float.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

static float af(float x) { return x < 0.0f ? -x : x; }

static int approx_eq(float a, float b, float tol) {
    float scale = af(a);
    if (af(b) > scale) scale = af(b);
    if (scale < 1.0f) scale = 1.0f;
    return af(a - b) <= scale * tol;
}

static void check_close(const float *a, const float *b, size_t n, float tol) {
    size_t i;
    for (i = 0; i < n; ++i) {
        ISO_CHECK(approx_eq(a[i], b[i], tol));
    }
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

static float make_min_subnormal(void) {
    union { uint32_t u; float f; } v;
    v.u = 0x00000001u;
    return v.f;
}

/* Deterministic SplitMix64 PRNG */
typedef struct {
    uint64_t state;
} splitmix64_t;

static uint64_t sm64_next(splitmix64_t *rng) {
    uint64_t z;
    rng->state += UINT64_C(0x9E3779B97F4A7C15);
    z = rng->state;
    z = (z ^ (z >> 30)) * UINT64_C(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)) * UINT64_C(0x94D049BB133111EB);
    return z ^ (z >> 31);
}

static float sm64_sample_f32(splitmix64_t *rng, float min_v, float max_v) {
    uint64_t u = sm64_next(rng);
    double unit = (double)(u >> 11) / 9007199254740992.0;
    return (float)((double)min_v + ((double)max_v - (double)min_v) * unit);
}

static void test_index_and_parsing(void) {
    int hv = 0;
    size_t idx = 0;
    float sv = 0.0f;
    size_t np = 0;
    conv_boundary_t b = CONV_BOUND_ZERO;
    conv_mode_t m = CONV_MODE_FULL;
    const float sig[3] = {10.0f, 20.0f, 30.0f};

    ISO_CHECK(conv_extend_index(2, 5, CONV_BOUND_ZERO, &hv, &idx) == CONV_OK);
    ISO_CHECK(hv == 1 && idx == 2u);
    ISO_CHECK(conv_extend_index(-1, 5, CONV_BOUND_ZERO, &hv, &idx) == CONV_OK);
    ISO_CHECK(hv == 0);
    ISO_CHECK(conv_extend_index(-3, 5, CONV_BOUND_REPLICATE, &hv, &idx) == CONV_OK);
    ISO_CHECK(hv == 1 && idx == 0u);
    ISO_CHECK(conv_extend_index(7, 5, CONV_BOUND_REPLICATE, &hv, &idx) == CONV_OK);
    ISO_CHECK(hv == 1 && idx == 4u);
    ISO_CHECK(conv_extend_index(-1, 5, CONV_BOUND_WRAP, &hv, &idx) == CONV_OK);
    ISO_CHECK(hv == 1 && idx == 4u);
    ISO_CHECK(conv_extend_index(6, 5, CONV_BOUND_WRAP, &hv, &idx) == CONV_OK);
    ISO_CHECK(hv == 1 && idx == 1u);
    ISO_CHECK(conv_extend_index(-1, 4, CONV_BOUND_REFLECT, &hv, &idx) == CONV_OK);
    ISO_CHECK(hv == 1 && idx == 1u);
    ISO_CHECK(conv_extend_index(4, 4, CONV_BOUND_REFLECT, &hv, &idx) == CONV_OK);
    ISO_CHECK(hv == 1 && idx == 2u);
    ISO_CHECK(conv_extend_index(3, 1, CONV_BOUND_REFLECT, &hv, &idx) == CONV_OK);
    ISO_CHECK(hv == 1 && idx == 0u);

    ISO_CHECK(conv_sample(sig, 3, -1, CONV_BOUND_ZERO, &sv) == CONV_OK);
    ISO_CHECK(approx_eq(sv, 0.0f, 1e-7f));
    ISO_CHECK(conv_sample(sig, 3, -1, CONV_BOUND_REPLICATE, &sv) == CONV_OK);
    ISO_CHECK(approx_eq(sv, 10.0f, 1e-7f));
    ISO_CHECK(conv_sample(sig, 3, 3, CONV_BOUND_REPLICATE, &sv) == CONV_OK);
    ISO_CHECK(approx_eq(sv, 30.0f, 1e-7f));

    ISO_CHECK(conv_next_pow2(0, &np) == CONV_OK && np == 1u);
    ISO_CHECK(conv_next_pow2(1, &np) == CONV_OK && np == 1u);
    ISO_CHECK(conv_next_pow2(3, &np) == CONV_OK && np == 4u);
    ISO_CHECK(conv_next_pow2(100, &np) == CONV_OK && np == 128u);

    ISO_CHECK(conv_parse_boundary("  Reflect \n", &b) == CONV_OK && b == CONV_BOUND_REFLECT);
    ISO_CHECK(conv_parse_boundary("WRAP", &b) == CONV_OK && b == CONV_BOUND_WRAP);
    ISO_CHECK(conv_parse_boundary("zero extra", &b) == CONV_ERR_INVALID_INPUT);
    ISO_CHECK(conv_parse_mode(" Valid ", &m) == CONV_OK && m == CONV_MODE_VALID);
    ISO_CHECK(conv_parse_mode("full!", &m) == CONV_ERR_INVALID_INPUT);
}

static void test_conv1d_and_modes(void) {
    const float sig[5] = {1.0f, 2.0f, 3.0f, 4.0f, 5.0f};
    const float id_k[1] = {1.0f};
    const float delta_k[3] = {0.0f, 1.0f, 0.0f};
    const float ones3[3] = {1.0f, 1.0f, 1.0f};
    const float s3[3] = {1.0f, 2.0f, 3.0f};
    const float s3_ref[3] = {10.0f, 20.0f, 30.0f};
    float *out = NULL;
    size_t olen = 0;
    int m;

    for (m = 0; m < 4; ++m) {
        ISO_CHECK(conv_conv1d(sig, 5, id_k, 1, (conv_boundary_t)m, &out, &olen) == CONV_OK);
        ISO_CHECK_EQ_UINT(olen, 5);
        check_close(out, sig, 5, 1e-6f);
        conv_free_buf(&out, olen);
        ISO_CHECK(out == NULL);
    }

    ISO_CHECK(conv_conv1d(sig, 5, delta_k, 3, CONV_BOUND_ZERO, &out, &olen) == CONV_OK);
    check_close(out, sig, 5, 1e-6f);
    conv_free_buf(&out, olen);

    {
        const float exp_rep[3] = {4.0f, 6.0f, 8.0f};
        ISO_CHECK(conv_conv1d(s3, 3, ones3, 3, CONV_BOUND_REPLICATE, &out, &olen) == CONV_OK);
        check_close(out, exp_rep, 3, 1e-6f);
        conv_free_buf(&out, olen);
    }
    {
        const float exp_ref[3] = {50.0f, 60.0f, 70.0f};
        ISO_CHECK(conv_conv1d(s3_ref, 3, ones3, 3, CONV_BOUND_REFLECT, &out, &olen) == CONV_OK);
        check_close(out, exp_ref, 3, 1e-6f);
        conv_free_buf(&out, olen);
    }
    {
        const float exp_wrap[3] = {6.0f, 6.0f, 6.0f};
        ISO_CHECK(conv_conv1d(s3, 3, ones3, 3, CONV_BOUND_WRAP, &out, &olen) == CONV_OK);
        check_close(out, exp_wrap, 3, 1e-6f);
        conv_free_buf(&out, olen);
    }

    /* Full / Same / Valid + Correlate */
    {
        const float k2[2] = {0.5f, 1.0f};
        const float exp_full[4] = {0.5f, 2.0f, 3.5f, 3.0f};
        const float exp_valid[2] = {2.0f, 3.5f};
        ISO_CHECK(conv_convolve(s3, 3, k2, 2, CONV_MODE_FULL, CONV_BOUND_ZERO, &out, &olen) == CONV_OK);
        ISO_CHECK_EQ_UINT(olen, 4);
        check_close(out, exp_full, 4, 1e-6f);
        conv_free_buf(&out, olen);

        ISO_CHECK(conv_convolve(s3, 3, k2, 2, CONV_MODE_VALID, CONV_BOUND_ZERO, &out, &olen) == CONV_OK);
        ISO_CHECK_EQ_UINT(olen, 2);
        check_close(out, exp_valid, 2, 1e-6f);
        conv_free_buf(&out, olen);
    }
    {
        const float pat[2] = {2.0f, 3.0f};
        const float exp_corr[4] = {8.0f, 13.0f, 18.0f, 23.0f};
        ISO_CHECK(conv_correlate(sig, 5, pat, 2, CONV_MODE_VALID, &out, &olen) == CONV_OK);
        ISO_CHECK_EQ_UINT(olen, 4);
        check_close(out, exp_corr, 4, 1e-6f);
        conv_free_buf(&out, olen);
    }
}

static void test_fft_and_overlap_add(void) {
    const float sig[8] = {1.0f, -2.0f, 3.5f, 0.5f, -1.0f, 2.0f, 4.0f, -3.0f};
    const float ker[5] = {0.1f, 0.2f, 0.4f, 0.2f, 0.1f};
    const size_t bsizes[3] = {4u, 8u, 16u};
    int mode_idx;

    for (mode_idx = 0; mode_idx < 3; ++mode_idx) {
        conv_mode_t mode = (conv_mode_t)mode_idx;
        float *d_out = NULL, *f_out = NULL, *o_out = NULL;
        size_t d_len = 0, f_len = 0, o_len = 0;
        size_t b;

        ISO_CHECK(conv_convolve(sig, 8, ker, 5, mode, CONV_BOUND_ZERO, &d_out, &d_len) == CONV_OK);
        ISO_CHECK(conv_convolve_fft(sig, 8, ker, 5, mode, &f_out, &f_len) == CONV_OK);
        ISO_CHECK_EQ_UINT(d_len, f_len);
        check_close(d_out, f_out, d_len, 1e-5f);

        ISO_CHECK(conv_overlap_add(sig, 8, ker, 5, NULL, mode, &o_out, &o_len) == CONV_OK);
        ISO_CHECK_EQ_UINT(d_len, o_len);
        check_close(d_out, o_out, d_len, 1e-5f);
        conv_free_buf(&o_out, o_len);

        for (b = 0; b < 3; ++b) {
            ISO_CHECK(conv_overlap_add(sig, 8, ker, 5, &bsizes[b], mode, &o_out, &o_len) == CONV_OK);
            ISO_CHECK_EQ_UINT(d_len, o_len);
            check_close(d_out, o_out, d_len, 1e-5f);
            conv_free_buf(&o_out, o_len);
        }

        conv_free_buf(&d_out, d_len);
        conv_free_buf(&f_out, f_len);
    }
}

static void test_2d_and_kernels(void) {
    const float img[9] = {1.0f, 2.0f, 3.0f, 4.0f, 5.0f, 6.0f, 7.0f, 8.0f, 9.0f};
    const float delta2d[9] = {0, 0, 0, 0, 1, 0, 0, 0, 0};
    const float h_ker[3] = {0.25f, 0.5f, 0.25f};
    const float v_ker[3] = {0.2f, 0.6f, 0.2f};
    float outer[9];
    float *out2d = NULL, *out_sep = NULL, *out_full = NULL, *out_valid = NULL;
    size_t l2d = 0, lsep = 0, lfull = 0, lvalid = 0;
    size_t r, c;

    for (r = 0; r < 3; ++r) {
        for (c = 0; c < 3; ++c) {
            outer[r * 3 + c] = v_ker[r] * h_ker[c];
        }
    }

    ISO_CHECK(conv_conv2d(img, 9, delta2d, 9, 3, 3, 3, 3, CONV_BOUND_ZERO, &out2d, &l2d) == CONV_OK);
    check_close(out2d, img, 9, 1e-6f);
    conv_free_buf(&out2d, l2d);

    ISO_CHECK(conv_conv2d(img, 9, outer, 9, 3, 3, 3, 3, CONV_BOUND_REPLICATE, &out2d, &l2d) == CONV_OK);
    ISO_CHECK(conv_sep_conv2d(img, 9, h_ker, 3, v_ker, 3, 3, 3, CONV_BOUND_REPLICATE, &out_sep, &lsep) == CONV_OK);
    ISO_CHECK_EQ_UINT(l2d, lsep);
    check_close(out2d, out_sep, l2d, 1e-5f);
    conv_free_buf(&out2d, l2d);
    conv_free_buf(&out_sep, lsep);

    {
        const float k2x2[4] = {0.0f, 1.0f, 1.0f, 0.0f};
        ISO_CHECK(conv_convolve_2d(img, 9, k2x2, 4, 3, 3, 2, 2,
                                   CONV_MODE_FULL, CONV_BOUND_ZERO, &out_full, &lfull) == CONV_OK);
        ISO_CHECK_EQ_UINT(lfull, 16);
        ISO_CHECK(conv_convolve_2d(img, 9, k2x2, 4, 3, 3, 2, 2,
                                   CONV_MODE_VALID, CONV_BOUND_ZERO, &out_valid, &lvalid) == CONV_OK);
        ISO_CHECK_EQ_UINT(lvalid, 4);
        conv_free_buf(&out_full, lfull);
        conv_free_buf(&out_valid, lvalid);
    }

    /* Kernels */
    {
        float *gk = NULL, *bk = NULL, *sx = NULL, *sy = NULL, *lap = NULL, *sh = NULL;
        size_t gkl = 0, bkl = 0, sxl = 0, syl = 0, lapl = 0, shl = 0;
        double gsum = 0.0, bsum = 0.0, sxsum = 0.0, sysum = 0.0, lapsum = 0.0, shsum = 0.0;
        size_t i;

        ISO_CHECK(conv_gaussian_blur_kernel(1.5f, 9u, &gk, &gkl) == CONV_OK);
        ISO_CHECK_EQ_UINT(gkl, 9);
        for (i = 0; i < gkl; ++i) gsum += gk[i];
        ISO_CHECK(approx_eq((float)gsum, 1.0f, 1e-5f));
        for (i = 0; i < gkl / 2; ++i) ISO_CHECK(approx_eq(gk[i], gk[gkl - 1 - i], 1e-6f));

        ISO_CHECK(conv_box_blur_kernel(5u, &bk, &bkl) == CONV_OK);
        for (i = 0; i < bkl; ++i) bsum += bk[i];
        ISO_CHECK(approx_eq((float)bsum, 1.0f, 1e-5f));

        ISO_CHECK(conv_sobel_x_kernel(&sx, &sxl) == CONV_OK);
        ISO_CHECK(conv_sobel_y_kernel(&sy, &syl) == CONV_OK);
        ISO_CHECK(conv_laplacian_kernel(&lap, &lapl) == CONV_OK);
        ISO_CHECK(conv_sharpen_kernel(1.5f, &sh, &shl) == CONV_OK);
        for (i = 0; i < 9u; ++i) {
            sxsum += sx[i]; sysum += sy[i]; lapsum += lap[i]; shsum += sh[i];
        }
        ISO_CHECK(approx_eq((float)sxsum, 0.0f, 1e-6f));
        ISO_CHECK(approx_eq((float)sysum, 0.0f, 1e-6f));
        ISO_CHECK(approx_eq((float)lapsum, 0.0f, 1e-6f));
        ISO_CHECK(approx_eq((float)shsum, 1.0f, 1e-5f));

        conv_free_buf(&gk, gkl);
        conv_free_buf(&bk, bkl);
        conv_free_buf(&sx, sxl);
        conv_free_buf(&sy, syl);
        conv_free_buf(&lap, lapl);
        conv_free_buf(&sh, shl);
    }
}

static void test_hostile_and_cwe(void) {
    float dummy[4] = {1.0f, 2.0f, 3.0f, 4.0f};
    float *out = NULL;
    size_t olen = 0;
    float bad_vals[3];
    int i;
    size_t zero_bs = 0;

    bad_vals[0] = make_nan();
    bad_vals[1] = make_inf();
    bad_vals[2] = make_neg_inf();

    /* Empty / NULL / Invalid enum */
    ISO_CHECK(conv_conv1d(dummy, 0, dummy, 1, CONV_BOUND_ZERO, &out, &olen) == CONV_ERR_EMPTY_SIGNAL);
    ISO_CHECK(conv_conv1d(dummy, 1, dummy, 0, CONV_BOUND_ZERO, &out, &olen) == CONV_ERR_EMPTY_KERNEL);
    ISO_CHECK(conv_conv1d(NULL, 4, dummy, 1, CONV_BOUND_ZERO, &out, &olen) == CONV_ERR_INVALID_INPUT);
    ISO_CHECK(conv_conv1d(dummy, 4, dummy, 1, (conv_boundary_t)99, &out, &olen) == CONV_ERR_INVALID_INPUT);
    ISO_CHECK(conv_convolve(dummy, 4, dummy, 1, (conv_mode_t)99, CONV_BOUND_ZERO, &out, &olen) == CONV_ERR_INVALID_INPUT);

    /* CWE-125 / CWE-400: oversized cap rejected BEFORE reading element pointers */
    ISO_CHECK(conv_conv1d(dummy, (size_t)CONV_MAX_SAMPLES + 1u, dummy, 1, CONV_BOUND_ZERO, &out, &olen) == CONV_ERR_INVALID_INPUT);
    ISO_CHECK(conv_conv1d(dummy, 1, dummy, (size_t)CONV_MAX_KERNEL_LEN + 1u, CONV_BOUND_ZERO, &out, &olen) == CONV_ERR_INVALID_INPUT);
    ISO_CHECK(conv_conv2d(dummy, 4, dummy, 1, 10000u, 10000u, 1u, 1u, CONV_BOUND_ZERO, &out, &olen) == CONV_ERR_INVALID_INPUT);
    ISO_CHECK(conv_conv2d(dummy, 4, dummy, 1, 100u, 100u, 1u, 1u, CONV_BOUND_ZERO, &out, &olen) == CONV_ERR_IMAGE_SIZE_MISMATCH);
    {
        conv_error_t ov_err = conv_conv2d(dummy, 4, dummy, 1, UINT32_MAX, UINT32_MAX, 1u, 1u, CONV_BOUND_ZERO, &out, &olen);
        ISO_CHECK(ov_err == CONV_ERR_IMAGE_SIZE_MISMATCH || ov_err == CONV_ERR_INVALID_INPUT);
    }

    /* Non-finite inputs */
    for (i = 0; i < 3; ++i) {
        float sig_bad[2] = {1.0f, 0.0f};
        sig_bad[1] = bad_vals[i];
        ISO_CHECK(conv_conv1d(sig_bad, 2, dummy, 1, CONV_BOUND_ZERO, &out, &olen) == CONV_ERR_INVALID_INPUT);
        ISO_CHECK(conv_convolve_fft(sig_bad, 2, dummy, 1, CONV_MODE_FULL, &out, &olen) == CONV_ERR_INVALID_INPUT);
        ISO_CHECK(conv_overlap_add(sig_bad, 2, dummy, 1, NULL, CONV_MODE_FULL, &out, &olen) == CONV_ERR_INVALID_INPUT);
        ISO_CHECK(conv_gaussian_blur_kernel(bad_vals[i], 3u, &out, &olen) == CONV_ERR_INVALID_INPUT);
        ISO_CHECK(conv_sharpen_kernel(bad_vals[i], &out, &olen) == CONV_ERR_INVALID_INPUT);
    }

    /* Subnormal / overflow kernels & output overflow */
    ISO_CHECK(conv_gaussian_blur_kernel(make_min_subnormal(), 5u, &out, &olen) == CONV_ERR_INVALID_INPUT);
    ISO_CHECK(conv_gaussian_blur_kernel(FLT_MAX, 5u, &out, &olen) == CONV_ERR_INVALID_INPUT);
    ISO_CHECK(conv_gaussian_blur_kernel(1.0f, 4u, &out, &olen) == CONV_ERR_INVALID_INPUT);
    ISO_CHECK(conv_sharpen_kernel(FLT_MAX, &out, &olen) == CONV_ERR_INVALID_INPUT);
    {
        const float hot[2] = {FLT_MAX, FLT_MAX};
        const float two[2] = {2.0f, 2.0f};
        ISO_CHECK(conv_conv1d(hot, 2, two, 2, CONV_BOUND_REPLICATE, &out, &olen) == CONV_ERR_INVALID_INPUT);
        ISO_CHECK(conv_convolve_fft(hot, 2, two, 2, CONV_MODE_FULL, &out, &olen) == CONV_ERR_INVALID_INPUT);
    }

    /* Overlap-add zero block size (including Valid mode where signal < kernel) & valid kernel too large */
    ISO_CHECK(conv_overlap_add(dummy, 4, dummy, 2, &zero_bs, CONV_MODE_FULL, &out, &olen) == CONV_ERR_INVALID_INPUT);
    ISO_CHECK(conv_overlap_add(dummy, 2, dummy, 3, &zero_bs, CONV_MODE_VALID, &out, &olen) == CONV_ERR_INVALID_INPUT);
    ISO_CHECK(conv_convolve(dummy, 2, dummy, 3, CONV_MODE_VALID, CONV_BOUND_ZERO, &out, &olen) == CONV_ERR_KERNEL_TOO_LARGE);
    ISO_CHECK(conv_convolve_2d(dummy, 0, dummy, 1, 0u, 2u, 1u, 1u, (conv_mode_t)99, CONV_BOUND_ZERO, &out, &olen) == CONV_ERR_IMAGE_SIZE_MISMATCH);
    ISO_CHECK(conv_convolve_2d(dummy, 4, dummy, 0, 2u, 2u, 0u, 1u, (conv_mode_t)99, CONV_BOUND_ZERO, &out, &olen) == CONV_ERR_EMPTY_KERNEL);

    /* Aliasing safety: signal == kernel */
    ISO_CHECK(conv_correlate(dummy, 4, dummy, 4, CONV_MODE_VALID, &out, &olen) == CONV_OK);
    ISO_CHECK_EQ_UINT(olen, 1);
    ISO_CHECK(approx_eq(out[0], 30.0f, 1e-6f));
    /* CWE-415/416: idempotent zero-on-free */
    conv_free_buf(&out, olen);
    ISO_CHECK(out == NULL);
    conv_free_buf(&out, olen);
    ISO_CHECK(out == NULL);
}

static void test_pbt_splitmix64_invariants(void) {
    splitmix64_t rng;
    int iter;
    rng.state = UINT64_C(0xCAFEBABE01234567);

    for (iter = 0; iter < 15; ++iter) {
        size_t len_a = 4u + (size_t)(sm64_next(&rng) % 24u);
        size_t len_b = 3u + (size_t)(sm64_next(&rng) % 15u);
        float *a = (float *)malloc(len_a * sizeof(float));
        float *b = (float *)malloc(len_b * sizeof(float));
        float *ab = NULL, *ba = NULL, *fft_ab = NULL, *ola_ab = NULL;
        size_t lab = 0, lba = 0, lfft = 0, lola = 0;
        size_t i;

        for (i = 0; i < len_a; ++i) a[i] = sm64_sample_f32(&rng, -5.0f, 5.0f);
        for (i = 0; i < len_b; ++i) b[i] = sm64_sample_f32(&rng, -5.0f, 5.0f);

        /* Commutativity: conv(a, b) == conv(b, a) */
        ISO_CHECK(conv_convolve(a, len_a, b, len_b, CONV_MODE_FULL, CONV_BOUND_ZERO, &ab, &lab) == CONV_OK);
        ISO_CHECK(conv_convolve(b, len_b, a, len_a, CONV_MODE_FULL, CONV_BOUND_ZERO, &ba, &lba) == CONV_OK);
        ISO_CHECK_EQ_UINT(lab, lba);
        check_close(ab, ba, lab, 1e-4f);

        /* Cross-algorithm equivalence: direct == FFT == overlap_add */
        ISO_CHECK(conv_convolve_fft(a, len_a, b, len_b, CONV_MODE_FULL, &fft_ab, &lfft) == CONV_OK);
        ISO_CHECK(conv_overlap_add(a, len_a, b, len_b, NULL, CONV_MODE_FULL, &ola_ab, &lola) == CONV_OK);
        ISO_CHECK_EQ_UINT(lab, lfft);
        ISO_CHECK_EQ_UINT(lab, lola);
        check_close(ab, fft_ab, lab, 1e-4f);
        check_close(ab, ola_ab, lab, 1e-4f);

        conv_free_buf(&a, len_a);
        conv_free_buf(&b, len_b);
        conv_free_buf(&ab, lab);
        conv_free_buf(&ba, lba);
        conv_free_buf(&fft_ab, lfft);
        conv_free_buf(&ola_ab, lola);
    }

    /* Linearity: conv(alpha*x + beta*y, k) == alpha*conv(x,k) + beta*conv(y,k) */
    for (iter = 0; iter < 10; ++iter) {
        size_t n = 16u;
        size_t klen = 5u;
        float x[16], y[16], comb[16], k[5];
        float alpha = sm64_sample_f32(&rng, -2.0f, 2.0f);
        float beta = sm64_sample_f32(&rng, -2.0f, 2.0f);
        float *lhs = NULL, *cx = NULL, *cy = NULL;
        size_t ll = 0, lx = 0, ly = 0;
        size_t i;

        for (i = 0; i < n; ++i) {
            x[i] = sm64_sample_f32(&rng, -2.0f, 2.0f);
            y[i] = sm64_sample_f32(&rng, -2.0f, 2.0f);
            comb[i] = alpha * x[i] + beta * y[i];
        }
        for (i = 0; i < klen; ++i) {
            k[i] = sm64_sample_f32(&rng, -1.0f, 1.0f);
        }

        ISO_CHECK(conv_convolve(comb, n, k, klen, CONV_MODE_FULL, CONV_BOUND_ZERO, &lhs, &ll) == CONV_OK);
        ISO_CHECK(conv_convolve(x, n, k, klen, CONV_MODE_FULL, CONV_BOUND_ZERO, &cx, &lx) == CONV_OK);
        ISO_CHECK(conv_convolve(y, n, k, klen, CONV_MODE_FULL, CONV_BOUND_ZERO, &cy, &ly) == CONV_OK);
        for (i = 0; i < ll; ++i) {
            ISO_CHECK(approx_eq(lhs[i], alpha * cx[i] + beta * cy[i], 1e-4f));
        }
        conv_free_buf(&lhs, ll);
        conv_free_buf(&cx, lx);
        conv_free_buf(&cy, ly);
    }
}

int main(void) {
    test_index_and_parsing();
    test_conv1d_and_modes();
    test_fft_and_overlap_add();
    test_2d_and_kernels();
    test_hostile_and_cwe();
    test_pbt_splitmix64_invariants();
    return ISO_TEST_RESULT();
}
