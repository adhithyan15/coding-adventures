/*
 * dsp_dct.h — Discrete Cosine and Sine Transforms (pure ISO C17).
 * ---------------------------------------------------------------------------
 *
 * Implements DCT-I, DCT-II, DCT-III, DCT-IV and DST-I, DST-II, DST-III, DST-IV
 * in 1-D, plus separable 2-D DCT/IDCT and DST/IDST, under both un-normalised
 * (DSP_DCT_NORM_NONE) and orthonormal (DSP_DCT_NORM_ORTHO) conventions.
 *
 * DCT-II and DCT-III use Makhoul's O(N log N) reduction over an internal
 * iterative radix-2 + Bluestein FFT in double precision, avoiding <math.h>
 * and libm completely. DCT-I, DCT-IV, and DST-I..IV use direct double-precision
 * iterative sums.
 *
 * Memory & Error Contract.
 * - Allocating routines return a fresh buffer via `*out` / `*out_len`; free
 *   with `dsp_dct_free()`. On any error `*out` is set to NULL and `*out_len`
 *   to 0 (nothing is leaked).
 * - In-place / caller-buffer `*_into()` routines accept `out == signal`
 *   safely (staging into an internal buffer and copying out only on success).
 * - All fallible functions return `dsp_dct_error_t` (`DSP_DCT_OK == 0`).
 *
 * VER15 Hardening:
 * - CWE-20: Upfront enum validation, finite float input & output checks,
 *           DCT-I/DST-I N >= 2 enforcement, 2-D dimension matching.
 * - CWE-190: Checked size_t arithmetic for 2-D H*W and FFT buffer sizing.
 * - CWE-400: Defensive caps DSP_DCT_MAX_LEN (2^20) and DSP_DCT_MAX_SAMPLES (2^24).
 * - CWE-476: Explicit NULL checks on every pointer parameter.
 * - CWE-674: Strictly iterative execution (zero recursion).
 */
#ifndef CA_DSP_DCT_H
#define CA_DSP_DCT_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Defensive caps (matching DSP02 / dsp-dct.warrant). */
#define DSP_DCT_MAX_LEN     ((size_t)1u << 20)  /* 1,048,576 */
#define DSP_DCT_MAX_SAMPLES ((size_t)1u << 24)  /* 16,777,216 */

typedef enum {
    DSP_DCT_I   = 1,
    DSP_DCT_II  = 2,
    DSP_DCT_III = 3,
    DSP_DCT_IV  = 4
} dsp_dct_type_t;

typedef enum {
    DSP_DST_I   = 1,
    DSP_DST_II  = 2,
    DSP_DST_III = 3,
    DSP_DST_IV  = 4
} dsp_dst_type_t;

typedef enum {
    DSP_DCT_NORM_NONE  = 0,
    DSP_DCT_NORM_ORTHO = 1
} dsp_dct_norm_t;

typedef enum {
    DSP_DCT_OK                = 0,
    DSP_DCT_ERR_EMPTY_INPUT   = 1,
    DSP_DCT_ERR_INVALID_INPUT = 2,
    DSP_DCT_ERR_ALLOC         = 3
} dsp_dct_error_t;

/* Free a heap buffer allocated by this library (NULL-safe). */
void dsp_dct_free(void *p);

/* String parsing helpers (case-insensitive, whitespace-trimmed, no regex). */
dsp_dct_error_t dsp_dct_parse_type(const char *str, dsp_dct_type_t *out);
dsp_dct_error_t dsp_dst_parse_type(const char *str, dsp_dst_type_t *out);
dsp_dct_error_t dsp_dct_parse_norm(const char *str, dsp_dct_norm_t *out);

/* ------------------------------------------------------------------ */
/* 1-D DCT / IDCT (allocating)                                        */
/* ------------------------------------------------------------------ */

dsp_dct_error_t dsp_dct_1d(const float *signal, size_t len,
                           dsp_dct_type_t type, dsp_dct_norm_t norm,
                           float **out, size_t *out_len);

dsp_dct_error_t dsp_idct_1d(const float *coeffs, size_t len,
                            dsp_dct_type_t type, dsp_dct_norm_t norm,
                            float **out, size_t *out_len);

/* ------------------------------------------------------------------ */
/* 1-D DST / IDST (allocating)                                        */
/* ------------------------------------------------------------------ */

dsp_dct_error_t dsp_dst_1d(const float *signal, size_t len,
                           dsp_dst_type_t type, dsp_dct_norm_t norm,
                           float **out, size_t *out_len);

dsp_dct_error_t dsp_idst_1d(const float *coeffs, size_t len,
                            dsp_dst_type_t type, dsp_dct_norm_t norm,
                            float **out, size_t *out_len);

/* ------------------------------------------------------------------ */
/* 1-D Caller-buffer / alias-safe in-place (`out == signal` allowed)  */
/* ------------------------------------------------------------------ */

dsp_dct_error_t dsp_dct_1d_into(const float *signal, size_t len,
                                dsp_dct_type_t type, dsp_dct_norm_t norm,
                                float *out);

dsp_dct_error_t dsp_idct_1d_into(const float *coeffs, size_t len,
                                 dsp_dct_type_t type, dsp_dct_norm_t norm,
                                 float *out);

dsp_dct_error_t dsp_dst_1d_into(const float *signal, size_t len,
                                dsp_dst_type_t type, dsp_dct_norm_t norm,
                                float *out);

dsp_dct_error_t dsp_idst_1d_into(const float *coeffs, size_t len,
                                 dsp_dst_type_t type, dsp_dct_norm_t norm,
                                 float *out);

/* ------------------------------------------------------------------ */
/* 2-D DCT / IDCT and DST / IDST (row-major [height, width])          */
/* ------------------------------------------------------------------ */

dsp_dct_error_t dsp_dct_2d(const float *image, size_t image_len,
                           uint32_t height, uint32_t width,
                           dsp_dct_type_t type, dsp_dct_norm_t norm,
                           float **out, size_t *out_len);

dsp_dct_error_t dsp_idct_2d(const float *coeffs, size_t coeffs_len,
                            uint32_t height, uint32_t width,
                            dsp_dct_type_t type, dsp_dct_norm_t norm,
                            float **out, size_t *out_len);

dsp_dct_error_t dsp_dst_2d(const float *image, size_t image_len,
                           uint32_t height, uint32_t width,
                           dsp_dst_type_t type, dsp_dct_norm_t norm,
                           float **out, size_t *out_len);

dsp_dct_error_t dsp_idst_2d(const float *coeffs, size_t coeffs_len,
                            uint32_t height, uint32_t width,
                            dsp_dst_type_t type, dsp_dct_norm_t norm,
                            float **out, size_t *out_len);

/* Alias-safe 2-D caller-buffer variants (`out == image` allowed). */
dsp_dct_error_t dsp_dct_2d_into(const float *image, size_t image_len,
                                uint32_t height, uint32_t width,
                                dsp_dct_type_t type, dsp_dct_norm_t norm,
                                float *out);

dsp_dct_error_t dsp_idct_2d_into(const float *coeffs, size_t coeffs_len,
                                 uint32_t height, uint32_t width,
                                 dsp_dct_type_t type, dsp_dct_norm_t norm,
                                 float *out);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* CA_DSP_DCT_H */
