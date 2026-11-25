/*
 * dsp_complex.h — Interleaved Complex Tensor & Scalar Operations (pure ISO C17).
 * ============================================================================
 *
 * A faithful C17 port of the `dsp-complex` crate (DSP00 / DSP01). Represents
 * a 1-D sequence of `N` complex `float` elements as an interleaved array of
 * length `2 * N`:
 *
 *     [re_0, im_0, re_1, im_1, ..., re_{N-1}, im_{N-1}]
 *
 * Pure ISO C17 with zero `<math.h>` / `-lm` dependency so `iso-harness` builds
 * cleanly across GCC, Clang, and MSVC without extra linker flags.
 *
 * Hardened against VER15 CWEs:
 * - CWE-20:  Finite-number validation (rejects NaN, +Inf, -Inf) on all inputs,
 *            even-length checks on interleaved buffers, non-zero divisor guard.
 * - CWE-120/125/787: Bounds-checked element access (`dsp_complex_tensor_get/set`).
 * - CWE-190: Overflow-checked `size_t` arithmetic (`2 * n`, byte sizing).
 * - CWE-400: Pre-flight `DSP_COMPLEX_MAX_SAMPLES` (2^24) cap before malloc.
 * - CWE-415/416: Zero-on-free and idempotent cleanup (`dsp_complex_tensor_free`,
 *            `dsp_complex_free_buffer`), leaving freed structs zeroed.
 * - CWE-476: Defensive NULL guards on all public entry points.
 */
#ifndef DSP_COMPLEX_H
#define DSP_COMPLEX_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Maximum number of complex samples in a tensor (2^24 = 16,777,216). */
#define DSP_COMPLEX_MAX_SAMPLES ((size_t)16777216u)

/* Status codes returned by all fallible dsp-complex routines. */
typedef enum {
    DSP_COMPLEX_OK = 0,
    DSP_COMPLEX_ERR_INVALID_INPUT,
    DSP_COMPLEX_ERR_LENGTH_MISMATCH,
    DSP_COMPLEX_ERR_ODD_INTERLEAVED_LENGTH,
    DSP_COMPLEX_ERR_OUT_OF_RANGE,
    DSP_COMPLEX_ERR_ALLOC
} dsp_complex_error_t;

/* Scalar single-precision complex number. */
typedef struct {
    float re;
    float im;
} dsp_complex_t;

/* Owned interleaved complex tensor of `len` complex elements (`2 * len` floats).
 * Always initialize to `{NULL, 0}` and release with `dsp_complex_tensor_free`. */
typedef struct {
    float *data;
    size_t len;
} dsp_complex_tensor_t;

/* ── Diagnostics & checked helpers ───────────────────────────────────────── */

/* Human-readable error label (never NULL). */
const char *dsp_complex_error_str(dsp_complex_error_t err);

/* Pure IEEE-754 finiteness check (returns 1 if finite, 0 for NaN or +/-Inf). */
int dsp_complex_is_finite(float x);

/* Checked size_t arithmetic (returns 1 on success, 0 on overflow or NULL out). */
int dsp_complex_checked_add(size_t a, size_t b, size_t *out);
int dsp_complex_checked_mul(size_t a, size_t b, size_t *out);

/* ── Scalar complex operations ───────────────────────────────────────────── */

dsp_complex_t dsp_c_make(float re, float im);
dsp_complex_t dsp_c_add(dsp_complex_t a, dsp_complex_t b);
dsp_complex_t dsp_c_sub(dsp_complex_t a, dsp_complex_t b);
dsp_complex_t dsp_c_mul(dsp_complex_t a, dsp_complex_t b);
dsp_complex_t dsp_c_conj(dsp_complex_t a);
float dsp_c_abs(dsp_complex_t a);
float dsp_c_arg(dsp_complex_t a);

/* ── Tensor lifecycle & constructors ─────────────────────────────────────── */

/* Allocate a zero-initialized tensor of `n` complex elements. */
dsp_complex_error_t dsp_complex_tensor_zeros(size_t n, dsp_complex_tensor_t *out);

/* Construct from `n` real samples (imag = 0). `real` may be NULL iff `n == 0`. */
dsp_complex_error_t dsp_complex_tensor_from_real(const float *real, size_t n,
                                                 dsp_complex_tensor_t *out);

/* Construct from separate `real` and `imag` arrays. Rejects mismatched lengths
 * with `DSP_COMPLEX_ERR_LENGTH_MISMATCH`. */
dsp_complex_error_t dsp_complex_tensor_from_real_imag(const float *real,
                                                      size_t real_len,
                                                      const float *imag,
                                                      size_t imag_len,
                                                      dsp_complex_tensor_t *out);

/* Construct from an interleaved `[re_0, im_0, ...]` buffer of `float_len` floats.
 * Rejects odd `float_len` with `DSP_COMPLEX_ERR_ODD_INTERLEAVED_LENGTH`. */
dsp_complex_error_t dsp_complex_tensor_from_interleaved(const float *data,
                                                        size_t float_len,
                                                        dsp_complex_tensor_t *out);

/* Deep copy an existing tensor. */
dsp_complex_error_t dsp_complex_tensor_clone(const dsp_complex_tensor_t *src,
                                             dsp_complex_tensor_t *out);

/* Zero-on-free: wipes owned sample bytes, frees memory, and zeroes `*tensor`.
 * Safe on NULL and idempotent on repeated calls (CWE-415/416). */
void dsp_complex_tensor_free(dsp_complex_tensor_t *tensor);

/* Zero-on-free for float buffers returned by projection functions. */
void dsp_complex_free_buffer(float *buf, size_t float_len);

/* Convenience free matching `wv_free` style (does not wipe length). */
void dsp_complex_free(void *p);

/* ── Accessors & bounds-checked indexing ─────────────────────────────────── */

size_t dsp_complex_tensor_len(const dsp_complex_tensor_t *t);
int dsp_complex_tensor_is_empty(const dsp_complex_tensor_t *t);
const float *dsp_complex_tensor_as_interleaved(const dsp_complex_tensor_t *t);

dsp_complex_error_t dsp_complex_tensor_get(const dsp_complex_tensor_t *t,
                                           size_t idx, dsp_complex_t *out);
dsp_complex_error_t dsp_complex_tensor_set(dsp_complex_tensor_t *t, size_t idx,
                                           dsp_complex_t val);

/* ── Projections & element-wise tensor arithmetic ────────────────────────── */

/* Extract real, imag, magnitude, or phase components into a newly allocated
 * float buffer of length `t->len` (`*out == NULL` and `*out_len == 0` when empty).
 * Free non-NULL results with `dsp_complex_free_buffer(*out, *out_len)`. */
dsp_complex_error_t dsp_complex_tensor_real(const dsp_complex_tensor_t *t,
                                            float **out, size_t *out_len);
dsp_complex_error_t dsp_complex_tensor_imag(const dsp_complex_tensor_t *t,
                                            float **out, size_t *out_len);
dsp_complex_error_t dsp_complex_tensor_magnitude(const dsp_complex_tensor_t *t,
                                                 float **out, size_t *out_len);
dsp_complex_error_t dsp_complex_tensor_phase(const dsp_complex_tensor_t *t,
                                             float **out, size_t *out_len);

dsp_complex_error_t dsp_complex_tensor_conjugate(const dsp_complex_tensor_t *t,
                                                 dsp_complex_tensor_t *out);
dsp_complex_error_t dsp_complex_tensor_scale(const dsp_complex_tensor_t *t,
                                             float factor,
                                             dsp_complex_tensor_t *out);
dsp_complex_error_t dsp_complex_tensor_add(const dsp_complex_tensor_t *a,
                                           const dsp_complex_tensor_t *b,
                                           dsp_complex_tensor_t *out);
dsp_complex_error_t dsp_complex_tensor_sub(const dsp_complex_tensor_t *a,
                                           const dsp_complex_tensor_t *b,
                                           dsp_complex_tensor_t *out);
dsp_complex_error_t dsp_complex_tensor_mul(const dsp_complex_tensor_t *a,
                                           const dsp_complex_tensor_t *b,
                                           dsp_complex_tensor_t *out);
dsp_complex_error_t dsp_complex_tensor_div(const dsp_complex_tensor_t *a,
                                           const dsp_complex_tensor_t *b,
                                           dsp_complex_tensor_t *out);

#ifdef __cplusplus
}
#endif

#endif /* DSP_COMPLEX_H */
