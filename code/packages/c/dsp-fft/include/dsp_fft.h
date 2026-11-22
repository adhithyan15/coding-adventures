/*
 * dsp_fft.h — Iterative 1-D FFT / IFFT / RFFT / IRFFT (pure ISO C17).
 * ============================================================================
 *
 * A faithful C17 port of the `dsp-fft` crate (DSP01). Computes forward and
 * inverse 1-D Discrete Fourier Transforms on interleaved `[re, im]` single-
 * precision `float` buffers using:
 *
 * - Decimation-in-time radix-2 Cooley-Tukey for power-of-two lengths `N = 2^k`.
 * - Bluestein's chirp-Z convolution algorithm for arbitrary `N >= 1`.
 * - Half-spectrum `rfft` / `irfft` (`floor(N/2) + 1` bins) for real signals.
 *
 * Normalization convention ("backward", matching numpy / scipy / Rust `dsp-fft`):
 * - Forward transform scale: `1`
 * - Inverse transform scale: `1 / N`
 *
 * Pure ISO C17 with zero `<math.h>` / `-lm` dependency.
 *
 * Hardened against VER15 CWEs:
 * - CWE-20:  Rejects NaN, +Inf, -Inf, zero length, odd interleaved buffers,
 *            mismatched `irfft` bin counts, and non-pow2 sizes on `*_scalar`.
 * - CWE-190: Overflow-checked `size_t` arithmetic on all sizes & strides.
 * - CWE-400: Pre-flight `DSP_FFT_MAX_LEN` (2^20) & `DSP_FFT_MAX_SAMPLES` (2^24).
 * - CWE-415/416: Zero-on-free buffer cleanup (`dsp_fft_free_buffer`) and safe
 *            tensor disposal via `dsp_complex_tensor_free`.
 * - CWE-476: Defensive NULL pointer checks across all public entry points.
 * - CWE-674: Strictly iterative execution (zero recursion).
 */
#ifndef DSP_FFT_H
#define DSP_FFT_H

#include "dsp_complex.h"

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Maximum 1-D FFT length (2^20 = 1,048,576 complex elements). */
#define DSP_FFT_MAX_LEN ((size_t)1048576u)

/* Maximum float sample count across any interleaved buffer (2^24 = 16,777,216). */
#define DSP_FFT_MAX_SAMPLES ((size_t)16777216u)

/* Status codes returned by all fallible dsp-fft routines. */
typedef enum {
    DSP_FFT_OK = 0,
    DSP_FFT_ERR_INVALID_INPUT,
    DSP_FFT_ERR_NOT_POWER_OF_TWO,
    DSP_FFT_ERR_ALLOC
} dsp_fft_error_t;

/* Transform direction for Bluestein / internal butterflies. */
typedef enum {
    DSP_FFT_FORWARD = 0,
    DSP_FFT_INVERSE = 1
} dsp_fft_direction_t;

/* ── Diagnostics & cleanup ───────────────────────────────────────────────── */

const char *dsp_fft_error_str(dsp_fft_error_t err);
int dsp_fft_is_power_of_two(size_t n);

/* Zero-on-free for heap buffers returned by scalar/irfft routines. */
void dsp_fft_free_buffer(float *buf, size_t float_len);
void dsp_fft_free(void *p);

/* ── Low-level interleaved scalar routines ───────────────────────────────── */

/* Forward radix-2 FFT on interleaved `[re, im]` buffer (`float_len == 2 * N`,
 * `N` power of two). Rejects non-power-of-two with `DSP_FFT_ERR_NOT_POWER_OF_TWO`. */
dsp_fft_error_t dsp_fft_scalar(const float *signal, size_t float_len,
                               float **out, size_t *out_len);

/* Inverse radix-2 FFT with `1/N` backward normalization. */
dsp_fft_error_t dsp_ifft_scalar(const float *spectrum, size_t float_len,
                                float **out, size_t *out_len);

/* Arbitrary-N Bluestein FFT/IFFT on interleaved `[re, im]` buffer (`N >= 1`). */
dsp_fft_error_t dsp_bluestein_scalar(const float *signal, size_t float_len,
                                     dsp_fft_direction_t dir,
                                     float **out, size_t *out_len);

/* Real-input forward FFT returning interleaved half-spectrum (`2 * (N/2 + 1)` floats). */
dsp_fft_error_t dsp_rfft_scalar(const float *signal, size_t signal_len,
                                float **out, size_t *out_len);

/* Inverse real-input FFT taking `2 * (output_length/2 + 1)` interleaved floats
 * and returning `output_length` real floats. */
dsp_fft_error_t dsp_irfft_scalar(const float *half_spectrum, size_t float_len,
                                 uint32_t output_length,
                                 float **out, size_t *out_len);

/* ── High-level ComplexTensor API ────────────────────────────────────────── */

/* Compute forward 1-D FFT for arbitrary `N >= 1` (radix-2 if power of two,
 * Bluestein otherwise). If `is_complex == 0`, `signal` holds `len` real floats;
 * if `is_complex != 0`, `signal` holds `len` interleaved floats (`2 * N`). */
dsp_fft_error_t dsp_fft(const float *signal, size_t len, int is_complex,
                        dsp_complex_tensor_t *out);

/* Compute inverse 1-D FFT for arbitrary `N >= 1` with `1/N` scaling. */
dsp_fft_error_t dsp_ifft(const dsp_complex_tensor_t *spectrum,
                         dsp_complex_tensor_t *out);

/* Compute real-input half-spectrum FFT (`floor(len/2) + 1` complex bins). */
dsp_fft_error_t dsp_rfft(const float *signal, size_t len,
                         dsp_complex_tensor_t *out);

/* Compute inverse real-input FFT from half-spectrum tensor to real `float*` buffer. */
dsp_fft_error_t dsp_irfft(const dsp_complex_tensor_t *half_spectrum,
                          uint32_t output_length,
                          float **out, size_t *out_len);

#ifdef __cplusplus
}
#endif

#endif /* DSP_FFT_H */
