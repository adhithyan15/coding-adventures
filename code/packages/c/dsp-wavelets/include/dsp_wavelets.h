/*
 * dsp_wavelets.h — Discrete Wavelet Transforms (pure ISO C17).
 * ---------------------------------------------------------------------------
 *
 * A faithful C port of the Rust `dsp-wavelets` crate.  It implements the
 * Discrete Wavelet Transform (DWT) and its inverse via the Mallat pyramid
 * algorithm, in one and two dimensions, for the orthogonal wavelet families
 * Haar, Daubechies (Db2/4/6/8), Symlets (Sym4), and Coiflets (Coif1).
 *
 * A wavelet transform decomposes a signal into scale-and-position-localised
 * basis functions.  One forward step is two FIR filter passes — a lowpass `h`
 * and a highpass `g` — followed by downsample-by-2:
 *
 *     x[n] ─┬─ lowpass  h ─► ↓2 → cA   (approximation, half length)
 *           └─ highpass g ─► ↓2 → cD   (detail,         half length)
 *
 * `levels` of DWT applies the same pair recursively to `cA`, giving the
 * flattened layout  [cA_J | cD_J | cD_{J-1} | ... | cD_1].
 *
 * Memory.  Transform routines allocate their result with malloc and return it
 * through an out-pointer + length; the caller frees it with wv_free().  On any
 * error the out-pointer is left NULL and nothing is allocated.  wv_slice_level
 * instead returns a *borrowed* pointer into the caller's coefficient buffer
 * (do not free it).
 *
 * Errors.  Every fallible routine returns a wv_error_t (WV_OK == 0 on success).
 * The Rust variants carry a human-readable message; here only the discriminant
 * is returned.
 */
#ifndef DSP_WAVELETS_H
#define DSP_WAVELETS_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* The reciprocal of sqrt(2) as a float — the Haar filter tap.  Hard-coded (no
 * <math.h>) so the lane stays pure-ISO; the literal rounds to the same float
 * as Rust's std::f32::consts::FRAC_1_SQRT_2. */
#define WV_FRAC_1_SQRT_2 0.70710678f

/* Defensive caps (mirror the crate's security-review bounds). */
#define WV_MAX_LEVELS 31u
#define WV_MAX_SAMPLES (1u << 30)

/* ------------------------------------------------------------------ */
/* Wavelet family / boundary / band selectors                         */
/* ------------------------------------------------------------------ */

typedef enum {
    WV_HAAR = 0,
    WV_DAUBECHIES,
    WV_SYMLETS,
    WV_COIFLETS,
    WV_BIORTHOGONAL,
    WV_MORLET,
    WV_MEXICAN_HAT
} wv_family_t;

/* A wavelet selector.  `n` is the family order (e.g. Daubechies(4)); `n2` is
 * only meaningful for Biorthogonal (the reconstruction order). */
typedef struct {
    wv_family_t family;
    uint32_t n;
    uint32_t n2;
} wv_wavelet_t;

/* Constructors. */
wv_wavelet_t wv_haar(void);
wv_wavelet_t wv_daubechies(uint32_t n);
wv_wavelet_t wv_symlets(uint32_t n);
wv_wavelet_t wv_coiflets(uint32_t n);
wv_wavelet_t wv_biorthogonal(uint32_t vm_decomp, uint32_t vm_recon);
wv_wavelet_t wv_morlet(void);
wv_wavelet_t wv_mexican_hat(void);

typedef enum {
    WV_BOUND_ZERO = 0,
    WV_BOUND_REPLICATE,
    WV_BOUND_REFLECT,
    WV_BOUND_SYMMETRIC,
    WV_BOUND_PERIODIC
} wv_boundary_t;

typedef enum { WV_BAND_APPROXIMATION = 0, WV_BAND_DETAIL } wv_band_t;

/* ------------------------------------------------------------------ */
/* Error codes                                                        */
/* ------------------------------------------------------------------ */

typedef enum {
    WV_OK = 0,                    /* success sentinel (not in Rust) */
    WV_ERR_EMPTY_SIGNAL,          /* WaveletError::EmptySignal */
    WV_ERR_INVALID_PARAM,         /* WaveletError::InvalidParam */
    WV_ERR_SIGNAL_TOO_SHORT,      /* WaveletError::SignalTooShort */
    WV_ERR_INVALID_COEFFICIENTS,  /* WaveletError::InvalidCoefficients */
    WV_ERR_FFT,                   /* WaveletError::Fft (reserved) */
    WV_ERR_ALLOC                  /* out-of-memory (no Rust equivalent) */
} wv_error_t;

/* Free a buffer returned by a transform routine (float* or size_t*). */
void wv_free(void *p);

/* ------------------------------------------------------------------ */
/* Filter tables                                                      */
/* ------------------------------------------------------------------ */

/* Return a borrowed pointer to the analysis lowpass filter `h` for `wavelet`,
 * setting *out_len to its length.  Returns NULL with *out_len == 0 for any
 * (family, order) pair not shipped here (Haar included — it is hard-coded in
 * the transform as the canonical worked example).  Do NOT free the result. */
const float *wv_analysis_lowpass(wv_wavelet_t wavelet, size_t *out_len);

/* QMF-derive the analysis highpass g[i] = (-1)^i * h[L-1-i].  Allocates the
 * result (free with wv_free). */
wv_error_t wv_qmf_highpass(const float *h, size_t h_len, float **out, size_t *out_len);

/* ------------------------------------------------------------------ */
/* 1-D API                                                            */
/* ------------------------------------------------------------------ */

/* Forward 1-D DWT.  Output layout: [cA_J | cD_J | ... | cD_1]. */
wv_error_t wv_dwt_1d(const float *signal, size_t signal_len, wv_wavelet_t wavelet,
                     uint32_t levels, wv_boundary_t boundary, float **out, size_t *out_len);

/* Inverse 1-D DWT.  `output_length` recovers the parity bit lost to
 * downsampling — pass the original signal length. */
wv_error_t wv_idwt_1d(const float *coeffs, size_t coeffs_len, wv_wavelet_t wavelet,
                      uint32_t levels, wv_boundary_t boundary, uint32_t output_length,
                      float **out, size_t *out_len);

/* Per-band offsets in a flattened dwt_1d buffer:
 *   [offset_cA_J, offset_cD_J, ..., offset_cD_1, total_len]   (length levels+2).
 * Allocates a size_t array (free with wv_free). */
wv_error_t wv_split_levels(size_t coeffs_len, size_t signal_len, uint32_t levels,
                           size_t **out, size_t *out_len);

/* Borrowed view of the (target_level, band) slice within `coeffs`.  *out points
 * INTO coeffs (do not free); *out_len is the slice length. */
wv_error_t wv_slice_level(const float *coeffs, size_t coeffs_len, size_t signal_len,
                          uint32_t levels, uint32_t target_level, wv_band_t band,
                          const float **out, size_t *out_len);

/* ------------------------------------------------------------------ */
/* 2-D API                                                            */
/* ------------------------------------------------------------------ */

/* Forward 2-D DWT (separable row-then-column).  `image` is row-major
 * [n_rows, n_cols].  Output layout:
 *   [LL_J | HL_J | LH_J | HH_J | HL_{J-1} | LH_{J-1} | HH_{J-1} | ... ]. */
wv_error_t wv_dwt_2d(const float *image, size_t image_len, uint32_t n_rows, uint32_t n_cols,
                     wv_wavelet_t wavelet, uint32_t levels, wv_boundary_t boundary,
                     float **out, size_t *out_len);

/* Inverse 2-D DWT — reverses wv_dwt_2d. */
wv_error_t wv_idwt_2d(const float *coeffs, size_t coeffs_len, uint32_t n_rows, uint32_t n_cols,
                      wv_wavelet_t wavelet, uint32_t levels, wv_boundary_t boundary,
                      float **out, size_t *out_len);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* DSP_WAVELETS_H */
