// Tests for audio-device-sink, using the header-only iso_test.h harness (pure
// ISO). Cases mirror the Rust crate's own unit tests, plus VER15 hostile
// adversarial tests (CWE-20, CWE-190, CWE-400) and SplitMix64 PBT tests.
#include "iso_test.h"

#include <cstdint>
#include <limits>
#include <string>
#include <utility>
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

static std::uint64_t splitmix64_next(std::uint64_t &state) {
    std::uint64_t z = (state += UINT64_C(0x9E3779B97F4A7C15));
    z = (z ^ (z >> 30)) * UINT64_C(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)) * UINT64_C(0x94D049BB133111EB);
    return z ^ (z >> 31);
}

static void test_baseline_contract() {
    using ads::AudioSinkError;
    using ads::ErrorKind;
    using ads::NoopAudioSink;
    using ads::PcmFormat;
    using ads::PcmPlaybackBuffer;
    using ads::PlaybackReport;

    // ── Constants ────────────────────────────────────────────────────────────
    ISO_CHECK_STR_EQ(ads::kVersion, "0.1.0");
    ISO_CHECK_EQ_UINT(ads::kMinSampleRateHz, 1u);
    ISO_CHECK_EQ_UINT(ads::MIN_SAMPLE_RATE_HZ, 1u);
    ISO_CHECK_EQ_UINT(ads::kMaxSampleRateHz, 384000u);
    ISO_CHECK_EQ_UINT(ads::MAX_SAMPLE_RATE_HZ, 384000u);
    ISO_CHECK_EQ_UINT(ads::kSupportedBitDepth, 16u);
    ISO_CHECK_EQ_UINT(ads::kSupportedChannelCount, 1u);
    ISO_CHECK_EQ_UINT(ads::kMaxChannels, 1u);
    ISO_CHECK_EQ_UINT(ads::MAX_CHANNELS, 1u);
    ISO_CHECK_EQ_UINT(ads::kMaxSamples, 230400000UL);
    ISO_CHECK_EQ_UINT(ads::MAX_SAMPLES, 230400000UL);
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
        ISO_CHECK_EQ_UINT(buf.byte_size(), 8u);
        ISO_CHECK(buf.samples()[3] == 32000);
        ISO_CHECK_EQ_DBL(buf.duration_seconds(), 0.0005, 1e-9);
        ISO_CHECK(buf.format() == fmt);

        // empty buffer
        PcmPlaybackBuffer empty(std::vector<std::int16_t>{}, fmt);
        ISO_CHECK(empty.is_empty());
        ISO_CHECK_EQ_UINT(empty.byte_size(), 0u);
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
}

// Hostile nullptr / const char* safety checks.
static void test_hostile_nullptr_and_cstr() {
    using ads::AudioSinkError;
    using ads::ErrorKind;
    using ads::NoopAudioSink;
    using ads::PcmFormat;
    using ads::PcmPlaybackBuffer;
    using ads::PlaybackReport;

    PcmFormat fmt = PcmFormat::create(44100, 1, 16);
    const std::int16_t raw[4] = {10, -20, 30, -40};

    // from_raw with nullptr and count == 0 succeeds cleanly
    {
        PcmPlaybackBuffer empty = PcmPlaybackBuffer::from_raw(nullptr, 0, fmt);
        ISO_CHECK(empty.is_empty());
        ISO_CHECK_EQ_UINT(empty.byte_size(), 0u);
    }

    // from_raw with valid ptr copies samples cleanly
    {
        PcmPlaybackBuffer copied = PcmPlaybackBuffer::from_raw(raw, 4, fmt);
        ISO_CHECK_EQ_UINT(copied.sample_count(), 4u);
        ISO_CHECK_EQ_UINT(copied.byte_size(), 8u);
        ISO_CHECK_EQ_INT(copied.samples()[1], -20);
    }

    // from_raw with nullptr and count > 0 must reject without dereferencing
    ISO_CHECK(throws_kind(
        [&] { (void)PcmPlaybackBuffer::from_raw(nullptr, 4, fmt); },
        ErrorKind::InvalidSamples));
    ISO_CHECK(throws_kind(
        [&] {
            (void)PcmPlaybackBuffer::from_raw(
                nullptr, (std::numeric_limits<std::size_t>::max)(), fmt);
        },
        ErrorKind::InvalidSamples));

    // nullptr backend_name in NoopAudioSink & PlaybackReport::for_buffer
    ISO_CHECK(throws_kind(
        [] {
            const char *null_name = nullptr;
            NoopAudioSink sink(null_name);
        },
        ErrorKind::BackendFailure));
    {
        PcmPlaybackBuffer buf(std::vector<std::int16_t>{1, 2}, fmt);
        ISO_CHECK(throws_kind(
            [&] {
                const char *null_name = nullptr;
                (void)PlaybackReport::for_buffer(buf, null_name);
            },
            ErrorKind::BackendFailure));
    }

    // AudioSinkError with nullptr const char* must not trigger std::string UB
    {
        const char *null_msg = nullptr;
        AudioSinkError e1 = AudioSinkError::invalid_format(null_msg);
        AudioSinkError e2 = AudioSinkError::invalid_samples(null_msg);
        AudioSinkError e3 = AudioSinkError::unsupported_platform(null_msg);
        AudioSinkError e4 = AudioSinkError::backend_unavailable(null_msg);
        AudioSinkError e5 = AudioSinkError::backend_failure(null_msg);
        ISO_CHECK(e1.kind() == ErrorKind::InvalidFormat);
        ISO_CHECK(e2.kind() == ErrorKind::InvalidSamples);
        ISO_CHECK(e3.kind() == ErrorKind::UnsupportedPlatform);
        ISO_CHECK(e4.kind() == ErrorKind::BackendUnavailable);
        ISO_CHECK(e5.kind() == ErrorKind::BackendFailure);
    }

    // CWE-400: bounded_strlen and oversized const char* / std::string caps
    {
        ISO_CHECK_EQ_UINT(ads::MAX_ERROR_MESSAGE_BYTES, 256u);
        ISO_CHECK_EQ_UINT(ads::MAX_BACKEND_NAME_BYTES, 128u);
        ISO_CHECK_EQ_UINT(ads::detail::bounded_strlen(nullptr, 10u), 0u);
        ISO_CHECK_EQ_UINT(ads::detail::bounded_strlen("", 10u), 0u);
        ISO_CHECK_EQ_UINT(ads::detail::bounded_strlen("abc", 10u), 3u);

        std::string exact_backend(ads::MAX_BACKEND_NAME_BYTES, 'b');
        std::string over_backend(ads::MAX_BACKEND_NAME_BYTES + 1u, 'b');
        ISO_CHECK_EQ_UINT(
            ads::detail::bounded_strlen(exact_backend.c_str(),
                                        ads::MAX_BACKEND_NAME_BYTES),
            ads::MAX_BACKEND_NAME_BYTES);
        ISO_CHECK_EQ_UINT(
            ads::detail::bounded_strlen(over_backend.c_str(),
                                        ads::MAX_BACKEND_NAME_BYTES),
            ads::MAX_BACKEND_NAME_BYTES + 1u);

        // Exact boundary succeeds; +1 byte rejects via both const char* and
        // std::string without unbounded allocation.
        NoopAudioSink ok_sink_cstr(exact_backend.c_str());
        NoopAudioSink ok_sink_str(exact_backend);
        ISO_CHECK_EQ_UINT(ok_sink_cstr.backend_name().size(),
                          ads::MAX_BACKEND_NAME_BYTES);
        ISO_CHECK_EQ_UINT(ok_sink_str.backend_name().size(),
                          ads::MAX_BACKEND_NAME_BYTES);

        ISO_CHECK(throws_kind(
            [&] { NoopAudioSink sink(over_backend.c_str()); },
            ErrorKind::BackendFailure));
        ISO_CHECK(throws_kind(
            [&] { NoopAudioSink sink(over_backend); },
            ErrorKind::BackendFailure));

        PcmPlaybackBuffer buf(std::vector<std::int16_t>{1, 2}, fmt);
        PlaybackReport ok_rep =
            PlaybackReport::for_buffer(buf, exact_backend.c_str());
        ISO_CHECK_EQ_UINT(ok_rep.backend_name.size(),
                          ads::MAX_BACKEND_NAME_BYTES);
        ISO_CHECK(throws_kind(
            [&] {
                (void)PlaybackReport::for_buffer(buf, over_backend.c_str());
            },
            ErrorKind::BackendFailure));
        ISO_CHECK(throws_kind(
            [&] { (void)PlaybackReport::for_buffer(buf, over_backend); },
            ErrorKind::BackendFailure));

        // Oversized AudioSinkError messages are bounded to MAX_ERROR_MESSAGE_BYTES
        std::string huge_err(1024u, 'E');
        AudioSinkError err_cstr =
            AudioSinkError::invalid_format(huge_err.c_str());
        AudioSinkError err_str = AudioSinkError::invalid_samples(huge_err);
        const std::string what_cstr = err_cstr.what();
        const std::string what_str = err_str.what();
        ISO_CHECK(what_cstr.size() <=
                  std::string("invalid PCM format: ").size() +
                      ads::MAX_ERROR_MESSAGE_BYTES);
        ISO_CHECK(what_str.size() <=
                  std::string("invalid PCM samples: ").size() +
                      ads::MAX_ERROR_MESSAGE_BYTES);
    }
}

// CWE-20 / CWE-190 / CWE-400: Bounds, wrapped negatives, SIZE_MAX, non-finite.
static void test_hostile_bounds_and_overflow() {
    using ads::ErrorKind;
    using ads::PcmFormat;
    using ads::PcmPlaybackBuffer;

    const std::int16_t dummy[1] = {0};

    // Zero, wrapped-negative, and excessive sample rates / channels / depths
    ISO_CHECK(throws_kind([] { PcmFormat::create(0u, 1u, 16u); },
                          ErrorKind::InvalidFormat));
    ISO_CHECK(throws_kind(
        [] { PcmFormat::create(static_cast<std::uint32_t>(-1), 1u, 16u); },
        ErrorKind::InvalidFormat));
    ISO_CHECK(throws_kind(
        [] { PcmFormat::create(ads::MAX_SAMPLE_RATE_HZ + 1u, 1u, 16u); },
        ErrorKind::InvalidFormat));
    ISO_CHECK(throws_kind([] { PcmFormat::create(44100u, 0u, 16u); },
                          ErrorKind::InvalidFormat));
    ISO_CHECK(throws_kind(
        [] { PcmFormat::create(44100u, static_cast<std::uint16_t>(-1), 16u); },
        ErrorKind::InvalidFormat));
    ISO_CHECK(throws_kind([] { PcmFormat::create(44100u, 1u, 0u); },
                          ErrorKind::InvalidFormat));
    ISO_CHECK(throws_kind([] { PcmFormat::create(44100u, 1u, 8u); },
                          ErrorKind::InvalidFormat));
    ISO_CHECK(throws_kind(
        [] { PcmFormat::create(44100u, 1u, static_cast<std::uint16_t>(-1)); },
        ErrorKind::InvalidFormat));

    // Checked multiplication helper invariants
    {
        std::size_t out = 0;
        const std::size_t smax = (std::numeric_limits<std::size_t>::max)();
        ISO_CHECK(!ads::detail::checked_mul_size(smax, 2u, out));
        ISO_CHECK(!ads::detail::checked_mul_size(smax / 2u + 1u, 2u, out));
        ISO_CHECK(ads::detail::checked_mul_size(0u, smax, out) && out == 0u);
        ISO_CHECK(ads::detail::checked_mul_size(ads::MAX_SAMPLES, 2u, out) &&
                  out == ads::MAX_SAMPLES * 2u);

        std::uint64_t uout = 0;
        const std::uint64_t umax = (std::numeric_limits<std::uint64_t>::max)();
        ISO_CHECK(!ads::detail::checked_mul_u64(umax, 2u, uout));
        ISO_CHECK(ads::detail::checked_mul_u64(384000ULL, 600ULL, uout) &&
                  uout == 230400000ULL);
    }

    // Pre-flight SIZE_MAX and MAX_SAMPLES + 1 rejection before allocation
    {
        PcmFormat max_fmt = PcmFormat::create(ads::MAX_SAMPLE_RATE_HZ, 1u, 16u);
        const std::size_t smax = (std::numeric_limits<std::size_t>::max)();
        ISO_CHECK(throws_kind(
            [&] { (void)PcmPlaybackBuffer::from_raw(dummy, smax, max_fmt); },
            ErrorKind::InvalidSamples));
        ISO_CHECK(throws_kind(
            [&] {
                (void)PcmPlaybackBuffer::from_raw(dummy, smax / 2u, max_fmt);
            },
            ErrorKind::InvalidSamples));
        ISO_CHECK(throws_kind(
            [&] {
                (void)PcmPlaybackBuffer::from_raw(
                    dummy, (smax / sizeof(std::int16_t)) + 1u, max_fmt);
            },
            ErrorKind::InvalidSamples));
        ISO_CHECK(throws_kind(
            [&] {
                (void)PcmPlaybackBuffer::from_raw(
                    dummy, ads::MAX_SAMPLES + 1u, max_fmt);
            },
            ErrorKind::InvalidSamples));
        ISO_CHECK_EQ_UINT(max_fmt.max_blocking_samples(), 230400000ULL);
    }

    // Per-rate cap boundary: 44100 * 600 + 1 rejected pre-flight
    {
        PcmFormat fmt44 = PcmFormat::create(44100u, 1u, 16u);
        ISO_CHECK_EQ_UINT(fmt44.max_blocking_samples(), 26460000ULL);
        ISO_CHECK(throws_kind(
            [&] {
                (void)PcmPlaybackBuffer::from_raw(
                    dummy, static_cast<std::size_t>(44100u) * 600u + 1u,
                    fmt44);
            },
            ErrorKind::InvalidSamples));
    }

    // Non-finite IEEE-754 checks
    {
        const double qnan = std::numeric_limits<double>::quiet_NaN();
        const double pinf = std::numeric_limits<double>::infinity();
        const double ninf = -std::numeric_limits<double>::infinity();
        ISO_CHECK(!ads::detail::is_finite(qnan));
        ISO_CHECK(!ads::detail::is_finite(pinf));
        ISO_CHECK(!ads::detail::is_finite(ninf));
        ISO_CHECK(ads::detail::is_finite(0.0));
        ISO_CHECK(ads::detail::is_finite(600.0));
    }
}

// Moved-from / lifetime and deep-copy isolation invariants.
static void test_hostile_lifetime_invariants() {
    using ads::NoopAudioSink;
    using ads::PcmFormat;
    using ads::PcmPlaybackBuffer;
    using ads::PlaybackReport;

    PcmFormat fmt = PcmFormat::create(48000u, 1u, 16u);
    std::vector<std::int16_t> source = {100, -200, 300, -400};
    PcmPlaybackBuffer buf1(source, fmt);
    source[0] = 9999;
    ISO_CHECK_EQ_INT(buf1.samples()[0], 100);

    PcmPlaybackBuffer buf2 = std::move(buf1);
    ISO_CHECK_EQ_UINT(buf2.sample_count(), 4u);
    ISO_CHECK_EQ_UINT(buf1.sample_count(), 0u);
    ISO_CHECK_EQ_UINT(buf1.frame_count(), 0u);
    ISO_CHECK(buf1.is_empty());
    ISO_CHECK(ads::detail::is_finite(buf1.duration_seconds()));
    ISO_CHECK_EQ_DBL(buf1.duration_seconds(), 0.0, 1e-12);

    NoopAudioSink sink("move-noop");
    PlaybackReport r_moved = sink.play_blocking(buf1);
    ISO_CHECK_EQ_UINT(r_moved.frames_played, 0u);
    PlaybackReport r_owned = sink.play_blocking(buf2);
    ISO_CHECK_EQ_UINT(r_owned.frames_played, 4u);
}

// SplitMix64 Property-Based Tests across format, buffer, duration, and sink.
static void test_pbt_splitmix64() {
    using ads::ErrorKind;
    using ads::NoopAudioSink;
    using ads::PcmFormat;
    using ads::PcmPlaybackBuffer;
    using ads::PlaybackReport;

    std::uint64_t rng = UINT64_C(0xA511E9B3D4C2017F);
    NoopAudioSink sink("pbt-noop");

    for (std::size_t trial = 0; trial < 256; ++trial) {
        std::uint32_t rate = static_cast<std::uint32_t>(
                                 splitmix64_next(rng) % ads::MAX_SAMPLE_RATE_HZ) +
                             1u;
        std::size_t max_n = static_cast<std::size_t>(rate) * 600u;
        std::size_t n = static_cast<std::size_t>(splitmix64_next(rng) % 513u);
        if (n > max_n) n = max_n;

        PcmFormat fmt = PcmFormat::create(rate, 1u, 16u);
        ISO_CHECK_EQ_UINT(fmt.sample_width_bytes(), 2u);

        std::vector<std::int16_t> samples(n);
        for (std::size_t i = 0; i < n; ++i) {
            samples[i] = static_cast<std::int16_t>(
                static_cast<std::uint16_t>(splitmix64_next(rng) & 0xFFFFu));
        }

        PcmPlaybackBuffer buf =
            PcmPlaybackBuffer::from_raw(n == 0 ? nullptr : samples.data(), n,
                                        fmt);
        PcmPlaybackBuffer buf_vec(samples, fmt);
        ISO_CHECK(buf == buf_vec);

        ISO_CHECK_EQ_UINT(buf.sample_count(), n);
        ISO_CHECK_EQ_UINT(buf.frame_count(), n);
        ISO_CHECK(buf.is_empty() == (n == 0));
        ISO_CHECK_EQ_UINT(buf.byte_size(), n * sizeof(std::int16_t));

        double expected_dur =
            static_cast<double>(n) / static_cast<double>(rate);
        double actual_dur = buf.duration_seconds();
        ISO_CHECK(ads::detail::is_finite(actual_dur));
        ISO_CHECK(actual_dur >= 0.0 && actual_dur <= 600.0);
        ISO_CHECK_EQ_DBL(actual_dur, expected_dur, 1e-12);

        PlaybackReport report = sink.play_blocking(buf);
        ISO_CHECK_EQ_UINT(report.frames_played, n);
        ISO_CHECK_EQ_UINT(report.sample_rate_hz, rate);
        ISO_CHECK_EQ_UINT(report.channel_count, 1u);
        ISO_CHECK_EQ_DBL(report.duration_seconds, actual_dur, 1e-12);
        ISO_CHECK_STR_EQ(report.backend_name.c_str(), "pbt-noop");

        // Hostile PBT: invalid rates always throw InvalidFormat
        std::uint32_t bad_rate =
            ads::MAX_SAMPLE_RATE_HZ + 1u +
            static_cast<std::uint32_t>(splitmix64_next(rng) % 1000000u);
        ISO_CHECK(throws_kind(
            [&] { (void)PcmFormat::create(bad_rate, 1u, 16u); },
            ErrorKind::InvalidFormat));

        // Hostile PBT: over-cap lengths always throw InvalidSamples pre-flight
        std::size_t over_n =
            static_cast<std::size_t>(rate) * 600u + 1u +
            static_cast<std::size_t>(splitmix64_next(rng) % 100000u);
        static const std::int16_t dummy_pbt[1] = {0};
        ISO_CHECK(throws_kind(
            [&] { (void)PcmPlaybackBuffer::from_raw(dummy_pbt, over_n, fmt); },
            ErrorKind::InvalidSamples));
    }
}

int main() {
    test_baseline_contract();
    test_hostile_nullptr_and_cstr();
    test_hostile_bounds_and_overflow();
    test_hostile_lifetime_invariants();
    test_pbt_splitmix64();
    return ISO_TEST_RESULT();
}
