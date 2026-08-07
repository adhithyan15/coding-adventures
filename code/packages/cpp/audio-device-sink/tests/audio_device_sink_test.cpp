// Tests for audio-device-sink, using the header-only iso_test.h harness (pure
// ISO). Cases mirror the Rust crate's own unit tests.
#include "iso_test.h"

#include <cstdint>
#include <string>
#include <vector>

#include "audio_device_sink.hpp"

namespace ads = ca::audio_device_sink;

// Did calling `fn` throw an AudioSinkError of the given kind?
template <class Fn>
static bool throws_kind(Fn fn, ads::ErrorKind kind) {
    try {
        fn();
    } catch (const ads::AudioSinkError &e) {
        return e.kind() == kind;
    }
    return false;
}

int main() {
    using ads::AudioSinkError;
    using ads::ErrorKind;
    using ads::NoopAudioSink;
    using ads::PcmFormat;
    using ads::PcmPlaybackBuffer;
    using ads::PlaybackReport;

    // ── Constants ────────────────────────────────────────────────────────────
    ISO_CHECK_STR_EQ(ads::kVersion, "0.1.0");
    ISO_CHECK_EQ_UINT(ads::kMaxSampleRateHz, 384000u);
    ISO_CHECK_EQ_UINT(ads::kSupportedBitDepth, 16u);
    ISO_CHECK_EQ_UINT(ads::kSupportedChannelCount, 1u);
    ISO_CHECK_EQ_DBL(ads::kMaxBlockingDurationSeconds, 600.0, 1e-9);

    // ── PcmFormat validation ─────────────────────────────────────────────────
    {
        PcmFormat fmt = PcmFormat::create(48000, 1, 16);
        ISO_CHECK(fmt.sample_rate_hz == 48000u && fmt.channel_count == 1 &&
                  fmt.bit_depth == 16);
        ISO_CHECK_EQ_UINT(fmt.sample_width_bytes(), 2u);

        ISO_CHECK(throws_kind([] { PcmFormat::create(0, 1, 16); },
                              ErrorKind::InvalidFormat));
        ISO_CHECK(throws_kind([] { PcmFormat::create(384001u, 1, 16); },
                              ErrorKind::InvalidFormat));
        ISO_CHECK(throws_kind([] { PcmFormat::create(48000, 2, 16); },
                              ErrorKind::InvalidFormat));
        ISO_CHECK(throws_kind([] { PcmFormat::create(48000, 1, 24); },
                              ErrorKind::InvalidFormat));
        // boundary: exactly the max rate is allowed
        ISO_CHECK(PcmFormat::create(384000u, 1, 16).sample_rate_hz == 384000u);

        // the thrown message carries the Display prefix + detail
        try {
            PcmFormat::create(0, 1, 16);
        } catch (const AudioSinkError &e) {
            std::string what = e.what();
            ISO_CHECK(what.find("invalid PCM format") != std::string::npos);
            ISO_CHECK(what.find("greater than zero") != std::string::npos);
        }
    }

    // ── PcmPlaybackBuffer ────────────────────────────────────────────────────
    {
        PcmFormat fmt = PcmFormat::create(8000, 1, 16);
        std::vector<std::int16_t> samples = {0, 100, -100, 32000};
        PcmPlaybackBuffer buf(samples, fmt);
        ISO_CHECK(buf.sample_count() == 4);
        ISO_CHECK(buf.frame_count() == 4);  // mono
        ISO_CHECK(!buf.is_empty());
        ISO_CHECK(buf.samples()[3] == 32000);
        ISO_CHECK_EQ_DBL(buf.duration_seconds(), 0.0005, 1e-9);
        ISO_CHECK(buf.format() == fmt);

        // empty buffer
        PcmPlaybackBuffer empty(std::vector<std::int16_t>{}, fmt);
        ISO_CHECK(empty.is_empty());
        ISO_CHECK_EQ_DBL(empty.duration_seconds(), 0.0, 1e-12);

        // invalid format rejected
        ISO_CHECK(throws_kind(
            [&] {
                PcmPlaybackBuffer(samples, PcmFormat{0, 1, 16});
            },
            ErrorKind::InvalidFormat));

        // V1 size cap: > cap samples rejected, exactly cap allowed
        PcmFormat tiny{1, 1, 16};  // cap = 1 Hz * 600 s = 600 samples
        ISO_CHECK(throws_kind(
            [&] {
                PcmPlaybackBuffer(std::vector<std::int16_t>(601, 0), tiny);
            },
            ErrorKind::InvalidSamples));
        PcmPlaybackBuffer at_cap(std::vector<std::int16_t>(600, 0), tiny);
        ISO_CHECK(at_cap.sample_count() == 600);
    }

    // ── PlaybackReport & NoopAudioSink ───────────────────────────────────────
    {
        PcmFormat fmt = PcmFormat::create(48000, 1, 16);
        PcmPlaybackBuffer buf(std::vector<std::int16_t>{1, 2, 3}, fmt);

        PlaybackReport report =
            PlaybackReport::for_buffer(buf, "test-backend");
        ISO_CHECK(report.frames_played == 3 && report.sample_rate_hz == 48000u);
        ISO_CHECK(report.channel_count == 1);
        ISO_CHECK_STR_EQ(report.backend_name.c_str(), "test-backend");

        NoopAudioSink sink("noop");
        ISO_CHECK_STR_EQ(sink.backend_name().c_str(), "noop");
        // usable through the AudioSink interface (the trait-object analog)
        const ads::AudioSink &as_iface = sink;
        PlaybackReport r2 = as_iface.play_blocking(buf);
        ISO_CHECK(r2.frames_played == 3);
        ISO_CHECK_STR_EQ(r2.backend_name.c_str(), "noop");
    }

    return ISO_TEST_RESULT();
}
