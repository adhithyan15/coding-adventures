// audio_device_sink.hpp — backend-neutral PCM playback primitives, ISO C++17.
// ===========================================================================
//
// A faithful, header-only port of the Rust `audio-device-sink` crate, in
// namespace `ca::audio_device_sink`. Intentionally boring: it does NOT open
// devices, talk to Core Audio, parse notes, or generate waves. It defines the
// shared contract every real audio backend must obey — a validated PCM format,
// an owned playback buffer, a playback report, an `AudioSink` interface, and a
// no-op sink for tests.
//
// V1 scope (matching the Rust crate): mono, signed 16-bit PCM only.
//
// Hardened against VER15 CWEs:
// - CWE-20 / CWE-400: Strict MIN_SAMPLE_RATE_HZ..=MAX_SAMPLE_RATE_HZ,
//   MAX_CHANNELS, and MAX_SAMPLES pre-flight bounds; explicit nullptr /
//   const char* safety across string/pointer APIs.
// - CWE-190: Overflow-checked std::size_t / std::uint64_t frame, duration, and
//   byte-size arithmetic before allocation.
//
// Pure ISO C++17 — no <cmath>, no compiler extensions.
#ifndef AUDIO_DEVICE_SINK_HPP
#define AUDIO_DEVICE_SINK_HPP

#include <cstddef>
#include <cstdint>
#include <limits>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace ca {
namespace audio_device_sink {

// ── Constants ────────────────────────────────────────────────────────────────
inline constexpr const char *kVersion = "0.1.0";
// V1 keeps buffers small enough that accidental calls don't block for hours.
inline constexpr double kMaxBlockingDurationSeconds = 10.0 * 60.0;
// Integer blocking-duration limit (seconds) for exact checked bounds.
inline constexpr std::uint64_t kMaxBlockingDurationSecondsInt = 600;
// Minimum valid sample rate (Hz).
inline constexpr std::uint32_t kMinSampleRateHz = 1;
// Practical ceiling for high-resolution audio interfaces.
inline constexpr std::uint32_t kMaxSampleRateHz = 384000;
// Only signed 16-bit PCM is supported in the first slice.
inline constexpr std::uint16_t kSupportedBitDepth = 16;
// Only mono PCM is supported until interleaved channels arrive.
inline constexpr std::uint16_t kSupportedChannelCount = 1;
inline constexpr std::uint16_t kMaxChannels = kSupportedChannelCount;
// Pre-flight ceiling: 384,000 Hz * 600 s * 1 channel = 230,400,000 samples.
inline constexpr std::size_t kMaxSamples =
    static_cast<std::size_t>(384000ULL * 600ULL);
// Bounded string ceilings (CWE-400) to avoid unbounded strlen / heap growth.
inline constexpr std::size_t kMaxErrorMessageBytes = 256;
inline constexpr std::size_t kMaxBackendNameBytes = 128;

// VER15 canonical bound aliases.
inline constexpr std::uint32_t MIN_SAMPLE_RATE_HZ = kMinSampleRateHz;
inline constexpr std::uint32_t MAX_SAMPLE_RATE_HZ = kMaxSampleRateHz;
inline constexpr std::uint16_t MAX_CHANNELS = kMaxChannels;
inline constexpr std::size_t MAX_SAMPLES = kMaxSamples;
inline constexpr std::size_t MAX_ERROR_MESSAGE_BYTES = kMaxErrorMessageBytes;
inline constexpr std::size_t MAX_BACKEND_NAME_BYTES = kMaxBackendNameBytes;

namespace detail {

inline bool checked_mul_size(std::size_t a, std::size_t b,
                             std::size_t &out) noexcept {
    if (a != 0 && b > (std::numeric_limits<std::size_t>::max)() / a) {
        return false;
    }
    out = a * b;
    return true;
}

inline bool checked_mul_u64(std::uint64_t a, std::uint64_t b,
                            std::uint64_t &out) noexcept {
    if (a != 0 && b > (std::numeric_limits<std::uint64_t>::max)() / a) {
        return false;
    }
    out = a * b;
    return true;
}

inline bool is_finite(double x) noexcept {
    // Pure IEEE-754 finiteness without <cmath>.
    return (x == x) && ((x - x) == 0.0);
}

// Scan at most max_len + 1 bytes so lengths > max_len are caught in O(max_len)
// without walking an unbounded C-string.
inline std::size_t bounded_strlen(const char *s, std::size_t max_len) noexcept {
    if (s == nullptr) return 0;
    std::size_t len = 0;
    while (len <= max_len && s[len] != '\0') {
        ++len;
    }
    return len;
}

inline std::string bounded_cstr(const char *s, std::size_t max_len) {
    if (s == nullptr) return std::string();
    std::size_t len = bounded_strlen(s, max_len);
    if (len > max_len) len = max_len;
    return std::string(s, len);
}

inline std::string bounded_str(const std::string &s, std::size_t max_len) {
    if (s.size() <= max_len) return s;
    return s.substr(0, max_len);
}

}  // namespace detail

// ── Error ────────────────────────────────────────────────────────────────────

// The kind of an audio-sink error (mirrors the Rust enum's variants).
enum class ErrorKind {
    InvalidFormat,
    InvalidSamples,
    UnsupportedPlatform,
    BackendUnavailable,
    BackendFailure
};

// Shared error type for validation and backend failures.
class AudioSinkError : public std::runtime_error {
   public:
    AudioSinkError(ErrorKind kind, const std::string &message)
        : std::runtime_error(
              prefix_for(kind) +
              detail::bounded_str(message, kMaxErrorMessageBytes)),
          kind_(kind) {}
    AudioSinkError(ErrorKind kind, const char *message)
        : std::runtime_error(
              prefix_for(kind) +
              detail::bounded_cstr(message, kMaxErrorMessageBytes)),
          kind_(kind) {}
    ErrorKind kind() const noexcept { return kind_; }

    static AudioSinkError invalid_format(const std::string &m) {
        return AudioSinkError(ErrorKind::InvalidFormat, m);
    }
    static AudioSinkError invalid_format(const char *m) {
        return AudioSinkError(ErrorKind::InvalidFormat, m);
    }
    static AudioSinkError invalid_samples(const std::string &m) {
        return AudioSinkError(ErrorKind::InvalidSamples, m);
    }
    static AudioSinkError invalid_samples(const char *m) {
        return AudioSinkError(ErrorKind::InvalidSamples, m);
    }
    static AudioSinkError unsupported_platform(const std::string &m) {
        return AudioSinkError(ErrorKind::UnsupportedPlatform, m);
    }
    static AudioSinkError unsupported_platform(const char *m) {
        return AudioSinkError(ErrorKind::UnsupportedPlatform, m);
    }
    static AudioSinkError backend_unavailable(const std::string &m) {
        return AudioSinkError(ErrorKind::BackendUnavailable, m);
    }
    static AudioSinkError backend_unavailable(const char *m) {
        return AudioSinkError(ErrorKind::BackendUnavailable, m);
    }
    static AudioSinkError backend_failure(const std::string &m) {
        return AudioSinkError(ErrorKind::BackendFailure, m);
    }
    static AudioSinkError backend_failure(const char *m) {
        return AudioSinkError(ErrorKind::BackendFailure, m);
    }

   private:
    static std::string prefix_for(ErrorKind kind) {
        switch (kind) {
            case ErrorKind::InvalidFormat: return "invalid PCM format: ";
            case ErrorKind::InvalidSamples: return "invalid PCM samples: ";
            case ErrorKind::UnsupportedPlatform: return "unsupported platform: ";
            case ErrorKind::BackendUnavailable:
                return "audio backend unavailable: ";
            case ErrorKind::BackendFailure: return "audio backend failure: ";
        }
        return "";
    }
    ErrorKind kind_;
};

// ── PcmFormat ────────────────────────────────────────────────────────────────

// Metadata telling a sink how to interpret integer samples.
struct PcmFormat {
    std::uint32_t sample_rate_hz = 0;
    std::uint16_t channel_count = 0;
    std::uint16_t bit_depth = 0;

    // Check the V1 sink constraints; throws AudioSinkError on violation.
    void validate() const {
        if (sample_rate_hz < kMinSampleRateHz)
            throw AudioSinkError::invalid_format(
                "sample_rate_hz must be greater than zero");
        if (sample_rate_hz > kMaxSampleRateHz)
            throw AudioSinkError::invalid_format(
                "sample_rate_hz must be <= " + std::to_string(kMaxSampleRateHz) +
                ", got " + std::to_string(sample_rate_hz));
        if (channel_count != kSupportedChannelCount ||
            channel_count > kMaxChannels)
            throw AudioSinkError::invalid_format(
                "only mono PCM is supported in V1, got " +
                std::to_string(channel_count) + " channels");
        if (bit_depth != kSupportedBitDepth)
            throw AudioSinkError::invalid_format(
                "only signed 16-bit PCM is supported in V1, got " +
                std::to_string(bit_depth) + " bits");
    }

    // Construct and validate a PCM format (throws on invalid).
    static PcmFormat create(std::uint32_t sample_rate_hz,
                            std::uint16_t channel_count,
                            std::uint16_t bit_depth) {
        PcmFormat format{sample_rate_hz, channel_count, bit_depth};
        format.validate();
        return format;
    }

    // Bytes occupied by one PCM sample.
    std::size_t sample_width_bytes() const {
        return static_cast<std::size_t>(bit_depth / 8);
    }

    // Exact checked maximum sample count allowed for blocking playback.
    std::uint64_t max_blocking_samples() const {
        validate();
        std::uint64_t max_frames = 0;
        std::uint64_t max_samples = 0;
        if (!detail::checked_mul_u64(
                static_cast<std::uint64_t>(sample_rate_hz),
                kMaxBlockingDurationSecondsInt, max_frames) ||
            !detail::checked_mul_u64(max_frames,
                                     static_cast<std::uint64_t>(channel_count),
                                     max_samples)) {
            throw AudioSinkError::invalid_format(
                "sample count limit calculation overflow");
        }
        return max_samples;
    }

    bool operator==(const PcmFormat &o) const {
        return sample_rate_hz == o.sample_rate_hz &&
               channel_count == o.channel_count && bit_depth == o.bit_depth;
    }
    bool operator!=(const PcmFormat &o) const { return !(*this == o); }
};

// ── PcmPlaybackBuffer ────────────────────────────────────────────────────────

// Owned PCM samples plus the metadata a backend needs.
class PcmPlaybackBuffer {
   public:
    // Construct and validate an owned playback buffer (throws on invalid
    // format or a buffer exceeding the V1 blocking-duration / MAX_SAMPLES cap).
    PcmPlaybackBuffer(std::vector<std::int16_t> samples, PcmFormat format)
        : samples_(std::move(samples)), format_(format) {
        format_.validate();
        validate_count(samples_.size(), format_);
    }

    // Construct from a raw pointer + count with pre-flight bounds and nullptr
    // checks BEFORE allocating. `samples` may be nullptr iff `sample_count == 0`.
    static PcmPlaybackBuffer from_raw(const std::int16_t *samples,
                                      std::size_t sample_count,
                                      PcmFormat format) {
        format.validate();
        if (sample_count > 0 && samples == nullptr) {
            throw AudioSinkError::invalid_samples(
                "samples pointer must not be null when sample_count > 0");
        }
        validate_count(sample_count, format);
        std::vector<std::int16_t> owned;
        if (sample_count > 0) {
            owned.assign(samples, samples + sample_count);
        }
        return PcmPlaybackBuffer(std::move(owned), format);
    }

    const std::vector<std::int16_t> &samples() const { return samples_; }
    PcmFormat format() const { return format_; }
    std::size_t sample_count() const { return samples_.size(); }
    // V1 is mono, so frames and samples are the same count; safe if zeroed.
    std::size_t frame_count() const {
        if (format_.channel_count == 0) return 0;
        return sample_count() / format_.channel_count;
    }
    bool is_empty() const { return samples_.empty(); }
    // Overflow-checked byte length of the sample payload.
    std::size_t byte_size() const {
        std::size_t bytes = 0;
        if (!detail::checked_mul_size(samples_.size(),
                                      sizeof(std::int16_t), bytes)) {
            throw AudioSinkError::invalid_samples(
                "sample buffer byte size overflow");
        }
        return bytes;
    }
    // Intended playback duration in seconds (always finite, 0.0 if invalid).
    double duration_seconds() const {
        if (format_.sample_rate_hz == 0 || format_.channel_count == 0) {
            return 0.0;
        }
        double duration = static_cast<double>(frame_count()) /
                          static_cast<double>(format_.sample_rate_hz);
        return detail::is_finite(duration) ? duration : 0.0;
    }

    bool operator==(const PcmPlaybackBuffer &o) const {
        return samples_ == o.samples_ && format_ == o.format_;
    }
    bool operator!=(const PcmPlaybackBuffer &o) const { return !(*this == o); }

   private:
    static void validate_count(std::size_t count, const PcmFormat &format) {
        if (count > kMaxSamples) {
            throw AudioSinkError::invalid_samples(
                "blocking playback is limited to " +
                std::to_string(kMaxBlockingDurationSeconds) + " seconds");
        }
        std::size_t byte_len = 0;
        if (!detail::checked_mul_size(count, sizeof(std::int16_t), byte_len)) {
            throw AudioSinkError::invalid_samples(
                "sample buffer byte size overflow");
        }
        (void)byte_len;
        const std::uint64_t max_samples = format.max_blocking_samples();
        if (static_cast<std::uint64_t>(count) > max_samples) {
            throw AudioSinkError::invalid_samples(
                "blocking playback is limited to " +
                std::to_string(kMaxBlockingDurationSeconds) + " seconds");
        }
    }

    std::vector<std::int16_t> samples_;
    PcmFormat format_;
};

namespace detail {

inline std::string validate_backend_name(const char *backend_name) {
    if (backend_name == nullptr) {
        throw AudioSinkError::backend_failure("backend_name must not be null");
    }
    const std::size_t len = bounded_strlen(backend_name, kMaxBackendNameBytes);
    if (len > kMaxBackendNameBytes) {
        throw AudioSinkError::backend_failure(
            "backend_name must be <= " +
            std::to_string(kMaxBackendNameBytes) + " bytes");
    }
    return std::string(backend_name, len);
}

inline std::string validate_backend_name(std::string backend_name) {
    if (backend_name.size() > kMaxBackendNameBytes) {
        throw AudioSinkError::backend_failure(
            "backend_name must be <= " +
            std::to_string(kMaxBackendNameBytes) + " bytes");
    }
    return backend_name;
}

}  // namespace detail

// ── PlaybackReport ───────────────────────────────────────────────────────────

// Result returned after a sink accepts or completes playback.
struct PlaybackReport {
    std::size_t frames_played = 0;
    std::uint32_t sample_rate_hz = 0;
    std::uint16_t channel_count = 0;
    double duration_seconds = 0.0;
    std::string backend_name;

    // Build a report mirroring one validated playback buffer.
    static PlaybackReport for_buffer(const PcmPlaybackBuffer &buffer,
                                     std::string backend_name) {
        buffer.format().validate();
        return PlaybackReport{
            buffer.frame_count(),
            buffer.format().sample_rate_hz,
            buffer.format().channel_count,
            buffer.duration_seconds(),
            detail::validate_backend_name(std::move(backend_name))};
    }
    static PlaybackReport for_buffer(const PcmPlaybackBuffer &buffer,
                                     const char *backend_name) {
        buffer.format().validate();
        return PlaybackReport{
            buffer.frame_count(),
            buffer.format().sample_rate_hz,
            buffer.format().channel_count,
            buffer.duration_seconds(),
            detail::validate_backend_name(backend_name)};
    }
};

// ── AudioSink / NoopAudioSink ────────────────────────────────────────────────

// Something that can play a PCM buffer.
class AudioSink {
   public:
    virtual ~AudioSink() = default;
    virtual PlaybackReport play_blocking(
        const PcmPlaybackBuffer &buffer) const = 0;
};

// Test and teaching sink that accepts buffers without touching a device.
class NoopAudioSink final : public AudioSink {
   public:
    explicit NoopAudioSink(std::string backend_name)
        : backend_name_(
              detail::validate_backend_name(std::move(backend_name))) {}
    explicit NoopAudioSink(const char *backend_name)
        : backend_name_(detail::validate_backend_name(backend_name)) {}
    PlaybackReport play_blocking(
        const PcmPlaybackBuffer &buffer) const override {
        return PlaybackReport::for_buffer(buffer, backend_name_);
    }
    const std::string &backend_name() const { return backend_name_; }

   private:
    std::string backend_name_;
};

}  // namespace audio_device_sink
}  // namespace ca

#endif  // AUDIO_DEVICE_SINK_HPP
