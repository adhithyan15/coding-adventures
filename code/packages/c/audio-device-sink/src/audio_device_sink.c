/*
 * audio_device_sink.c — implementation of the pure-ISO C PCM playback layer.
 * =========================================================================
 *
 * See audio_device_sink.h. All validation logic mirrors the Rust crate; owned
 * samples are a calloc'd int16_t buffer; error messages are written into the
 * caller's optional buffer with snprintf.
 *
 * Hardened against VER15 CWEs (190, 400, 415, 416, 476) with zero <math.h>.
 */
#include "audio_device_sink.h"

#include <stdio.h>  /* snprintf */
#include <stdlib.h> /* calloc, free */
#include <string.h> /* memcpy, memset */

const char *const ADS_VERSION = "0.1.0";

/* ── Checked arithmetic & IEEE-754 helpers (CWE-190 / zero libm) ───────────*/

static int checked_mul_size(size_t a, size_t b, size_t *out) {
    if (out == NULL) return 0;
    if (a != 0 && b > SIZE_MAX / a) return 0;
    *out = a * b;
    return 1;
}

static int checked_mul_u64(uint64_t a, uint64_t b, uint64_t *out) {
    if (out == NULL) return 0;
    if (a != 0 && b > UINT64_MAX / a) return 0;
    *out = a * b;
    return 1;
}

static int is_finite_double(double x) {
    /* Pure IEEE-754: x == x rejects NaN; (x - x) == 0.0 rejects +/-Inf. */
    return (x == x) && ((x - x) == 0.0);
}

/* Compute exact integer max sample count for a validated format. */
static int max_samples_for_format(AdsPcmFormat format, uint64_t *out) {
    uint64_t max_frames = 0;
    if (!checked_mul_u64((uint64_t)format.sample_rate_hz,
                         ADS_MAX_BLOCKING_DURATION_SECONDS_INT, &max_frames)) {
        return 0;
    }
    if (!checked_mul_u64(max_frames, (uint64_t)format.channel_count, out)) {
        return 0;
    }
    return 1;
}

const char *ads_status_label(AdsStatus status) {
    switch (status) {
        case ADS_OK: return "ok";
        case ADS_ERR_INVALID_FORMAT: return "invalid PCM format";
        case ADS_ERR_INVALID_SAMPLES: return "invalid PCM samples";
        case ADS_ERR_UNSUPPORTED_PLATFORM: return "unsupported platform";
        case ADS_ERR_BACKEND_UNAVAILABLE: return "audio backend unavailable";
        case ADS_ERR_BACKEND_FAILURE: return "audio backend failure";
    }
    return "unknown error";
}

/* Write `text` into the optional caller buffer (safe if msg is NULL). */
static void set_msg(char *msg, size_t cap, const char *text) {
    if (msg == NULL || cap == 0) return;
    snprintf(msg, cap, "%s", text != NULL ? text : "");
}

/* ── PcmFormat ─────────────────────────────────────────────────────────────*/

AdsStatus ads_pcm_format_validate(AdsPcmFormat format, char *msg,
                                  size_t msg_cap) {
    if (format.sample_rate_hz < ADS_MIN_SAMPLE_RATE_HZ) {
        set_msg(msg, msg_cap, "sample_rate_hz must be greater than zero");
        return ADS_ERR_INVALID_FORMAT;
    }
    if (format.sample_rate_hz > ADS_MAX_SAMPLE_RATE_HZ) {
        if (msg != NULL && msg_cap > 0)
            snprintf(msg, msg_cap,
                     "sample_rate_hz must be <= %lu, got %lu",
                     (unsigned long)ADS_MAX_SAMPLE_RATE_HZ,
                     (unsigned long)format.sample_rate_hz);
        return ADS_ERR_INVALID_FORMAT;
    }
    if (format.channel_count != ADS_SUPPORTED_CHANNEL_COUNT ||
        format.channel_count > ADS_MAX_CHANNELS) {
        if (msg != NULL && msg_cap > 0)
            snprintf(msg, msg_cap,
                     "only mono PCM is supported in V1, got %u channels",
                     (unsigned)format.channel_count);
        return ADS_ERR_INVALID_FORMAT;
    }
    if (format.bit_depth != ADS_SUPPORTED_BIT_DEPTH) {
        if (msg != NULL && msg_cap > 0)
            snprintf(msg, msg_cap,
                     "only signed 16-bit PCM is supported in V1, got %u bits",
                     (unsigned)format.bit_depth);
        return ADS_ERR_INVALID_FORMAT;
    }
    return ADS_OK;
}

AdsStatus ads_pcm_format_new(uint32_t sample_rate_hz, uint16_t channel_count,
                             uint16_t bit_depth, AdsPcmFormat *out, char *msg,
                             size_t msg_cap) {
    AdsPcmFormat format;
    AdsStatus st;
    if (out == NULL) {
        set_msg(msg, msg_cap, "out format pointer must not be NULL");
        return ADS_ERR_INVALID_FORMAT;
    }
    memset(out, 0, sizeof *out);
    format.sample_rate_hz = sample_rate_hz;
    format.channel_count = channel_count;
    format.bit_depth = bit_depth;
    st = ads_pcm_format_validate(format, msg, msg_cap);
    if (st == ADS_OK) *out = format;
    return st;
}

size_t ads_pcm_format_sample_width_bytes(AdsPcmFormat format) {
    return (size_t)(format.bit_depth / 8);
}

/* ── PcmPlaybackBuffer ─────────────────────────────────────────────────────*/

/* frame_count using an explicit format+count; safe against div-by-zero. */
static size_t frames_of(size_t sample_count, AdsPcmFormat format) {
    if (format.channel_count == 0) return 0;
    return sample_count / (size_t)format.channel_count;
}

static AdsStatus validate_sample_bounds(const int16_t *samples, size_t n,
                                        AdsPcmFormat format, char *msg,
                                        size_t msg_cap) {
    uint64_t max_samples = 0;
    size_t byte_len = 0;

    if (n > 0 && samples == NULL) {
        set_msg(msg, msg_cap,
                "samples pointer must not be NULL when sample_count > 0");
        return ADS_ERR_INVALID_SAMPLES;
    }

    if (n > ADS_MAX_SAMPLES) {
        if (msg != NULL && msg_cap > 0)
            snprintf(msg, msg_cap,
                     "blocking playback is limited to %g seconds",
                     (double)ADS_MAX_BLOCKING_DURATION_SECONDS);
        return ADS_ERR_INVALID_SAMPLES;
    }

    if (!checked_mul_size(n, sizeof(int16_t), &byte_len)) {
        set_msg(msg, msg_cap, "sample buffer byte size overflow");
        return ADS_ERR_INVALID_SAMPLES;
    }

    if (!max_samples_for_format(format, &max_samples) ||
        (uint64_t)n > max_samples) {
        if (msg != NULL && msg_cap > 0)
            snprintf(msg, msg_cap,
                     "blocking playback is limited to %g seconds",
                     (double)ADS_MAX_BLOCKING_DURATION_SECONDS);
        return ADS_ERR_INVALID_SAMPLES;
    }

    (void)byte_len;
    return ADS_OK;
}

AdsStatus ads_pcm_playback_buffer_new(const int16_t *samples, size_t n,
                                      AdsPcmFormat format,
                                      AdsPcmPlaybackBuffer *out, char *msg,
                                      size_t msg_cap) {
    AdsStatus st;
    size_t byte_len = 0;
    int16_t *copy = NULL;

    if (out == NULL) {
        set_msg(msg, msg_cap, "out buffer pointer must not be NULL");
        return ADS_ERR_INVALID_SAMPLES;
    }
    memset(out, 0, sizeof *out);

    st = ads_pcm_format_validate(format, msg, msg_cap);
    if (st != ADS_OK) return st;

    st = validate_sample_bounds(samples, n, format, msg, msg_cap);
    if (st != ADS_OK) return st;

    if (n == 0) {
        copy = NULL; /* empty buffer owns nothing */
    } else {
        if (!checked_mul_size(n, sizeof(int16_t), &byte_len)) {
            set_msg(msg, msg_cap, "sample buffer byte size overflow");
            return ADS_ERR_INVALID_SAMPLES;
        }
        copy = (int16_t *)calloc(n, sizeof(int16_t));
        if (copy == NULL) {
            set_msg(msg, msg_cap, "out of memory");
            return ADS_ERR_BACKEND_FAILURE;
        }
        memcpy(copy, samples, byte_len);
    }

    out->samples = copy;
    out->sample_count = n;
    out->format = format;
    return ADS_OK;
}

void ads_pcm_playback_buffer_free(AdsPcmPlaybackBuffer *buffer) {
    size_t byte_len = 0;
    if (buffer == NULL) return;
    if (buffer->samples != NULL) {
        if (buffer->sample_count > 0 &&
            buffer->sample_count <= ADS_MAX_SAMPLES &&
            checked_mul_size(buffer->sample_count, sizeof(int16_t),
                             &byte_len)) {
            volatile unsigned char *vp =
                (volatile unsigned char *)buffer->samples;
            size_t i;
            for (i = 0; i < byte_len; ++i) {
                vp[i] = 0;
            }
        }
        free(buffer->samples);
    }
    memset(buffer, 0, sizeof *buffer);
}

size_t ads_pcm_playback_buffer_sample_count(
    const AdsPcmPlaybackBuffer *buffer) {
    if (buffer == NULL) return 0;
    if (buffer->sample_count > 0 && buffer->samples == NULL) return 0;
    return buffer->sample_count;
}

size_t ads_pcm_playback_buffer_frame_count(const AdsPcmPlaybackBuffer *buffer) {
    if (buffer == NULL) return 0;
    if (buffer->sample_count > 0 && buffer->samples == NULL) return 0;
    return frames_of(buffer->sample_count, buffer->format);
}

int ads_pcm_playback_buffer_is_empty(const AdsPcmPlaybackBuffer *buffer) {
    return ads_pcm_playback_buffer_sample_count(buffer) == 0;
}

double ads_pcm_playback_buffer_duration_seconds(
    const AdsPcmPlaybackBuffer *buffer) {
    size_t frames;
    double duration;
    if (buffer == NULL) return 0.0;
    if (buffer->format.sample_rate_hz == 0 ||
        buffer->format.channel_count == 0) {
        return 0.0;
    }
    if (buffer->sample_count > 0 && buffer->samples == NULL) return 0.0;
    frames = ads_pcm_playback_buffer_frame_count(buffer);
    duration = (double)frames / (double)buffer->format.sample_rate_hz;
    return is_finite_double(duration) ? duration : 0.0;
}

size_t ads_pcm_playback_buffer_byte_size(const AdsPcmPlaybackBuffer *buffer) {
    size_t count;
    size_t bytes = 0;
    if (buffer == NULL) return 0;
    count = ads_pcm_playback_buffer_sample_count(buffer);
    if (count == 0 || count > ADS_MAX_SAMPLES) return 0;
    if (!checked_mul_size(count, sizeof(int16_t), &bytes)) return 0;
    return bytes;
}

/* ── PlaybackReport ────────────────────────────────────────────────────────*/

AdsPlaybackReport ads_playback_report_for_buffer(
    const AdsPcmPlaybackBuffer *buffer, const char *backend_name) {
    AdsPlaybackReport report;
    const char *safe_name = (backend_name != NULL) ? backend_name : "";
    memset(&report, 0, sizeof report);
    if (buffer == NULL) {
        report.backend_name = safe_name;
        return report;
    }
    report.frames_played = ads_pcm_playback_buffer_frame_count(buffer);
    report.sample_rate_hz = buffer->format.sample_rate_hz;
    report.channel_count = buffer->format.channel_count;
    report.duration_seconds = ads_pcm_playback_buffer_duration_seconds(buffer);
    report.backend_name = safe_name;
    return report;
}

/* ── AudioSink / NoopAudioSink ─────────────────────────────────────────────*/

static AdsStatus noop_play_blocking(const AdsAudioSink *self,
                                    const AdsPcmPlaybackBuffer *buffer,
                                    AdsPlaybackReport *report, char *msg,
                                    size_t msg_cap) {
    AdsStatus st;
    if (report != NULL) {
        memset(report, 0, sizeof *report);
    }
    if (self == NULL) {
        set_msg(msg, msg_cap, "audio sink must not be NULL");
        return ADS_ERR_BACKEND_FAILURE;
    }
    if (report == NULL) {
        set_msg(msg, msg_cap, "output report pointer must not be NULL");
        return ADS_ERR_BACKEND_FAILURE;
    }
    if (buffer == NULL) {
        set_msg(msg, msg_cap, "playback buffer must not be NULL");
        return ADS_ERR_INVALID_SAMPLES;
    }
    st = ads_pcm_format_validate(buffer->format, msg, msg_cap);
    if (st != ADS_OK) return st;

    st = validate_sample_bounds(buffer->samples, buffer->sample_count,
                                buffer->format, msg, msg_cap);
    if (st != ADS_OK) return st;

    *report = ads_playback_report_for_buffer(buffer, self->backend_name);
    return ADS_OK;
}

AdsAudioSink ads_noop_audio_sink(const char *backend_name) {
    AdsAudioSink sink;
    sink.play_blocking = noop_play_blocking;
    sink.backend_name = (backend_name != NULL) ? backend_name : "";
    return sink;
}
