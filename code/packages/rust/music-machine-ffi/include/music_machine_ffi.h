/*
 * music_machine_ffi.h
 *
 * C ABI bindings for music-machine-core.
 * Confined ABI ensuring Family-1 memory safety.
 */

#ifndef MUSIC_MACHINE_FFI_H
#define MUSIC_MACHINE_FFI_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Status codes */
#define MM_STATUS_OK 0
#define MM_STATUS_NULL_POINTER 1
#define MM_STATUS_INVALID_ARGUMENT 2
#define MM_STATUS_BUDGET_EXCEEDED 3
#define MM_STATUS_PARSE_ERROR 4
#define MM_STATUS_RENDER_ERROR 5
#define MM_STATUS_PANIC 6

/**
 * Render a beginner text score to signed 16-bit mono PCM audio.
 *
 * @param text_ptr Pointer to UTF-8 score text bytes.
 * @param text_len Length of score text in bytes (must be <= 1,000,000).
 * @param sample_rate_hz Override sample rate in Hz (0 uses score's declared rate).
 * @param out_samples_ptr Receives pointer to dynamically allocated PCM i16 buffer. Caller must free via mm_free_buffer.
 * @param out_samples_len Receives number of i16 samples in output buffer.
 * @param out_clipped_count Receives count of saturated/clipped samples.
 * @return Status code (MM_STATUS_OK on success).
 */
int32_t mm_render_score_text_to_pcm16(
    const uint8_t *text_ptr,
    size_t text_len,
    uint32_t sample_rate_hz,
    int16_t **out_samples_ptr,
    size_t *out_samples_len,
    size_t *out_clipped_count
);

/**
 * Render a canonical JSON score to signed 16-bit mono PCM audio.
 *
 * @param json_ptr Pointer to UTF-8 JSON text bytes.
 * @param json_len Length of JSON text in bytes (must be <= 1,000,000).
 * @param max_samples Maximum allowable rendered sample count (0 uses default 10,000,000).
 * @param out_samples_ptr Receives pointer to dynamically allocated PCM i16 buffer. Caller must free via mm_free_buffer.
 * @param out_samples_len Receives number of i16 samples in output buffer.
 * @param out_clipped_count Receives count of saturated/clipped samples.
 * @return Status code (MM_STATUS_OK on success).
 */
int32_t mm_render_portable_json_to_pcm16(
    const uint8_t *json_ptr,
    size_t json_len,
    size_t max_samples,
    int16_t **out_samples_ptr,
    size_t *out_samples_len,
    size_t *out_clipped_count
);

/**
 * Render a canonical V2 portable text score to signed 16-bit mono PCM audio.
 *
 * @param text_ptr Pointer to UTF-8 portable score text bytes.
 * @param text_len Length of score text in bytes (must be <= 1,000,000).
 * @param max_samples Maximum allowable rendered sample count (0 uses default 10,000,000).
 * @param out_samples_ptr Receives pointer to dynamically allocated PCM i16 buffer. Caller must free via mm_free_buffer.
 * @param out_samples_len Receives number of i16 samples in output buffer.
 * @param out_clipped_count Receives count of saturated/clipped samples.
 * @return Status code (MM_STATUS_OK on success).
 */
int32_t mm_render_portable_text_to_pcm16(
    const uint8_t *text_ptr,
    size_t text_len,
    size_t max_samples,
    int16_t **out_samples_ptr,
    size_t *out_samples_len,
    size_t *out_clipped_count
);

/**
 * Free a PCM sample buffer allocated by one of the render functions.
 * Passing NULL or len == 0 is a safe no-op.
 *
 * @param ptr Pointer to i16 buffer to deallocate.
 * @param len Number of i16 elements originally reported in out_samples_len.
 */
void mm_free_buffer(
    int16_t *ptr,
    size_t len
);

#ifdef __cplusplus
}
#endif

#endif /* MUSIC_MACHINE_FFI_H */
