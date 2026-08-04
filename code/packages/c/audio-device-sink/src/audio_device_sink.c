/*
 * audio_device_sink.c — implementation of the pure-ISO C PCM playback layer.
 * =========================================================================
 *
 * See audio_device_sink.h. All validation logic mirrors the Rust crate; owned
 * samples are a malloc'd int16_t buffer; error messages are written into the
 * caller's optional buffer with snprintf.
 */
#include "audio_device_sink.h"

#include <stdio.h>  /* snprintf */
#include <stdlib.h> /* malloc, free */
#include <string.h> /* memcpy, memset */

const char *const ADS_VERSION = "0.1.0";

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
    snprintf(msg, cap, "%s", text);
}

/* ── PcmFormat ─────────────────────────────────────────────────────────────*/

AdsStatus ads_pcm_format_validate(AdsPcmFormat format, char *msg,
                                  size_t msg_cap) {
    if (format.sample_rate_hz == 0) {
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
    if (format.channel_count != ADS_SUPPORTED_CHANNEL_COUNT) {
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

/* frame_count using an explicit format+count (used before a buffer exists). */
static size_t frames_of(size_t sample_count, AdsPcmFormat format) {
    /* channel_count is validated non-zero (mono) before this is reached. */
    return sample_count / (size_t)format.channel_count;
}

AdsStatus ads_pcm_playback_buffer_new(const int16_t *samples, size_t n,
                                      AdsPcmFormat format,
                                      AdsPcmPlaybackBuffer *out, char *msg,
                                      size_t msg_cap) {
    AdsStatus st;
    double max_samples;
    int16_t *copy;

    memset(out, 0, sizeof *out);

    st = ads_pcm_format_validate(format, msg, msg_cap);
    if (st != ADS_OK) return st;

    /* V1 size cap: sample_count must fit within the blocking-duration limit. */
    max_samples =
        (double)format.sample_rate_hz * ADS_MAX_BLOCKING_DURATION_SECONDS;
    if ((double)n > max_samples) {
        if (msg != NULL && msg_cap > 0)
            snprintf(msg, msg_cap,
                     "blocking playback is limited to %g seconds",
                     (double)ADS_MAX_BLOCKING_DURATION_SECONDS);
        return ADS_ERR_INVALID_SAMPLES;
    }

    if (n == 0) {
        copy = NULL; /* empty buffer owns nothing */
    } else {
        /* calloc does the checked multiply for us — a guard against size_t
         * overflow even though the duration cap already bounds `n`. */
        copy = (int16_t *)calloc(n, sizeof(int16_t));
        if (copy == NULL) {
            set_msg(msg, msg_cap, "out of memory");
            return ADS_ERR_BACKEND_FAILURE;
        }
        memcpy(copy, samples, n * sizeof(int16_t));
    }

    out->samples = copy;
    out->sample_count = n;
    out->format = format;
    return ADS_OK;
}

void ads_pcm_playback_buffer_free(AdsPcmPlaybackBuffer *buffer) {
    if (buffer == NULL) return;
    free(buffer->samples);
    memset(buffer, 0, sizeof *buffer);
}

size_t ads_pcm_playback_buffer_sample_count(
    const AdsPcmPlaybackBuffer *buffer) {
    return buffer->sample_count;
}

size_t ads_pcm_playback_buffer_frame_count(const AdsPcmPlaybackBuffer *buffer) {
    return frames_of(buffer->sample_count, buffer->format);
}

int ads_pcm_playback_buffer_is_empty(const AdsPcmPlaybackBuffer *buffer) {
    return buffer->sample_count == 0;
}

double ads_pcm_playback_buffer_duration_seconds(
    const AdsPcmPlaybackBuffer *buffer) {
    return (double)ads_pcm_playback_buffer_frame_count(buffer) /
           (double)buffer->format.sample_rate_hz;
}

/* ── PlaybackReport ────────────────────────────────────────────────────────*/

AdsPlaybackReport ads_playback_report_for_buffer(
    const AdsPcmPlaybackBuffer *buffer, const char *backend_name) {
    AdsPlaybackReport report;
    report.frames_played = ads_pcm_playback_buffer_frame_count(buffer);
    report.sample_rate_hz = buffer->format.sample_rate_hz;
    report.channel_count = buffer->format.channel_count;
    report.duration_seconds = ads_pcm_playback_buffer_duration_seconds(buffer);
    report.backend_name = backend_name;
    return report;
}

/* ── AudioSink / NoopAudioSink ─────────────────────────────────────────────*/

static AdsStatus noop_play_blocking(const AdsAudioSink *self,
                                    const AdsPcmPlaybackBuffer *buffer,
                                    AdsPlaybackReport *report, char *msg,
                                    size_t msg_cap) {
    (void)msg;
    (void)msg_cap;
    *report = ads_playback_report_for_buffer(buffer, self->backend_name);
    return ADS_OK;
}

AdsAudioSink ads_noop_audio_sink(const char *backend_name) {
    AdsAudioSink sink;
    sink.play_blocking = noop_play_blocking;
    sink.backend_name = backend_name;
    return sink;
}
