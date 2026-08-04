/*
 * audio_device_sink.h — backend-neutral PCM playback primitives, ISO C17.
 * ======================================================================
 *
 * A faithful port of the Rust `audio-device-sink` crate. Intentionally boring:
 * it does NOT open devices, talk to Core Audio, parse notes, or generate waves.
 * It defines the shared contract every real audio backend must obey — a
 * validated PCM format, an owned playback buffer, a playback report, and a
 * sink "trait" (here a small vtable), with a no-op sink for tests.
 *
 * V1 scope (matching the Rust crate): mono, signed 16-bit PCM only.
 *
 * Rust's `Result<T, AudioSinkError>` (whose error carries a formatted message)
 * becomes a status code plus an optional caller-provided message buffer. Owned
 * samples (`Vec<i16>`) become a malloc'd `int16_t` buffer paired with a free.
 *
 * Pure ISO C17: no <math.h>, no compiler extensions.
 */
#ifndef AUDIO_DEVICE_SINK_H
#define AUDIO_DEVICE_SINK_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint16_t, uint32_t, int16_t */

#ifdef __cplusplus
extern "C" {
#endif

/* ── Constants ─────────────────────────────────────────────────────────────*/

/* Crate version string. */
extern const char *const ADS_VERSION;
/* V1 keeps buffers small enough that accidental calls don't block for hours. */
#define ADS_MAX_BLOCKING_DURATION_SECONDS (10.0 * 60.0)
/* Practical ceiling for high-resolution audio interfaces. */
#define ADS_MAX_SAMPLE_RATE_HZ ((uint32_t)384000)
/* Only signed 16-bit PCM is supported in the first slice. */
#define ADS_SUPPORTED_BIT_DEPTH ((uint16_t)16)
/* Only mono PCM is supported until interleaved channels arrive. */
#define ADS_SUPPORTED_CHANNEL_COUNT ((uint16_t)1)

/* ── Status / errors ───────────────────────────────────────────────────────*/

/* Mirrors the Rust `AudioSinkError` variants (each of which carried a message).
 * A function that can fail takes an optional `char *msg` / `size_t msg_cap`; on
 * error it writes a human-readable message there (truncated to fit). */
typedef enum {
    ADS_OK = 0,
    ADS_ERR_INVALID_FORMAT,
    ADS_ERR_INVALID_SAMPLES,
    ADS_ERR_UNSUPPORTED_PLATFORM,
    ADS_ERR_BACKEND_UNAVAILABLE,
    ADS_ERR_BACKEND_FAILURE
} AdsStatus;

/* A short label for a status ("invalid PCM format", etc.), like Rust's Display
 * prefix. Never NULL. */
const char *ads_status_label(AdsStatus status);

/* ── PcmFormat ─────────────────────────────────────────────────────────────*/

/* Metadata telling a sink how to interpret integer samples. */
typedef struct {
    uint32_t sample_rate_hz;
    uint16_t channel_count;
    uint16_t bit_depth;
} AdsPcmFormat;

/* Check the V1 sink constraints (rate in 1..=MAX, mono, 16-bit). Returns ADS_OK
 * or an ADS_ERR_INVALID_FORMAT with a message. `msg` may be NULL. */
AdsStatus ads_pcm_format_validate(AdsPcmFormat format, char *msg,
                                  size_t msg_cap);
/* Construct and validate a PCM format, writing it to `*out` on success. */
AdsStatus ads_pcm_format_new(uint32_t sample_rate_hz, uint16_t channel_count,
                             uint16_t bit_depth, AdsPcmFormat *out, char *msg,
                             size_t msg_cap);
/* Bytes occupied by one PCM sample (bit_depth / 8). */
size_t ads_pcm_format_sample_width_bytes(AdsPcmFormat format);

/* ── PcmPlaybackBuffer ─────────────────────────────────────────────────────*/

/* Owned PCM samples plus their format. `samples` is malloc-owned; pair
 * ads_pcm_playback_buffer_new with ads_pcm_playback_buffer_free. */
typedef struct {
    int16_t *samples;
    size_t sample_count;
    AdsPcmFormat format;
} AdsPcmPlaybackBuffer;

/* Construct and validate an owned playback buffer. Copies `n` samples from
 * `samples` (may be NULL iff n == 0). Validates the format and the V1 size cap
 * (<= MAX_BLOCKING_DURATION_SECONDS of audio). Returns ADS_OK (and fills
 * `*out`) or an error. On error `*out` is left zeroed and nothing is allocated
 * (except an out-of-memory case, which frees before returning). */
AdsStatus ads_pcm_playback_buffer_new(const int16_t *samples, size_t n,
                                      AdsPcmFormat format,
                                      AdsPcmPlaybackBuffer *out, char *msg,
                                      size_t msg_cap);
/* Release the owned sample buffer and zero the struct. */
void ads_pcm_playback_buffer_free(AdsPcmPlaybackBuffer *buffer);

/* Number of samples in the buffer. */
size_t ads_pcm_playback_buffer_sample_count(const AdsPcmPlaybackBuffer *buffer);
/* Number of frames (samples / channel_count). */
size_t ads_pcm_playback_buffer_frame_count(const AdsPcmPlaybackBuffer *buffer);
/* True when there is nothing to play. */
int ads_pcm_playback_buffer_is_empty(const AdsPcmPlaybackBuffer *buffer);
/* Intended playback duration in seconds (frames / sample_rate_hz). */
double ads_pcm_playback_buffer_duration_seconds(
    const AdsPcmPlaybackBuffer *buffer);

/* ── PlaybackReport ────────────────────────────────────────────────────────*/

/* Result returned after a sink accepts or completes playback. `backend_name`
 * borrows a caller-owned static string. */
typedef struct {
    size_t frames_played;
    uint32_t sample_rate_hz;
    uint16_t channel_count;
    double duration_seconds;
    const char *backend_name;
} AdsPlaybackReport;

/* Build a report mirroring one validated playback buffer. */
AdsPlaybackReport ads_playback_report_for_buffer(
    const AdsPcmPlaybackBuffer *buffer, const char *backend_name);

/* ── AudioSink (the trait, as a vtable) ────────────────────────────────────*/

/* A sink plays a PCM buffer. `play_blocking` returns ADS_OK and fills `*report`
 * on success, or an error (optionally writing `msg`). `self` is the concrete
 * sink instance below. */
typedef struct AdsAudioSink AdsAudioSink;
struct AdsAudioSink {
    AdsStatus (*play_blocking)(const AdsAudioSink *self,
                               const AdsPcmPlaybackBuffer *buffer,
                               AdsPlaybackReport *report, char *msg,
                               size_t msg_cap);
    const char *backend_name;
};

/* A test/teaching sink that accepts buffers without touching a device: its
 * play_blocking just reports the buffer. */
AdsAudioSink ads_noop_audio_sink(const char *backend_name);

#ifdef __cplusplus
}
#endif

#endif /* AUDIO_DEVICE_SINK_H */
