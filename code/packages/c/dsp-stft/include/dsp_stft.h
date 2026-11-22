/*
 * dsp_stft.h — Short-Time Fourier Transform, ISTFT, Spectrograms, Mel & MFCC
 *              (pure ISO C17).
 * ---------------------------------------------------------------------------
 *
 * Self-contained pure ISO C17 implementation (zero <math.h> / -lm) matching
 * the Rust and TypeScript `dsp-stft` contracts, with internal O(N log N)
 * radix-2 + Bluestein FFT and internal O(N log N) Makhoul DCT-II.
 *
 * Provides:
 * - Analysis windows: Hann, Hamming, Blackman, Rectangular + strict parsing.
 * - STFT (`stft_forward`, strict-mode framing `[num_frames, n_fft/2 + 1, 2]`).
 * - ISTFT (`stft_istft`, overlap-add with NOLA normalization), OLA/COLA check
 *   (`stft_check_ola`), and iterative Griffin-Lim phase reconstruction
 *   (`stft_griffin_lim`).
 * - Spectrograms: `stft_magnitude_spectrogram`, `stft_power_spectrogram`,
 *   `stft_spectrogram`, `stft_log_spectrogram`, `stft_log_spectrogram_eps`.
 * - Speech features: `stft_hz_to_mel`, `stft_mel_to_hz`, `stft_mel_filterbank`,
 *   `stft_mel_spectrogram`, `stft_mfcc`.
 *
 * Memory & Zero-on-Free (CWE-415 / CWE-416):
 * - Every fallible routine returns `stft_error_t` (`STFT_OK == 0` on success).
 * - Output pointers are initialized to NULL and left NULL on any error.
 * - Free with `stft_free_buf(&ptr, len)` (volatile-zeroes, frees, sets NULL)
 *   or `stft_free(ptr)`.
 */
#ifndef DSP_STFT_H
#define DSP_STFT_H

#include <float.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Defensive resource bounds (CWE-400 / CWE-190). */
#define STFT_MAX_FFT_LEN 1048576u          /* 2^20 */
#define STFT_MAX_SAMPLES 16777216u         /* 2^24 */
#define STFT_MAX_FRAMES 1048576u           /* 2^20 */
#define STFT_MAX_MELS 65536u               /* 2^16 */
#define STFT_MAX_GRIFFIN_LIM_ITERS 4096u   /* bounded iterative loop */

typedef enum {
    STFT_WIN_HANN = 0,
    STFT_WIN_HAMMING,
    STFT_WIN_BLACKMAN,
    STFT_WIN_RECTANGULAR
} stft_window_t;

typedef enum {
    STFT_OK = 0,
    STFT_ERR_EMPTY_SIGNAL,
    STFT_ERR_INVALID_PARAM,
    STFT_ERR_SIGNAL_TOO_SHORT,
    STFT_ERR_INVALID_SPECTROGRAM,
    STFT_ERR_FFT,
    STFT_ERR_ALLOC
} stft_error_t;

/* Memory management. */
void stft_free(void *p);
void stft_free_buf(float **p, size_t len);

/* Window parsing and generation. */
stft_error_t stft_parse_window(const char *str, stft_window_t *out);
stft_error_t stft_build_window(stft_window_t window, uint32_t n_fft,
                               float **out, size_t *out_len);
stft_error_t stft_hann_window(uint32_t length, float **out, size_t *out_len);
stft_error_t stft_hamming_window(uint32_t length, float **out, size_t *out_len);
stft_error_t stft_blackman_window(uint32_t length, float **out, size_t *out_len);
stft_error_t stft_rectangular_window(uint32_t length, float **out, size_t *out_len);

/* Forward STFT (strict framing: num_frames = 1 + (N - n_fft) / hop_length).
 * Output layout: row-major [num_frames, n_fft/2 + 1, 2] interleaved [re, im]. */
stft_error_t stft_forward(const float *signal, size_t signal_len,
                          uint32_t n_fft, uint32_t hop_length,
                          stft_window_t window,
                          float **out, size_t *out_len);

/* Inverse STFT via weighted overlap-add. */
stft_error_t stft_istft(const float *spectrogram, size_t spec_len,
                        uint32_t n_fft, uint32_t hop_length,
                        stft_window_t window, uint32_t output_length,
                        float **out, size_t *out_len);

/* Check NOLA (non-zero overlap-add) and COLA (constant squared-window overlap-add)
 * over the steady-state interior for (n_fft, hop_length, window). */
stft_error_t stft_check_ola(uint32_t n_fft, uint32_t hop_length,
                            stft_window_t window,
                            int *out_is_nola, int *out_is_cola,
                            float *out_min_norm, float *out_max_norm);

/* Iterative Griffin-Lim phase reconstruction from a magnitude spectrogram
 * of shape [num_frames, n_fft/2 + 1]. */
stft_error_t stft_griffin_lim(const float *mag_spec, size_t mag_len,
                              uint32_t n_fft, uint32_t hop_length,
                              stft_window_t window, uint32_t n_iter,
                              uint32_t output_length,
                              float **out, size_t *out_len);

/* Spectrograms: magnitude |STFT|, power |STFT|^2, and log(|STFT|^2 + eps). */
stft_error_t stft_magnitude_spectrogram(const float *signal, size_t signal_len,
                                        uint32_t n_fft, uint32_t hop_length,
                                        stft_window_t window,
                                        float **out, size_t *out_len);
stft_error_t stft_power_spectrogram(const float *signal, size_t signal_len,
                                    uint32_t n_fft, uint32_t hop_length,
                                    stft_window_t window,
                                    float **out, size_t *out_len);
stft_error_t stft_spectrogram(const float *signal, size_t signal_len,
                              uint32_t n_fft, uint32_t hop_length,
                              stft_window_t window,
                              float **out, size_t *out_len);
stft_error_t stft_log_spectrogram(const float *signal, size_t signal_len,
                                  uint32_t n_fft, uint32_t hop_length,
                                  stft_window_t window,
                                  float **out, size_t *out_len);
stft_error_t stft_log_spectrogram_eps(const float *signal, size_t signal_len,
                                      uint32_t n_fft, uint32_t hop_length,
                                      stft_window_t window, float eps,
                                      float **out, size_t *out_len);

/* HTK Mel scale, triangular Mel filterbank, Mel spectrogram, and MFCC. */
stft_error_t stft_hz_to_mel(float f_hz, float *out_mel);
stft_error_t stft_mel_to_hz(float mel, float *out_hz);
stft_error_t stft_mel_filterbank(uint32_t n_mels, uint32_t n_fft,
                                 float sample_rate,
                                 float **out, size_t *out_len);
stft_error_t stft_mel_spectrogram(const float *signal, size_t signal_len,
                                  uint32_t n_fft, uint32_t hop_length,
                                  uint32_t n_mels, float sample_rate,
                                  stft_window_t window,
                                  float **out, size_t *out_len);
stft_error_t stft_mfcc(const float *signal, size_t signal_len,
                       uint32_t n_fft, uint32_t hop_length,
                       uint32_t n_mels, uint32_t n_mfcc,
                       float sample_rate, stft_window_t window,
                       float **out, size_t *out_len);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* DSP_STFT_H */
