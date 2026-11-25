/* Tests for audio-device-sink, using the header-only iso_test.h harness (pure
 * ISO). Cases mirror the Rust crate's own unit tests, plus VER15 hostile
 * adversarial tests (CWE-190, CWE-400, CWE-415, CWE-416, CWE-476) and
 * SplitMix64 property-based tests (zero <math.h> / -lm dependency). */
#include "iso_test.h"

#include <stdint.h>
#include <string.h>

#include "audio_device_sink.h"

/* Pure IEEE-754 finiteness check without <math.h>. */
static int test_is_finite(double x) {
    return (x == x) && ((x - x) == 0.0);
}

static uint64_t splitmix64_next(uint64_t *state) {
    uint64_t z = (*state += UINT64_C(0x9E3779B97F4A7C15));
    z = (z ^ (z >> 30)) * UINT64_C(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)) * UINT64_C(0x94D049BB133111EB);
    return z ^ (z >> 31);
}

static void test_baseline_contract(void) {
    /* ── Constants ─────────────────────────────────────────────────────────*/
    ISO_CHECK_STR_EQ(ADS_VERSION, "0.1.0");
    ISO_CHECK_EQ_UINT(ADS_MIN_SAMPLE_RATE_HZ, 1u);
    ISO_CHECK_EQ_UINT(ADS_MAX_SAMPLE_RATE_HZ, 384000u);
    ISO_CHECK_EQ_UINT(ADS_SUPPORTED_BIT_DEPTH, 16u);
    ISO_CHECK_EQ_UINT(ADS_SUPPORTED_CHANNEL_COUNT, 1u);
    ISO_CHECK_EQ_UINT(ADS_MAX_CHANNELS, 1u);
    ISO_CHECK_EQ_UINT(ADS_MAX_SAMPLES, 230400000UL);
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
        ISO_CHECK_EQ_UINT(ads_pcm_playback_buffer_byte_size(&buf), 8u);
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
        ISO_CHECK_EQ_UINT(ads_pcm_playback_buffer_byte_size(&buf), 0u);
        ISO_CHECK_EQ_DBL(ads_pcm_playback_buffer_duration_seconds(&buf), 0.0,
                         1e-12);
        ads_pcm_playback_buffer_free(&buf);

        /* an invalid format is rejected before allocating */
        {
            AdsPcmFormat bad = {0, 1, 16};
            ISO_CHECK(ads_pcm_playback_buffer_new(samples, 4, bad, &buf, msg,
                                                  sizeof msg) ==
                      ADS_ERR_INVALID_FORMAT);
            ISO_CHECK(buf.samples == NULL);
        }

        /* the V1 size cap: > 600 s of audio at the rate is rejected */
        {
            AdsPcmFormat tiny;
            static int16_t big[601];
            (void)ads_pcm_format_new(1, 1, 16, &tiny, NULL, 0);
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
    ISO_CHECK_STR_EQ(ads_status_label((AdsStatus)999), "unknown error");
}

/* CWE-476: Hostile NULL pointer checks across every API entry point. */
static void test_hostile_null_pointers(void) {
    AdsPcmFormat fmt;
    AdsPcmPlaybackBuffer buf;
    AdsPlaybackReport report;
    AdsAudioSink sink;
    static const int16_t samples[4] = {10, 20, 30, 40};
    char msg[128];

    (void)ads_pcm_format_new(44100, 1, 16, &fmt, NULL, 0);

    /* NULL out-param in ads_pcm_format_new */
    msg[0] = '\0';
    ISO_CHECK(ads_pcm_format_new(44100, 1, 16, NULL, msg, sizeof msg) ==
              ADS_ERR_INVALID_FORMAT);
    ISO_CHECK(strlen(msg) > 0);
    ISO_CHECK(ads_pcm_format_new(44100, 1, 16, NULL, NULL, 0) ==
              ADS_ERR_INVALID_FORMAT);
    /* Zero msg_cap with non-NULL msg must not write */
    msg[0] = 'X';
    ISO_CHECK(ads_pcm_format_validate((AdsPcmFormat){0, 1, 16}, msg, 0) ==
              ADS_ERR_INVALID_FORMAT);
    ISO_CHECK(msg[0] == 'X');

    /* NULL out-param in ads_pcm_playback_buffer_new */
    msg[0] = '\0';
    ISO_CHECK(ads_pcm_playback_buffer_new(samples, 4, fmt, NULL, msg,
                                          sizeof msg) ==
              ADS_ERR_INVALID_SAMPLES);
    ISO_CHECK(strlen(msg) > 0);
    ISO_CHECK(ads_pcm_playback_buffer_new(samples, 4, fmt, NULL, NULL, 0) ==
              ADS_ERR_INVALID_SAMPLES);

    /* NULL samples with n > 0 */
    buf.samples = (int16_t *)samples; /* dirty pre-state must be zeroed */
    buf.sample_count = 999u;
    msg[0] = '\0';
    ISO_CHECK(ads_pcm_playback_buffer_new(NULL, 4, fmt, &buf, msg,
                                          sizeof msg) ==
              ADS_ERR_INVALID_SAMPLES);
    ISO_CHECK(buf.samples == NULL && buf.sample_count == 0);
    ISO_CHECK(strstr(msg, "NULL") != NULL);

    /* NULL struct in free and all accessors */
    ads_pcm_playback_buffer_free(NULL);
    ISO_CHECK_EQ_UINT(ads_pcm_playback_buffer_sample_count(NULL), 0u);
    ISO_CHECK_EQ_UINT(ads_pcm_playback_buffer_frame_count(NULL), 0u);
    ISO_CHECK_EQ_INT(ads_pcm_playback_buffer_is_empty(NULL), 1);
    ISO_CHECK_EQ_UINT(ads_pcm_playback_buffer_byte_size(NULL), 0u);
    {
        double dur = ads_pcm_playback_buffer_duration_seconds(NULL);
        ISO_CHECK(test_is_finite(dur));
        ISO_CHECK_EQ_DBL(dur, 0.0, 1e-12);
    }

    /* NULL buffer / backend_name in report & noop sink */
    report = ads_playback_report_for_buffer(NULL, "safe");
    ISO_CHECK_EQ_UINT(report.frames_played, 0u);
    ISO_CHECK_EQ_UINT(report.sample_rate_hz, 0u);
    ISO_CHECK_EQ_UINT(report.channel_count, 0u);
    ISO_CHECK(test_is_finite(report.duration_seconds));
    ISO_CHECK_STR_EQ(report.backend_name, "safe");

    report = ads_playback_report_for_buffer(NULL, NULL);
    ISO_CHECK(report.backend_name != NULL);
    ISO_CHECK_STR_EQ(report.backend_name, "");

    sink = ads_noop_audio_sink(NULL);
    ISO_CHECK(sink.backend_name != NULL);
    ISO_CHECK_STR_EQ(sink.backend_name, "");

    ISO_CHECK(ads_pcm_playback_buffer_new(samples, 4, fmt, &buf, NULL, 0) ==
              ADS_OK);
    ISO_CHECK(sink.play_blocking(NULL, &buf, &report, msg, sizeof msg) ==
              ADS_ERR_BACKEND_FAILURE);
    ISO_CHECK(sink.play_blocking(&sink, NULL, &report, msg, sizeof msg) ==
              ADS_ERR_INVALID_SAMPLES);
    ISO_CHECK(sink.play_blocking(&sink, &buf, NULL, msg, sizeof msg) ==
              ADS_ERR_BACKEND_FAILURE);
    ISO_CHECK(sink.play_blocking(&sink, &buf, &report, NULL, 0) == ADS_OK);
    ISO_CHECK_STR_EQ(report.backend_name, "");
    ads_pcm_playback_buffer_free(&buf);
}

/* CWE-190 / CWE-400: Hostile bounds, wrapped negatives, SIZE_MAX, non-finite. */
static void test_hostile_bounds_and_overflow(void) {
    AdsPcmFormat fmt;
    AdsPcmPlaybackBuffer buf;
    static const int16_t dummy[1] = {0};
    char msg[128];

    /* Zero / wrapped-negative / excessive sample rates, channels, bit depths */
    ISO_CHECK(ads_pcm_format_new(0u, 1u, 16u, &fmt, msg, sizeof msg) ==
              ADS_ERR_INVALID_FORMAT);
    ISO_CHECK(ads_pcm_format_new((uint32_t)-1, 1u, 16u, &fmt, msg,
                                 sizeof msg) == ADS_ERR_INVALID_FORMAT);
    ISO_CHECK(fmt.sample_rate_hz == 0u && fmt.channel_count == 0u &&
              fmt.bit_depth == 0u);
    ISO_CHECK(ads_pcm_format_new(ADS_MAX_SAMPLE_RATE_HZ + 1u, 1u, 16u, &fmt,
                                 msg, sizeof msg) == ADS_ERR_INVALID_FORMAT);
    ISO_CHECK(ads_pcm_format_new(44100u, 0u, 16u, &fmt, msg, sizeof msg) ==
              ADS_ERR_INVALID_FORMAT);
    ISO_CHECK(ads_pcm_format_new(44100u, (uint16_t)-1, 16u, &fmt, msg,
                                 sizeof msg) == ADS_ERR_INVALID_FORMAT);
    ISO_CHECK(ads_pcm_format_new(44100u, 1u, 0u, &fmt, msg, sizeof msg) ==
              ADS_ERR_INVALID_FORMAT);
    ISO_CHECK(ads_pcm_format_new(44100u, 1u, 8u, &fmt, msg, sizeof msg) ==
              ADS_ERR_INVALID_FORMAT);
    ISO_CHECK(ads_pcm_format_new(44100u, 1u, (uint16_t)-1, &fmt, msg,
                                 sizeof msg) == ADS_ERR_INVALID_FORMAT);

    /* SIZE_MAX and overflow-inducing sample counts rejected pre-flight */
    ISO_CHECK(ads_pcm_format_new(ADS_MAX_SAMPLE_RATE_HZ, 1u, 16u, &fmt, NULL,
                                 0) == ADS_OK);
    ISO_CHECK(ads_pcm_playback_buffer_new(dummy, SIZE_MAX, fmt, &buf, msg,
                                          sizeof msg) ==
              ADS_ERR_INVALID_SAMPLES);
    ISO_CHECK(buf.samples == NULL && buf.sample_count == 0);
    ISO_CHECK(ads_pcm_playback_buffer_new(dummy, SIZE_MAX / 2u, fmt, &buf, msg,
                                          sizeof msg) ==
              ADS_ERR_INVALID_SAMPLES);
    ISO_CHECK(ads_pcm_playback_buffer_new(
                  dummy, (SIZE_MAX / sizeof(int16_t)) + 1u, fmt, &buf, msg,
                  sizeof msg) == ADS_ERR_INVALID_SAMPLES);
    ISO_CHECK(ads_pcm_playback_buffer_new(dummy, ADS_MAX_SAMPLES + 1u, fmt,
                                          &buf, msg, sizeof msg) ==
              ADS_ERR_INVALID_SAMPLES);

    /* Per-rate duration cap boundary: 44100 * 600 + 1 rejected pre-flight */
    ISO_CHECK(ads_pcm_format_new(44100u, 1u, 16u, &fmt, NULL, 0) == ADS_OK);
    ISO_CHECK(ads_pcm_playback_buffer_new(
                  dummy, (size_t)44100u * (size_t)600u + 1u, fmt, &buf, msg,
                  sizeof msg) == ADS_ERR_INVALID_SAMPLES);

    /* IEEE-754 non-finite sanity: test helper detects NaN/Inf, and library
     * outputs are always finite even on forged zeroed structs. */
    {
        union { uint64_t u; double d; } v_nan, v_inf, v_ninf;
        AdsPcmPlaybackBuffer forged;
        v_nan.u = UINT64_C(0x7FF8000000000000);
        v_inf.u = UINT64_C(0x7FF0000000000000);
        v_ninf.u = UINT64_C(0xFFF0000000000000);
        ISO_CHECK(!test_is_finite(v_nan.d));
        ISO_CHECK(!test_is_finite(v_inf.d));
        ISO_CHECK(!test_is_finite(v_ninf.d));
        ISO_CHECK(test_is_finite(0.0));

        memset(&forged, 0, sizeof forged);
        forged.samples = (int16_t *)(void *)dummy;
        forged.sample_count = 4u;
        /* channel_count == 0 and sample_rate_hz == 0 must not div-by-zero/NaN */
        ISO_CHECK_EQ_UINT(ads_pcm_playback_buffer_frame_count(&forged), 0u);
        ISO_CHECK(test_is_finite(
            ads_pcm_playback_buffer_duration_seconds(&forged)));
        ISO_CHECK_EQ_DBL(ads_pcm_playback_buffer_duration_seconds(&forged), 0.0,
                         1e-12);
    }
}

/* CWE-415 / CWE-416: Double-free idempotency, zero-on-free, UAF invariants. */
static void test_hostile_double_free_and_uaf(void) {
    AdsPcmFormat fmt;
    AdsPcmPlaybackBuffer buf;
    AdsPlaybackReport report;
    AdsAudioSink sink;
    static const int16_t samples[4] = {1234, -5678, 32767, -32768};
    char msg[128];

    (void)ads_pcm_format_new(48000u, 1u, 16u, &fmt, NULL, 0);
    ISO_CHECK(ads_pcm_playback_buffer_new(samples, 4u, fmt, &buf, msg,
                                          sizeof msg) == ADS_OK);
    ISO_CHECK(buf.samples != NULL);

    /* First free zeroes fields and frees samples */
    ads_pcm_playback_buffer_free(&buf);
    ISO_CHECK(buf.samples == NULL);
    ISO_CHECK_EQ_UINT(buf.sample_count, 0u);
    ISO_CHECK_EQ_UINT(buf.format.sample_rate_hz, 0u);
    ISO_CHECK_EQ_UINT(buf.format.channel_count, 0u);
    ISO_CHECK_EQ_UINT(buf.format.bit_depth, 0u);

    /* Second free (double-free) is idempotent and safe */
    ads_pcm_playback_buffer_free(&buf);
    ISO_CHECK(buf.samples == NULL && buf.sample_count == 0u);

    /* Use-after-free invariants: accessors return safe zeroed defaults */
    ISO_CHECK_EQ_UINT(ads_pcm_playback_buffer_sample_count(&buf), 0u);
    ISO_CHECK_EQ_UINT(ads_pcm_playback_buffer_frame_count(&buf), 0u);
    ISO_CHECK_EQ_INT(ads_pcm_playback_buffer_is_empty(&buf), 1);
    ISO_CHECK_EQ_UINT(ads_pcm_playback_buffer_byte_size(&buf), 0u);
    ISO_CHECK(test_is_finite(ads_pcm_playback_buffer_duration_seconds(&buf)));
    ISO_CHECK_EQ_DBL(ads_pcm_playback_buffer_duration_seconds(&buf), 0.0,
                     1e-12);

    /* Freed buffer must be rejected by play_blocking */
    sink = ads_noop_audio_sink("noop");
    ISO_CHECK(sink.play_blocking(&sink, &buf, &report, msg, sizeof msg) ==
              ADS_ERR_INVALID_FORMAT);
    ISO_CHECK_EQ_UINT(report.frames_played, 0u);

    /* Forged NULL-samples with non-zero sample_count must be rejected */
    buf.format = fmt;
    buf.samples = NULL;
    buf.sample_count = 4u;
    ISO_CHECK_EQ_UINT(ads_pcm_playback_buffer_sample_count(&buf), 0u);
    ISO_CHECK_EQ_UINT(ads_pcm_playback_buffer_frame_count(&buf), 0u);
    ISO_CHECK_EQ_INT(ads_pcm_playback_buffer_is_empty(&buf), 1);
    ISO_CHECK_EQ_UINT(ads_pcm_playback_buffer_byte_size(&buf), 0u);
    ISO_CHECK(sink.play_blocking(&sink, &buf, &report, msg, sizeof msg) ==
              ADS_ERR_INVALID_SAMPLES);
}

/* SplitMix64 Property-Based Tests across format, buffer, duration, and sink. */
static void test_pbt_splitmix64(void) {
    uint64_t rng = UINT64_C(0xA511E9B3D4C2017F);
    AdsAudioSink sink = ads_noop_audio_sink("pbt-noop");
    int16_t scratch[512];
    int16_t mutated[512];
    size_t trial;

    for (trial = 0; trial < 256; ++trial) {
        uint32_t rate =
            (uint32_t)(splitmix64_next(&rng) % ADS_MAX_SAMPLE_RATE_HZ) + 1u;
        size_t max_n = (size_t)rate * 600u;
        size_t n = (size_t)(splitmix64_next(&rng) % 513u);
        AdsPcmFormat fmt;
        AdsPcmPlaybackBuffer buf;
        AdsPlaybackReport report;
        size_t i;

        if (n > max_n) n = max_n;

        ISO_CHECK(ads_pcm_format_new(rate, 1u, 16u, &fmt, NULL, 0) == ADS_OK);
        ISO_CHECK_EQ_UINT(ads_pcm_format_sample_width_bytes(fmt), 2u);

        for (i = 0; i < n; ++i) {
            int16_t val = (int16_t)(uint16_t)(splitmix64_next(&rng) & 0xFFFFu);
            scratch[i] = val;
            mutated[i] = val;
        }

        ISO_CHECK(ads_pcm_playback_buffer_new(
                      n == 0 ? NULL : mutated, n, fmt, &buf, NULL, 0) ==
                  ADS_OK);

        /* Mutating caller storage must not affect owned buffer */
        for (i = 0; i < n; ++i) {
            mutated[i] = (int16_t)(~mutated[i]);
        }

        ISO_CHECK_EQ_UINT(ads_pcm_playback_buffer_sample_count(&buf), n);
        ISO_CHECK_EQ_UINT(ads_pcm_playback_buffer_frame_count(&buf), n);
        ISO_CHECK_EQ_INT(ads_pcm_playback_buffer_is_empty(&buf), n == 0 ? 1 : 0);
        ISO_CHECK_EQ_UINT(ads_pcm_playback_buffer_byte_size(&buf),
                          n * sizeof(int16_t));
        if (n > 0) {
            ISO_CHECK_MEM_EQ(buf.samples, scratch, n * sizeof(int16_t));
        } else {
            ISO_CHECK(buf.samples == NULL);
        }

        {
            double expected_dur = (double)n / (double)rate;
            double actual_dur = ads_pcm_playback_buffer_duration_seconds(&buf);
            ISO_CHECK(test_is_finite(actual_dur));
            ISO_CHECK(actual_dur >= 0.0 && actual_dur <= 600.0);
            ISO_CHECK_EQ_DBL(actual_dur, expected_dur, 1e-12);
        }

        ISO_CHECK(sink.play_blocking(&sink, &buf, &report, NULL, 0) == ADS_OK);
        ISO_CHECK_EQ_UINT(report.frames_played, n);
        ISO_CHECK_EQ_UINT(report.sample_rate_hz, rate);
        ISO_CHECK_EQ_UINT(report.channel_count, 1u);
        ISO_CHECK_EQ_DBL(report.duration_seconds,
                         ads_pcm_playback_buffer_duration_seconds(&buf), 1e-12);
        ISO_CHECK_STR_EQ(report.backend_name, "pbt-noop");

        ads_pcm_playback_buffer_free(&buf);
        ads_pcm_playback_buffer_free(&buf);

        /* Hostile PBT: invalid rates/channels/depths always fail & zero out */
        {
            uint32_t bad_rate =
                ADS_MAX_SAMPLE_RATE_HZ + 1u +
                (uint32_t)(splitmix64_next(&rng) % 1000000u);
            AdsPcmFormat bad_fmt;
            bad_fmt.sample_rate_hz = 123u;
            ISO_CHECK(ads_pcm_format_new(bad_rate, 1u, 16u, &bad_fmt, NULL,
                                         0) == ADS_ERR_INVALID_FORMAT);
            ISO_CHECK_EQ_UINT(bad_fmt.sample_rate_hz, 0u);
        }

        /* Hostile PBT: over-cap sample counts always fail pre-flight */
        {
            size_t over_n =
                (size_t)rate * 600u + 1u +
                (size_t)(splitmix64_next(&rng) % 100000u);
            static const int16_t dummy_pbt[1] = {0};
            AdsPcmPlaybackBuffer bad_buf;
            ISO_CHECK(ads_pcm_playback_buffer_new(
                          dummy_pbt, over_n, fmt, &bad_buf, NULL, 0) ==
                      ADS_ERR_INVALID_SAMPLES);
            ISO_CHECK(bad_buf.samples == NULL && bad_buf.sample_count == 0u);
        }
    }
}

int main(void) {
    test_baseline_contract();
    test_hostile_null_pointers();
    test_hostile_bounds_and_overflow();
    test_hostile_double_free_and_uaf();
    test_pbt_splitmix64();
    return ISO_TEST_RESULT();
}
