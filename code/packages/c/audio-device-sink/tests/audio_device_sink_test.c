/* Tests for audio-device-sink, using the header-only iso_test.h harness (pure
 * ISO). Cases mirror the Rust crate's own unit tests. */
#include "iso_test.h"

#include <stdint.h>
#include <string.h>

#include "audio_device_sink.h"

int main(void) {
    /* ── Constants ─────────────────────────────────────────────────────────*/
    ISO_CHECK_STR_EQ(ADS_VERSION, "0.1.0");
    ISO_CHECK_EQ_UINT(ADS_MAX_SAMPLE_RATE_HZ, 384000u);
    ISO_CHECK_EQ_UINT(ADS_SUPPORTED_BIT_DEPTH, 16u);
    ISO_CHECK_EQ_UINT(ADS_SUPPORTED_CHANNEL_COUNT, 1u);
    ISO_CHECK_EQ_DBL(ADS_MAX_BLOCKING_DURATION_SECONDS, 600.0, 1e-9);

    /* ── PcmFormat validation ──────────────────────────────────────────────*/
    {
        AdsPcmFormat fmt;
        char msg[128];
        ISO_CHECK(ads_pcm_format_new(48000, 1, 16, &fmt, msg, sizeof msg) ==
                  ADS_OK);
        ISO_CHECK(fmt.sample_rate_hz == 48000u && fmt.channel_count == 1 &&
                  fmt.bit_depth == 16);
        ISO_CHECK_EQ_UINT(ads_pcm_format_sample_width_bytes(fmt), 2u);

        /* zero rate */
        msg[0] = '\0';
        ISO_CHECK(ads_pcm_format_new(0, 1, 16, &fmt, msg, sizeof msg) ==
                  ADS_ERR_INVALID_FORMAT);
        ISO_CHECK(strstr(msg, "greater than zero") != NULL);
        /* too high */
        ISO_CHECK(ads_pcm_format_new(384001u, 1, 16, &fmt, msg, sizeof msg) ==
                  ADS_ERR_INVALID_FORMAT);
        /* stereo unsupported in V1 */
        msg[0] = '\0';
        ISO_CHECK(ads_pcm_format_new(48000, 2, 16, &fmt, msg, sizeof msg) ==
                  ADS_ERR_INVALID_FORMAT);
        ISO_CHECK(strstr(msg, "mono") != NULL);
        /* 24-bit unsupported in V1 */
        ISO_CHECK(ads_pcm_format_new(48000, 1, 24, &fmt, msg, sizeof msg) ==
                  ADS_ERR_INVALID_FORMAT);
        /* boundary: exactly the max rate is allowed */
        ISO_CHECK(ads_pcm_format_new(384000u, 1, 16, &fmt, msg, sizeof msg) ==
                  ADS_OK);
    }

    /* ── PcmPlaybackBuffer ─────────────────────────────────────────────────*/
    {
        AdsPcmFormat fmt;
        AdsPcmPlaybackBuffer buf;
        static const int16_t samples[4] = {0, 100, -100, 32000};
        char msg[128];

        (void)ads_pcm_format_new(8000, 1, 16, &fmt, NULL, 0);

        ISO_CHECK(ads_pcm_playback_buffer_new(samples, 4, fmt, &buf, msg,
                                              sizeof msg) == ADS_OK);
        ISO_CHECK(ads_pcm_playback_buffer_sample_count(&buf) == 4);
        ISO_CHECK(ads_pcm_playback_buffer_frame_count(&buf) == 4); /* mono */
        ISO_CHECK(!ads_pcm_playback_buffer_is_empty(&buf));
        /* the samples were copied, not aliased */
        ISO_CHECK(buf.samples != samples && buf.samples[3] == 32000);
        /* duration = frames / rate = 4 / 8000 = 0.0005 s */
        ISO_CHECK_EQ_DBL(ads_pcm_playback_buffer_duration_seconds(&buf), 0.0005,
                         1e-9);
        ads_pcm_playback_buffer_free(&buf);
        ISO_CHECK(buf.samples == NULL && buf.sample_count == 0);

        /* empty buffer: allocates nothing, is_empty true, duration 0 */
        ISO_CHECK(ads_pcm_playback_buffer_new(NULL, 0, fmt, &buf, NULL, 0) ==
                  ADS_OK);
        ISO_CHECK(ads_pcm_playback_buffer_is_empty(&buf) &&
                  buf.samples == NULL);
        ISO_CHECK_EQ_DBL(ads_pcm_playback_buffer_duration_seconds(&buf), 0.0,
                         1e-12);
        ads_pcm_playback_buffer_free(&buf);

        /* an invalid format is rejected before allocating */
        AdsPcmFormat bad = {0, 1, 16};
        ISO_CHECK(ads_pcm_playback_buffer_new(samples, 4, bad, &buf, msg,
                                              sizeof msg) ==
                  ADS_ERR_INVALID_FORMAT);
        ISO_CHECK(buf.samples == NULL);

        /* the V1 size cap: > 600 s of audio at the rate is rejected */
        {
            /* claim more samples than the cap allows without allocating them:
             * use the count-only path via a tiny rate so the cap is small. */
            AdsPcmFormat tiny;
            (void)ads_pcm_format_new(1, 1, 16, &tiny, NULL, 0);
            /* cap = 1 Hz * 600 s = 600 samples; 601 must be rejected. Build a
             * 601-sample zero buffer on the stack. */
            static int16_t big[601];
            ISO_CHECK(ads_pcm_playback_buffer_new(big, 601, tiny, &buf, msg,
                                                  sizeof msg) ==
                      ADS_ERR_INVALID_SAMPLES);
            ISO_CHECK(strstr(msg, "limited to") != NULL);
            /* exactly 600 is allowed */
            ISO_CHECK(ads_pcm_playback_buffer_new(big, 600, tiny, &buf, msg,
                                                  sizeof msg) == ADS_OK);
            ads_pcm_playback_buffer_free(&buf);
        }
    }

    /* ── PlaybackReport & NoopAudioSink ────────────────────────────────────*/
    {
        AdsPcmFormat fmt;
        AdsPcmPlaybackBuffer buf;
        static const int16_t samples[3] = {1, 2, 3};
        AdsPlaybackReport report;
        AdsAudioSink sink;

        (void)ads_pcm_format_new(48000, 1, 16, &fmt, NULL, 0);
        (void)ads_pcm_playback_buffer_new(samples, 3, fmt, &buf, NULL, 0);

        report = ads_playback_report_for_buffer(&buf, "test-backend");
        ISO_CHECK(report.frames_played == 3 && report.sample_rate_hz == 48000u);
        ISO_CHECK(report.channel_count == 1);
        ISO_CHECK_STR_EQ(report.backend_name, "test-backend");

        sink = ads_noop_audio_sink("noop");
        {
            AdsPlaybackReport r2;
            ISO_CHECK(sink.play_blocking(&sink, &buf, &r2, NULL, 0) == ADS_OK);
            ISO_CHECK(r2.frames_played == 3);
            ISO_CHECK_STR_EQ(r2.backend_name, "noop");
        }
        ads_pcm_playback_buffer_free(&buf);
    }

    /* ── Status labels ─────────────────────────────────────────────────────*/
    ISO_CHECK_STR_EQ(ads_status_label(ADS_OK), "ok");
    ISO_CHECK(strlen(ads_status_label(ADS_ERR_INVALID_FORMAT)) > 0);
    ISO_CHECK(strstr(ads_status_label(ADS_ERR_INVALID_SAMPLES), "samples") !=
              NULL);

    return ISO_TEST_RESULT();
}
