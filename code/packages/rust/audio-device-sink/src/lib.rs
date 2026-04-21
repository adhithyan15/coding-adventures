//! # audio-device-sink
//!
//! Backend-neutral PCM playback primitives.
//!
//! This crate is intentionally boring in the best possible way: it does not
//! open devices, talk to Core Audio, parse notes, or generate waves. It defines
//! the shared contract that every real audio backend must obey.

use std::error::Error;
use std::fmt;

/// Crate version, kept explicit for examples and integration tests.
pub const VERSION: &str = "0.1.0";

/// V1 keeps buffers small enough that accidental calls do not block for hours.
pub const MAX_BLOCKING_DURATION_SECONDS: f64 = 10.0 * 60.0;

/// Practical ceiling for high-resolution audio interfaces.
pub const MAX_SAMPLE_RATE_HZ: u32 = 384_000;

/// Only signed 16-bit PCM is supported in the first device sink slice.
pub const SUPPORTED_BIT_DEPTH: u16 = 16;

/// Only mono PCM is supported until the pipeline grows interleaved channels.
pub const SUPPORTED_CHANNEL_COUNT: u16 = 1;

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
        if self.sample_rate_hz == 0 {
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
        if self.channel_count != SUPPORTED_CHANNEL_COUNT {
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
        format.validate()?;
        let buffer = Self { samples, format };
        buffer.validate_size()?;
        Ok(buffer)
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
        self.sample_count() / self.format.channel_count as usize
    }

    /// True when there is nothing for a backend to play.
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Return the intended playback duration.
    pub fn duration_seconds(&self) -> f64 {
        self.frame_count() as f64 / self.format.sample_rate_hz as f64
    }

    fn validate_size(&self) -> Result<(), AudioSinkError> {
        let max_samples = self.format.sample_rate_hz as f64 * MAX_BLOCKING_DURATION_SECONDS;
        if self.sample_count() as f64 > max_samples {
            return Err(AudioSinkError::invalid_samples(format!(
                "blocking playback is limited to {MAX_BLOCKING_DURATION_SECONDS} seconds"
            )));
        }
        Ok(())
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
        Self::InvalidFormat(message.into())
    }

    pub fn invalid_samples(message: impl Into<String>) -> Self {
        Self::InvalidSamples(message.into())
    }

    pub fn unsupported_platform(message: impl Into<String>) -> Self {
        Self::UnsupportedPlatform(message.into())
    }

    pub fn backend_unavailable(message: impl Into<String>) -> Self {
        Self::BackendUnavailable(message.into())
    }

    pub fn backend_failure(message: impl Into<String>) -> Self {
        Self::BackendFailure(message.into())
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
