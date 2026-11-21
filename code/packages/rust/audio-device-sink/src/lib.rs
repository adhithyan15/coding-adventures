//! # audio-device-sink
//!
//! Backend-neutral PCM playback primitives.
//!
//! This crate is intentionally boring in the best possible way: it does not
//! open devices, talk to Core Audio, parse notes, or generate waves. It defines
//! the shared contract that every real audio backend must obey.
//!
//! ## Security & Safety Hardening (VER15)
//!
//! - **Memory safety**: `#![forbid(unsafe_code)]` guarantees zero `unsafe` blocks.
//! - **CWE-20 (Improper Input Validation)**: Strict bounds on sample rate,
//!   channel count, bit depth, durations, and finite floating-point parameters.
//! - **CWE-190 (Integer Overflow / Wraparound)**: All sample, frame, and byte
//!   counts use checked `u64`/`usize` arithmetic instead of floating-point casts
//!   or unchecked multiplication.
//! - **CWE-400 (Uncontrolled Resource Consumption)**: Pre-flight resource caps
//!   ([`MAX_SAMPLES`], [`MAX_CHANNELS`], [`MAX_PCM_BYTES`],
//!   [`MAX_SAMPLE_RATE_HZ`]) reject oversized requests before allocation, and
//!   error messages are O(1) bounded to [`MAX_ERROR_MESSAGE_BYTES`].

#![forbid(unsafe_code)]

pub mod capabilities;

use std::error::Error;
use std::fmt;

/// Crate version, kept explicit for examples and integration tests.
pub const VERSION: &str = "0.1.0";

/// V1 blocking duration limit in whole seconds (`10` minutes = `600` seconds).
pub const MAX_BLOCKING_DURATION_SECONDS_U64: u64 = 10 * 60;

/// V1 keeps buffers small enough that accidental calls do not block for hours.
pub const MAX_BLOCKING_DURATION_SECONDS: f64 = 10.0 * 60.0;

/// Minimum supported sample rate in Hz (any positive integer rate in V1).
pub const MIN_SAMPLE_RATE_HZ: u32 = 1;

/// Practical ceiling for high-resolution audio interfaces.
pub const MAX_SAMPLE_RATE_HZ: u32 = 384_000;

/// Only signed 16-bit PCM is supported in the first device sink slice.
pub const SUPPORTED_BIT_DEPTH: u16 = 16;

/// Only mono PCM is supported until the pipeline grows interleaved channels.
pub const SUPPORTED_CHANNEL_COUNT: u16 = 1;

/// Pre-flight channel cap (aligned with V1 mono-only support).
pub const MAX_CHANNELS: u16 = SUPPORTED_CHANNEL_COUNT;

/// Global pre-flight sample cap (`384_000 Hz * 600 s * 1 ch = 230_400_000`).
pub const MAX_SAMPLES: usize =
    (MAX_SAMPLE_RATE_HZ as usize) * (MAX_BLOCKING_DURATION_SECONDS_U64 as usize);

/// Global pre-flight PCM byte cap (`MAX_SAMPLES * 2` bytes = `460_800_000`).
pub const MAX_PCM_BYTES: usize = MAX_SAMPLES * ((SUPPORTED_BIT_DEPTH / 8) as usize);

/// Maximum UTF-8 byte length stored in any [`AudioSinkError`] message (O(1) bound).
pub const MAX_ERROR_MESSAGE_BYTES: usize = 256;

/// Truncate an error message to at most [`MAX_ERROR_MESSAGE_BYTES`] on a UTF-8 boundary.
fn bound_error_message(message: impl Into<String>) -> String {
    let raw = message.into();
    if raw.len() <= MAX_ERROR_MESSAGE_BYTES {
        return raw;
    }
    let ellipsis = "...";
    let max_prefix = MAX_ERROR_MESSAGE_BYTES.saturating_sub(ellipsis.len());
    let mut end = max_prefix;
    while end > 0 && !raw.is_char_boundary(end) {
        end -= 1;
    }
    let mut bounded = String::with_capacity(end + ellipsis.len());
    bounded.push_str(&raw[..end]);
    bounded.push_str(ellipsis);
    bounded
}

/// Metadata that tells a device sink how to interpret integer samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PcmFormat {
    pub sample_rate_hz: u32,
    pub channel_count: u16,
    pub bit_depth: u16,
}

impl PcmFormat {
    /// Construct and validate a PCM format.
    pub fn new(
        sample_rate_hz: u32,
        channel_count: u16,
        bit_depth: u16,
    ) -> Result<Self, AudioSinkError> {
        let format = Self {
            sample_rate_hz,
            channel_count,
            bit_depth,
        };
        format.validate()?;
        Ok(format)
    }

    /// Check the V1 sink constraints.
    pub fn validate(self) -> Result<(), AudioSinkError> {
        if self.sample_rate_hz < MIN_SAMPLE_RATE_HZ {
            return Err(AudioSinkError::invalid_format(
                "sample_rate_hz must be greater than zero",
            ));
        }
        if self.sample_rate_hz > MAX_SAMPLE_RATE_HZ {
            return Err(AudioSinkError::invalid_format(format!(
                "sample_rate_hz must be <= {MAX_SAMPLE_RATE_HZ}, got {}",
                self.sample_rate_hz
            )));
        }
        if self.channel_count == 0
            || self.channel_count > MAX_CHANNELS
            || self.channel_count != SUPPORTED_CHANNEL_COUNT
        {
            return Err(AudioSinkError::invalid_format(format!(
                "only mono PCM is supported in V1, got {} channels",
                self.channel_count
            )));
        }
        if self.bit_depth != SUPPORTED_BIT_DEPTH {
            return Err(AudioSinkError::invalid_format(format!(
                "only signed 16-bit PCM is supported in V1, got {} bits",
                self.bit_depth
            )));
        }
        Ok(())
    }

    /// Number of bytes occupied by one PCM sample in this format.
    pub const fn sample_width_bytes(self) -> usize {
        (self.bit_depth / 8) as usize
    }

    /// Checked number of bytes occupied by one PCM frame (`sample_width * channel_count`).
    pub fn frame_width_bytes(self) -> Result<usize, AudioSinkError> {
        self.validate()?;
        self.sample_width_bytes()
            .checked_mul(self.channel_count as usize)
            .ok_or_else(|| AudioSinkError::invalid_format("frame byte width overflowed"))
    }

    /// Maximum allowed sample count for this format within the V1 blocking limit.
    ///
    /// Computed using checked `u64` multiplication and clamped to [`MAX_SAMPLES`].
    pub fn max_samples(self) -> Result<usize, AudioSinkError> {
        self.validate()?;
        let frames_u64 = (self.sample_rate_hz as u64)
            .checked_mul(MAX_BLOCKING_DURATION_SECONDS_U64)
            .ok_or_else(|| AudioSinkError::invalid_samples("max frame count overflowed"))?;
        let samples_u64 = frames_u64
            .checked_mul(self.channel_count as u64)
            .ok_or_else(|| AudioSinkError::invalid_samples("max sample count overflowed"))?;
        let samples_usize = usize::try_from(samples_u64).unwrap_or(MAX_SAMPLES);
        Ok(samples_usize.min(MAX_SAMPLES))
    }
}

/// Validate raw PCM format parameters and return a [`PcmFormat`].
pub fn validate_format(
    sample_rate_hz: u32,
    channel_count: u16,
    bit_depth: u16,
) -> Result<PcmFormat, AudioSinkError> {
    PcmFormat::new(sample_rate_hz, channel_count, bit_depth)
}

/// Validated frame count returned by [`frame_count_checked`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameCount {
    /// Number of PCM frames (`sample_count / channel_count`).
    pub frames: u64,
}

/// Compute frame count from `u64` `sample_count` and `channel_count` with checked math.
///
/// Mirrors [`checked_frame_count`] over a `u64` boundary so oversized counts
/// fail with [`AudioSinkError::InvalidSamples`] without platform truncation.
pub fn frame_count_checked(
    sample_count: u64,
    channel_count: u16,
) -> Result<FrameCount, AudioSinkError> {
    if channel_count == 0
        || channel_count > MAX_CHANNELS
        || channel_count != SUPPORTED_CHANNEL_COUNT
    {
        return Err(AudioSinkError::invalid_format(format!(
            "only mono PCM is supported in V1, got {channel_count} channels"
        )));
    }
    let sample_count_usize = usize::try_from(sample_count).map_err(|_| {
        AudioSinkError::invalid_samples(format!(
            "sample_count {sample_count} exceeds MAX_SAMPLES={MAX_SAMPLES}"
        ))
    })?;
    let frames = checked_frame_count(sample_count_usize, channel_count)?;
    Ok(FrameCount {
        frames: frames as u64,
    })
}

/// Compute the byte size of `sample_count` (`u64`) samples under `format` with pre-flight caps.
///
/// Validates `format` first, then enforces `format.max_samples()` and [`MAX_PCM_BYTES`].
pub fn checked_pcm_byte_len_u64(
    sample_count: u64,
    format: PcmFormat,
) -> Result<u64, AudioSinkError> {
    let _ = format.max_samples()?;
    let sample_count_usize = usize::try_from(sample_count).map_err(|_| {
        AudioSinkError::invalid_samples(format!(
            "blocking playback is limited to {MAX_BLOCKING_DURATION_SECONDS} seconds"
        ))
    })?;
    let bytes = checked_pcm_byte_len(sample_count_usize, format)?;
    Ok(bytes as u64)
}

/// Compute frame count from `sample_count` and `channel_count` with checked math.
///
/// Rejects unsupported channel counts, counts above [`MAX_SAMPLES`], and
/// non-frame-aligned sample counts (CWE-20 / CWE-190 / CWE-400).
pub fn checked_frame_count(
    sample_count: usize,
    channel_count: u16,
) -> Result<usize, AudioSinkError> {
    if channel_count == 0
        || channel_count > MAX_CHANNELS
        || channel_count != SUPPORTED_CHANNEL_COUNT
    {
        return Err(AudioSinkError::invalid_format(format!(
            "only mono PCM is supported in V1, got {channel_count} channels"
        )));
    }
    if sample_count > MAX_SAMPLES {
        return Err(AudioSinkError::invalid_samples(format!(
            "sample_count {sample_count} exceeds MAX_SAMPLES={MAX_SAMPLES}"
        )));
    }
    let ch = channel_count as usize;
    if !sample_count.is_multiple_of(ch) {
        return Err(AudioSinkError::invalid_samples(
            "sample_count must be an exact multiple of channel_count",
        ));
    }
    sample_count
        .checked_div(ch)
        .ok_or_else(|| AudioSinkError::invalid_format("channel_count must be > 0"))
}

/// Compute the byte size of `sample_count` samples under `format` with pre-flight caps.
pub fn checked_pcm_byte_len(
    sample_count: usize,
    format: PcmFormat,
) -> Result<usize, AudioSinkError> {
    let max_samples = format.max_samples()?;
    if sample_count > max_samples {
        return Err(AudioSinkError::invalid_samples(format!(
            "blocking playback is limited to {MAX_BLOCKING_DURATION_SECONDS} seconds"
        )));
    }
    let byte_len = sample_count
        .checked_mul(format.sample_width_bytes())
        .ok_or_else(|| AudioSinkError::invalid_samples("PCM byte count overflowed"))?;
    if byte_len > MAX_PCM_BYTES {
        return Err(AudioSinkError::invalid_samples(format!(
            "PCM byte length {byte_len} exceeds MAX_PCM_BYTES={MAX_PCM_BYTES}"
        )));
    }
    Ok(byte_len)
}

/// Validate a floating-point sample rate (CWE-20): must be a finite positive integer in range.
pub fn validate_sample_rate_f64(sample_rate_hz: f64) -> Result<u32, AudioSinkError> {
    if !sample_rate_hz.is_finite() {
        return Err(AudioSinkError::invalid_format(
            "sample_rate_hz must be finite",
        ));
    }
    if sample_rate_hz < (MIN_SAMPLE_RATE_HZ as f64) {
        return Err(AudioSinkError::invalid_format(
            "sample_rate_hz must be greater than zero",
        ));
    }
    if sample_rate_hz > (MAX_SAMPLE_RATE_HZ as f64) {
        return Err(AudioSinkError::invalid_format(format!(
            "sample_rate_hz must be <= {MAX_SAMPLE_RATE_HZ}"
        )));
    }
    if sample_rate_hz.fract() != 0.0 {
        return Err(AudioSinkError::invalid_format(
            "sample_rate_hz must be an integer-valued rate",
        ));
    }
    Ok(sample_rate_hz as u32)
}

/// Validate a blocking playback duration in seconds (CWE-20 / CWE-400).
pub fn validate_duration_seconds(duration_seconds: f64) -> Result<f64, AudioSinkError> {
    if !duration_seconds.is_finite() {
        return Err(AudioSinkError::invalid_format(
            "duration_seconds must be finite",
        ));
    }
    if duration_seconds < 0.0 {
        return Err(AudioSinkError::invalid_format(
            "duration_seconds must be non-negative",
        ));
    }
    if duration_seconds > MAX_BLOCKING_DURATION_SECONDS {
        return Err(AudioSinkError::invalid_samples(format!(
            "blocking playback is limited to {MAX_BLOCKING_DURATION_SECONDS} seconds"
        )));
    }
    Ok(duration_seconds)
}

/// Compute a checked sample count for a validated format and duration (CWE-190 / CWE-400).
pub fn checked_sample_count_for_duration(
    format: PcmFormat,
    duration_seconds: f64,
) -> Result<usize, AudioSinkError> {
    let max_samples = format.max_samples()?;
    let valid_duration = validate_duration_seconds(duration_seconds)?;
    let frames_f64 = (valid_duration * (format.sample_rate_hz as f64)).round();
    if !frames_f64.is_finite() || frames_f64 < 0.0 || frames_f64 > (max_samples as f64) {
        return Err(AudioSinkError::invalid_samples(
            "duration sample count exceeds blocking playback limit",
        ));
    }
    let frames = frames_f64 as u64;
    let total_samples = frames
        .checked_mul(format.channel_count as u64)
        .ok_or_else(|| AudioSinkError::invalid_samples("duration sample count overflowed"))?;
    let total_usize = usize::try_from(total_samples)
        .map_err(|_| AudioSinkError::invalid_samples("duration sample count exceeds usize"))?;
    if total_usize > max_samples {
        return Err(AudioSinkError::invalid_samples(format!(
            "blocking playback is limited to {MAX_BLOCKING_DURATION_SECONDS} seconds"
        )));
    }
    Ok(total_usize)
}

/// Verify that a floating-point sample slice is within [`MAX_SAMPLES`] and finite (CWE-20 / CWE-400).
pub fn validate_finite_samples(samples: &[f64]) -> Result<(), AudioSinkError> {
    if samples.len() > MAX_SAMPLES {
        return Err(AudioSinkError::invalid_samples(format!(
            "sample_count {} exceeds MAX_SAMPLES={MAX_SAMPLES}",
            samples.len()
        )));
    }
    for (index, &sample) in samples.iter().enumerate() {
        if !sample.is_finite() {
            return Err(AudioSinkError::invalid_samples(format!(
                "non-finite sample at index {index}"
            )));
        }
    }
    Ok(())
}

/// Owned PCM samples plus the metadata needed by an OS backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcmPlaybackBuffer {
    samples: Vec<i16>,
    format: PcmFormat,
}

impl PcmPlaybackBuffer {
    /// Construct and validate an owned playback buffer.
    pub fn new(samples: Vec<i16>, format: PcmFormat) -> Result<Self, AudioSinkError> {
        Self::validate_sample_count(samples.len(), format)?;
        Ok(Self { samples, format })
    }

    /// Pre-flight check that `sample_count` and `format` satisfy all V1 bounds.
    pub fn validate_sample_count(
        sample_count: usize,
        format: PcmFormat,
    ) -> Result<(), AudioSinkError> {
        let _ = checked_pcm_byte_len(sample_count, format)?;
        Ok(())
    }

    /// Borrow the signed 16-bit PCM samples.
    pub fn samples(&self) -> &[i16] {
        &self.samples
    }

    /// Return the PCM format.
    pub const fn format(&self) -> PcmFormat {
        self.format
    }

    /// Return the number of samples in the buffer.
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    /// V1 is mono, so frames and samples are the same count.
    pub fn frame_count(&self) -> usize {
        self.samples
            .len()
            .checked_div(self.format.channel_count as usize)
            .unwrap_or(0)
    }

    /// Checked total byte length of the PCM payload.
    pub fn byte_len(&self) -> Result<usize, AudioSinkError> {
        checked_pcm_byte_len(self.samples.len(), self.format)
    }

    /// True when there is nothing for a backend to play.
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Return the intended playback duration.
    pub fn duration_seconds(&self) -> f64 {
        self.frame_count() as f64 / self.format.sample_rate_hz as f64
    }
}

/// Result returned after a sink accepts or completes playback.
#[derive(Debug, Clone, PartialEq)]
pub struct PlaybackReport {
    pub frames_played: usize,
    pub sample_rate_hz: u32,
    pub channel_count: u16,
    pub duration_seconds: f64,
    pub backend_name: &'static str,
}

impl PlaybackReport {
    /// Build a report that mirrors one validated playback buffer.
    pub fn for_buffer(buffer: &PcmPlaybackBuffer, backend_name: &'static str) -> Self {
        Self {
            frames_played: buffer.frame_count(),
            sample_rate_hz: buffer.format.sample_rate_hz,
            channel_count: buffer.format.channel_count,
            duration_seconds: buffer.duration_seconds(),
            backend_name,
        }
    }
}

/// Something that can play a PCM buffer.
pub trait AudioSink {
    fn play_blocking(&self, buffer: &PcmPlaybackBuffer) -> Result<PlaybackReport, AudioSinkError>;
}

/// Test and teaching sink that accepts buffers without touching a device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoopAudioSink {
    backend_name: &'static str,
}

impl NoopAudioSink {
    /// Construct a no-op sink with a visible backend name.
    pub const fn new(backend_name: &'static str) -> Self {
        Self { backend_name }
    }
}

impl AudioSink for NoopAudioSink {
    fn play_blocking(&self, buffer: &PcmPlaybackBuffer) -> Result<PlaybackReport, AudioSinkError> {
        PcmPlaybackBuffer::validate_sample_count(buffer.sample_count(), buffer.format())?;
        Ok(PlaybackReport::for_buffer(buffer, self.backend_name))
    }
}

/// Shared error type for validation and backend failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioSinkError {
    InvalidFormat(String),
    InvalidSamples(String),
    UnsupportedPlatform(String),
    BackendUnavailable(String),
    BackendFailure(String),
}

impl AudioSinkError {
    pub fn invalid_format(message: impl Into<String>) -> Self {
        Self::InvalidFormat(bound_error_message(message))
    }

    pub fn invalid_samples(message: impl Into<String>) -> Self {
        Self::InvalidSamples(bound_error_message(message))
    }

    pub fn unsupported_platform(message: impl Into<String>) -> Self {
        Self::UnsupportedPlatform(bound_error_message(message))
    }

    pub fn backend_unavailable(message: impl Into<String>) -> Self {
        Self::BackendUnavailable(bound_error_message(message))
    }

    pub fn backend_failure(message: impl Into<String>) -> Self {
        Self::BackendFailure(bound_error_message(message))
    }
}

impl fmt::Display for AudioSinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat(message) => write!(f, "invalid PCM format: {message}"),
            Self::InvalidSamples(message) => write!(f, "invalid PCM samples: {message}"),
            Self::UnsupportedPlatform(message) => write!(f, "unsupported platform: {message}"),
            Self::BackendUnavailable(message) => write!(f, "audio backend unavailable: {message}"),
            Self::BackendFailure(message) => write!(f, "audio backend failure: {message}"),
        }
    }
}

impl Error for AudioSinkError {}
