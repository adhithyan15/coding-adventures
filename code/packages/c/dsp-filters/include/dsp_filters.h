/*
 * dsp_filters.h — FIR, IIR, Biquad, SOS, and Butterworth Filters (pure ISO C17).
 * ---------------------------------------------------------------------------
 *
 * Implements:
 * - FIR filtering via direct linear convolution (`dsp_fir`) and FFT
 *   fast convolution (`dsp_fir_fft`), plus windowed-sinc FIR design
 *   (`low_pass`, `high_pass`, `band_pass`, `band_stop`).
 * - Direct-Form-II Transposed IIR filtering (`dsp_iir`, `dsp_filtfilt`).
 * - Failure-atomic stateful `dsp_biquad_t` with Audio EQ Cookbook constructors.
 * - Second-Order Sections (`dsp_sosfilt`, `dsp_sosfiltfilt`).
 * - Bilinear-transform Butterworth filter design (`lowpass`, `highpass`,
 *   `bandpass`, `bandstop`) as both SOS cascades (orders 1..64) and `(b, a)`
 *   polynomials (orders 1..2).
 *
 * Memory & Error Contract.
 * - Allocating routines return a fresh buffer via `*out` / `*out_len`; free
 *   with `dsp_filter_free()`. On any error `*out` is set to NULL and `*out_len`
 *   to 0.
 * - Alias-safe `*_into()` variants allow `out == signal` safely (staging into
 *   an internal buffer and copying out only on success).
 * - All fallible functions return `dsp_filter_error_t` (`DSP_FILTER_OK == 0`).
 *
 * VER15 Hardening:
 * - CWE-20: Upfront enum validation, finite float input & output checks,
 *           a0 != 0 checks, range-checked double -> float narrowing.
 * - CWE-190: Checked size_t arithmetic for convolution, padding, and FFT sizing.
 * - CWE-400: Defensive caps MAX_SAMPLES (2^24), MAX_TAPS (2^20), MAX_ORDER (1024).
 * - CWE-476: Explicit NULL checks on every pointer parameter.
 * - CWE-674: Strictly iterative execution (zero recursion).
 */
#ifndef CA_DSP_FILTERS_H
#define CA_DSP_FILTERS_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Defensive caps (matching DSP03 / dsp-filters.warrant). */
#define DSP_FILTER_MAX_SAMPLES       ((size_t)1u << 24)  /* 16,777,216 */
#define DSP_FILTER_MAX_TAPS          ((size_t)1u << 20)  /* 1,048,576 */
#define DSP_FILTER_MAX_ORDER         ((size_t)1024u)
#define DSP_FILTER_MAX_BUTTER_ORDER  64u

typedef enum {
    DSP_FILTER_OK                      = 0,
    DSP_FILTER_ERR_EMPTY_SIGNAL        = 1,
    DSP_FILTER_ERR_EMPTY_KERNEL        = 2,
    DSP_FILTER_ERR_INVALID_COEFFICIENT = 3,
    DSP_FILTER_ERR_ALLOC               = 4
} dsp_filter_error_t;

typedef enum {
    DSP_WINDOW_RECTANGULAR = 0,
    DSP_WINDOW_HAMMING     = 1,
    DSP_WINDOW_HANN        = 2,
    DSP_WINDOW_BLACKMAN    = 3
} dsp_window_type_t;

typedef enum {
    DSP_BUTTER_LOWPASS  = 0,
    DSP_BUTTER_HIGHPASS = 1,
    DSP_BUTTER_BANDPASS = 2,
    DSP_BUTTER_BANDSTOP = 3
} dsp_butter_type_t;

/* One Second-Order Section: [b0, b1, b2, a0, a1, a2]. */
typedef struct {
    float b0;
    float b1;
    float b2;
    float a0;
    float a1;
    float a2;
} dsp_sos_section_t;

/* Stateful checked Direct-Form-II Transposed biquad. */
typedef struct {
    float b0;
    float b1;
    float b2;
    float a1;
    float a2;
    float z1;
    float z2;
} dsp_biquad_t;

/* Free any heap buffer allocated by this library (NULL-safe). */
void dsp_filter_free(void *p);

/* String parsing helpers (case-insensitive, whitespace-trimmed). */
dsp_filter_error_t dsp_window_parse(const char *str, dsp_window_type_t *out);
dsp_filter_error_t dsp_butter_parse_type(const char *str, dsp_butter_type_t *out);

/* ------------------------------------------------------------------ */
/* FIR Filtering                                                      */
/* ------------------------------------------------------------------ */

dsp_filter_error_t dsp_fir(const float *signal, size_t sig_len,
                           const float *kernel, size_t ker_len,
                           float **out, size_t *out_len);

dsp_filter_error_t dsp_fir_fft(const float *signal, size_t sig_len,
                               const float *kernel, size_t ker_len,
                               float **out, size_t *out_len);

/* ------------------------------------------------------------------ */
/* Windowed-Sinc FIR Design                                           */
/* ------------------------------------------------------------------ */

dsp_filter_error_t dsp_design_low_pass(float cutoff_norm, uint32_t num_taps,
                                       dsp_window_type_t window,
                                       float **out, size_t *out_len);

dsp_filter_error_t dsp_design_high_pass(float cutoff_norm, uint32_t num_taps,
                                        dsp_window_type_t window,
                                        float **out, size_t *out_len);

dsp_filter_error_t dsp_design_band_pass(float low_norm, float high_norm,
                                        uint32_t num_taps, dsp_window_type_t window,
                                        float **out, size_t *out_len);

dsp_filter_error_t dsp_design_band_stop(float low_norm, float high_norm,
                                        uint32_t num_taps, dsp_window_type_t window,
                                        float **out, size_t *out_len);

/* ------------------------------------------------------------------ */
/* Direct & Zero-Phase IIR Filtering                                  */
/* ------------------------------------------------------------------ */

dsp_filter_error_t dsp_iir(const float *signal, size_t sig_len,
                           const float *b, size_t b_len,
                           const float *a, size_t a_len,
                           float **out, size_t *out_len);

dsp_filter_error_t dsp_iir_into(const float *signal, size_t sig_len,
                                const float *b, size_t b_len,
                                const float *a, size_t a_len,
                                float *out);

dsp_filter_error_t dsp_filtfilt(const float *signal, size_t sig_len,
                                const float *b, size_t b_len,
                                const float *a, size_t a_len,
                                float **out, size_t *out_len);

dsp_filter_error_t dsp_filtfilt_into(const float *signal, size_t sig_len,
                                     const float *b, size_t b_len,
                                     const float *a, size_t a_len,
                                     float *out);

/* ------------------------------------------------------------------ */
/* Biquad (Checked DFII-T)                                            */
/* ------------------------------------------------------------------ */

dsp_filter_error_t dsp_biquad_init(dsp_biquad_t *bq,
                                   float b0, float b1, float b2,
                                   float a0, float a1, float a2);

void dsp_biquad_reset(dsp_biquad_t *bq);

dsp_filter_error_t dsp_biquad_process_sample(dsp_biquad_t *bq, float input, float *out);

dsp_filter_error_t dsp_biquad_process(dsp_biquad_t *bq,
                                      const float *signal, size_t sig_len,
                                      float **out, size_t *out_len);

dsp_filter_error_t dsp_biquad_process_into(dsp_biquad_t *bq,
                                           const float *signal, size_t sig_len,
                                           float *out);

/* Audio EQ Cookbook constructors (frequencies normalized in (0.0, 0.5)). */
dsp_filter_error_t dsp_biquad_lowpass(dsp_biquad_t *bq, float cutoff_norm, float q);
dsp_filter_error_t dsp_biquad_highpass(dsp_biquad_t *bq, float cutoff_norm, float q);
dsp_filter_error_t dsp_biquad_bandpass(dsp_biquad_t *bq, float center_norm, float q);
dsp_filter_error_t dsp_biquad_notch(dsp_biquad_t *bq, float center_norm, float q);
dsp_filter_error_t dsp_biquad_allpass(dsp_biquad_t *bq, float center_norm, float q);
dsp_filter_error_t dsp_biquad_peak(dsp_biquad_t *bq, float center_norm, float gain_db, float q);

/* ------------------------------------------------------------------ */
/* Second-Order Sections (SOS) Direct & Zero-Phase Filtering          */
/* ------------------------------------------------------------------ */

dsp_filter_error_t dsp_sosfilt(const dsp_sos_section_t *sections, size_t num_sections,
                               const float *signal, size_t sig_len,
                               float **out, size_t *out_len);

dsp_filter_error_t dsp_sosfilt_into(const dsp_sos_section_t *sections, size_t num_sections,
                                    const float *signal, size_t sig_len,
                                    float *out);

dsp_filter_error_t dsp_sosfiltfilt(const dsp_sos_section_t *sections, size_t num_sections,
                                   const float *signal, size_t sig_len,
                                   float **out, size_t *out_len);

dsp_filter_error_t dsp_sosfiltfilt_into(const dsp_sos_section_t *sections, size_t num_sections,
                                        const float *signal, size_t sig_len,
                                        float *out);

/* ------------------------------------------------------------------ */
/* Butterworth Filter Design (SOS orders 1..64 & (b,a) orders 1..2)   */
/* ------------------------------------------------------------------ */

dsp_filter_error_t dsp_butterworth_sos(dsp_butter_type_t type, uint32_t order,
                                       float low_cutoff_norm, float high_cutoff_norm,
                                       dsp_sos_section_t **out_sections,
                                       size_t *out_num_sections);

dsp_filter_error_t dsp_butterworth_lowpass(uint32_t order, float cutoff_norm,
                                           float **b_out, size_t *b_len,
                                           float **a_out, size_t *a_len);

dsp_filter_error_t dsp_butterworth_highpass(uint32_t order, float cutoff_norm,
                                            float **b_out, size_t *b_len,
                                            float **a_out, size_t *a_len);

dsp_filter_error_t dsp_butterworth_bandpass(uint32_t order, float low_norm, float high_norm,
                                            float **b_out, size_t *b_len,
                                            float **a_out, size_t *a_len);

dsp_filter_error_t dsp_butterworth_bandstop(uint32_t order, float low_norm, float high_norm,
                                            float **b_out, size_t *b_len,
                                            float **a_out, size_t *a_len);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* CA_DSP_FILTERS_H */
