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
// Rust's `Result<T, AudioSinkError>` becomes C++ exceptions (`AudioSinkError`,
// a std::runtime_error subclass carrying a kind + message). Owned samples
// (`Vec<i16>`) become a `std::vector<std::int16_t>`.
//
// Pure ISO C++17 — no <cmath>, no compiler extensions.
#ifndef AUDIO_DEVICE_SINK_HPP
#define AUDIO_DEVICE_SINK_HPP

#include <cstddef>
#include <cstdint>
#include <stdexcept>
#include <string>
#include <vector>

namespace ca {
namespace audio_device_sink {

// ── Constants ────────────────────────────────────────────────────────────────
inline constexpr const char *kVersion = "0.1.0";
// V1 keeps buffers small enough that accidental calls don't block for hours.
inline constexpr double kMaxBlockingDurationSeconds = 10.0 * 60.0;
// Practical ceiling for high-resolution audio interfaces.
inline constexpr std::uint32_t kMaxSampleRateHz = 384000;
// Only signed 16-bit PCM is supported in the first slice.
inline constexpr std::uint16_t kSupportedBitDepth = 16;
// Only mono PCM is supported until interleaved channels arrive.
inline constexpr std::uint16_t kSupportedChannelCount = 1;

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
        : std::runtime_error(prefix_for(kind) + message), kind_(kind) {}
    ErrorKind kind() const noexcept { return kind_; }

    static AudioSinkError invalid_format(const std::string &m) {
        return AudioSinkError(ErrorKind::InvalidFormat, m);
    }
    static AudioSinkError invalid_samples(const std::string &m) {
        return AudioSinkError(ErrorKind::InvalidSamples, m);
    }
    static AudioSinkError unsupported_platform(const std::string &m) {
        return AudioSinkError(ErrorKind::UnsupportedPlatform, m);
    }
    static AudioSinkError backend_unavailable(const std::string &m) {
        return AudioSinkError(ErrorKind::BackendUnavailable, m);
    }
    static AudioSinkError backend_failure(const std::string &m) {
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
        if (sample_rate_hz == 0)
            throw AudioSinkError::invalid_format(
                "sample_rate_hz must be greater than zero");
        if (sample_rate_hz > kMaxSampleRateHz)
            throw AudioSinkError::invalid_format(
                "sample_rate_hz must be <= " + std::to_string(kMaxSampleRateHz) +
                ", got " + std::to_string(sample_rate_hz));
        if (channel_count != kSupportedChannelCount)
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
    // format or a buffer exceeding the V1 blocking-duration cap).
    PcmPlaybackBuffer(std::vector<std::int16_t> samples, PcmFormat format)
        : samples_(std::move(samples)), format_(format) {
        format_.validate();
        validate_size();
    }

    const std::vector<std::int16_t> &samples() const { return samples_; }
    PcmFormat format() const { return format_; }
    std::size_t sample_count() const { return samples_.size(); }
    // V1 is mono, so frames and samples are the same count.
    std::size_t frame_count() const {
        return sample_count() / format_.channel_count;
    }
    bool is_empty() const { return samples_.empty(); }
    // Intended playback duration in seconds.
    double duration_seconds() const {
        return static_cast<double>(frame_count()) /
               static_cast<double>(format_.sample_rate_hz);
    }

    bool operator==(const PcmPlaybackBuffer &o) const {
        return samples_ == o.samples_ && format_ == o.format_;
    }
    bool operator!=(const PcmPlaybackBuffer &o) const { return !(*this == o); }

   private:
    void validate_size() const {
        double max_samples = static_cast<double>(format_.sample_rate_hz) *
                             kMaxBlockingDurationSeconds;
        if (static_cast<double>(sample_count()) > max_samples)
            throw AudioSinkError::invalid_samples(
                "blocking playback is limited to " +
                std::to_string(kMaxBlockingDurationSeconds) + " seconds");
    }
    std::vector<std::int16_t> samples_;
    PcmFormat format_;
};

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
        return PlaybackReport{buffer.frame_count(),
                              buffer.format().sample_rate_hz,
                              buffer.format().channel_count,
                              buffer.duration_seconds(),
                              std::move(backend_name)};
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
        : backend_name_(std::move(backend_name)) {}
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
