/*
 * dsp_conv.h — 1-D, FFT/Overlap-Add, and 2-D Convolution (pure ISO C17).
 * ---------------------------------------------------------------------------
 *
 * Self-contained pure ISO C17 implementation (zero <math.h> / -lm) matching
 * the Rust and TypeScript `dsp-conv` contracts.
 *
 * Provides:
 * - Same-size 1-D convolution (`conv_conv1d`) with Zero/Replicate/Reflect/Wrap.
 * - Full/Same/Valid 1-D convolution (`conv_convolve`) & cross-correlation
 *   (`conv_correlate`).
 * - O(N log N) FFT-based 1-D convolution (`conv_convolve_fft`) & iterative
 *   overlap-add block convolution (`conv_overlap_add`).
 * - Same-size 2-D (`conv_conv2d`), separable 2-D (`conv_sep_conv2d`), and
 *   Full/Same/Valid 2-D convolution (`conv_convolve_2d`).
 * - Image filter kernel generators: Gaussian blur, box blur, Sobel X/Y,
 *   Laplacian, and sharpen.
 *
 * Memory & Zero-on-Free (CWE-415 / CWE-416):
 * - Every fallible routine returns `conv_error_t` (`CONV_OK == 0` on success).
 * - Output pointers are initialized to NULL and left NULL on any error.
 * - Callers free allocated output buffers with `conv_free_buf(&ptr, len)`
 *   (which volatile-zeroes the buffer, frees it, and NULLs `ptr` idempotently)
 *   or `conv_free(ptr)`.
 */
#ifndef DSP_CONV_H
#define DSP_CONV_H

#include <float.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Defensive resource bounds (CWE-400 / CWE-190). */
#define CONV_MAX_SAMPLES 16777216u     /* 2^24 samples (~64 MB of float) */
#define CONV_MAX_KERNEL_LEN 1048576u   /* 2^20 taps */
#define CONV_MAX_FFT_LEN 1048576u      /* 2^20 FFT points */

typedef enum {
    CONV_BOUND_ZERO = 0,
    CONV_BOUND_REPLICATE,
    CONV_BOUND_REFLECT,
    CONV_BOUND_WRAP
} conv_boundary_t;

typedef enum {
    CONV_MODE_FULL = 0,
    CONV_MODE_SAME,
    CONV_MODE_VALID
} conv_mode_t;

typedef enum {
    CONV_OK = 0,
    CONV_ERR_EMPTY_SIGNAL,
    CONV_ERR_EMPTY_KERNEL,
    CONV_ERR_IMAGE_SIZE_MISMATCH,
    CONV_ERR_KERNEL_TOO_LARGE,
    CONV_ERR_INVALID_INPUT,
    CONV_ERR_ALLOC
} conv_error_t;

/* Free a buffer allocated by this library. Safe on NULL. */
void conv_free(void *p);

/* Volatile-zero `len` floats, free `*p`, and set `*p = NULL`.
 * Safe on NULL and idempotent on repeated calls (CWE-415/416). */
void conv_free_buf(float **p, size_t len);

/* Parsing helpers (case-insensitive, ASCII-whitespace-trimmed, bounded <= 64). */
conv_error_t conv_parse_boundary(const char *str, conv_boundary_t *out);
conv_error_t conv_parse_mode(const char *str, conv_mode_t *out);

/* Index extension and bounded sampling.
 * For CONV_BOUND_ZERO out of [0, n), sets *has_value = 0 and *out_idx = 0. */
conv_error_t conv_extend_index(int64_t idx, size_t n, conv_boundary_t mode,
                               int *has_value, size_t *out_idx);
conv_error_t conv_sample(const float *signal, size_t signal_len, int64_t idx,
                         conv_boundary_t mode, float *out);

/* Smallest power of 2 >= n (returns CONV_ERR_INVALID_INPUT on overflow). */
conv_error_t conv_next_pow2(size_t n, size_t *out);

/* 1-D convolution and correlation. */
conv_error_t conv_conv1d(const float *signal, size_t signal_len,
                         const float *kernel, size_t kernel_len,
                         conv_boundary_t mode,
                         float **out, size_t *out_len);

conv_error_t conv_convolve(const float *signal, size_t signal_len,
                           const float *kernel, size_t kernel_len,
                           conv_mode_t mode, conv_boundary_t boundary,
                           float **out, size_t *out_len);

conv_error_t conv_correlate(const float *signal, size_t signal_len,
                            const float *kernel, size_t kernel_len,
                            conv_mode_t mode,
                            float **out, size_t *out_len);

conv_error_t conv_convolve_fft(const float *signal, size_t signal_len,
                               const float *kernel, size_t kernel_len,
                               conv_mode_t mode,
                               float **out, size_t *out_len);

/* Iterative overlap-add 1-D convolution.
 * Pass block_size == NULL for automatic block size (max(64, next_pow2(K))).
 * Passing non-NULL *block_size == 0 is rejected with CONV_ERR_INVALID_INPUT. */
conv_error_t conv_overlap_add(const float *signal, size_t signal_len,
                              const float *kernel, size_t kernel_len,
                              const size_t *block_size, conv_mode_t mode,
                              float **out, size_t *out_len);

/* 2-D and separable 2-D convolution (row-major [H, W]). */
conv_error_t conv_conv2d(const float *image, size_t image_len,
                         const float *kernel, size_t kernel_len,
                         uint32_t image_height, uint32_t image_width,
                         uint32_t kernel_height, uint32_t kernel_width,
                         conv_boundary_t mode,
                         float **out, size_t *out_len);

conv_error_t conv_sep_conv2d(const float *image, size_t image_len,
                             const float *h_kernel, size_t h_len,
                             const float *v_kernel, size_t v_len,
                             uint32_t image_height, uint32_t image_width,
                             conv_boundary_t mode,
                             float **out, size_t *out_len);

conv_error_t conv_convolve_2d(const float *image, size_t image_len,
                              const float *kernel, size_t kernel_len,
                              uint32_t image_height, uint32_t image_width,
                              uint32_t kernel_height, uint32_t kernel_width,
                              conv_mode_t mode, conv_boundary_t boundary,
                              float **out, size_t *out_len);

/* Image filter kernel design helpers. */
conv_error_t conv_gaussian_blur_kernel(float sigma, uint32_t size,
                                       float **out, size_t *out_len);
conv_error_t conv_box_blur_kernel(uint32_t size,
                                  float **out, size_t *out_len);
conv_error_t conv_sobel_x_kernel(float **out, size_t *out_len);
conv_error_t conv_sobel_y_kernel(float **out, size_t *out_len);
conv_error_t conv_laplacian_kernel(float **out, size_t *out_len);
conv_error_t conv_sharpen_kernel(float amount, float **out, size_t *out_len);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* DSP_CONV_H */
