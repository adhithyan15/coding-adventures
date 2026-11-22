//! Audio format definitions, constants, and error representations.

use std::error::Error;
use std::fmt;

/// Default audio sampling rate in Hertz (CD quality: 44.1 kHz).
pub const DEFAULT_SAMPLE_RATE_HZ: f64 = 44_100.0;

/// Default bit depth for integer PCM (16 bits per sample).
pub const DEFAULT_BIT_DEPTH: u16 = 16;

/// Default number of channels (mono = 1).
pub const DEFAULT_CHANNEL_COUNT: u16 = 1;

/// Default full-scale DAC peak voltage (1.0 Volt).
pub const DEFAULT_FULL_SCALE_VOLTAGE: f64 = 1.0;

/// Minimum signed 16-bit PCM integer (-32,768).
pub const PCM16_MIN: i16 = -32_768;

/// Maximum signed 16-bit PCM integer (+32,767).
pub const PCM16_MAX: i16 = 32_767;

/// Maximum allowable sampling rate in Hertz (CWE-400 guard: 192 kHz).
pub const MAX_SAMPLE_RATE_HZ: f64 = 192_000.0;

/// Maximum duration in seconds for in-memory operations (CWE-400 guard: 60 s).
pub const MAX_DURATION_SECONDS: f64 = 60.0;

/// Maximum allowable sample count in a single buffer (CWE-400 guard: 2^24 = 16,777,216).
pub const MAX_SAMPLES: usize = 16_777_216;

/// Maximum allowable channels (CWE-400 guard: 8 channels).
pub const MAX_CHANNELS: u16 = 8;

/// Maximum allowable WAV file size in bytes (CWE-400 guard: 64 MiB).
pub const MAX_WAV_BYTES: usize = 64 * 1024 * 1024;

/// Canonical RIFF/WAVE header size in bytes (12-byte RIFF + 24-byte `fmt ` + 8-byte `data` header).
pub const MIN_WAV_HEADER_SIZE: usize = 44;

/// Maximum allowable PCM `data` chunk payload bytes in a canonical WAV file (`MAX_WAV_BYTES - 44`).
pub const MAX_WAV_DATA_BYTES: usize = MAX_WAV_BYTES - MIN_WAV_HEADER_SIZE;

/// Epsilon tolerance for detecting integer-valued sample rates.
pub const INTEGER_TOLERANCE: f64 = 1e-9;

/// Errors that can occur during PCM audio encoding, decoding, or WAV serialization.
#[derive(Debug, Clone, PartialEq)]
pub enum PcmError {
    /// A parameter failed general validation rules.
    InvalidParameter {
        name: &'static str,
        reason: String,
    },
    /// A sample value was non-finite (NaN or Inf) (CWE-20).
    NonFiniteSample { value: f64 },
    /// A numeric parameter was non-finite (CWE-20).
    NonFiniteNumber {
        name: &'static str,
        value: f64,
    },
    /// A parameter expected to be non-negative was negative.
    NegativeNumber {
        name: &'static str,
        value: f64,
    },
    /// Sample rate was not an integer, which is required for standard WAV containers.
    IntegerSampleRateRequired { value: f64 },
    /// Requested sample rate exceeds [`MAX_SAMPLE_RATE_HZ`] (CWE-400).
    SampleRateTooHigh { rate: f64, max: f64 },
    /// Requested duration exceeds [`MAX_DURATION_SECONDS`] (CWE-400).
    DurationTooLong { duration: f64, max: f64 },
    /// Requested sample count exceeds [`MAX_SAMPLES`] (CWE-400).
    SampleCountTooLarge { count: usize, max: usize },
    /// WAV file size exceeds [`MAX_WAV_BYTES`] (CWE-400).
    OversizedWav { size: usize, max: usize },
    /// Malformed or truncated WAV file (CWE-20).
    MalformedWav(String),
    /// Audio format code in WAV is unsupported (e.g. not PCM=1 or IEEE float=3).
    UnsupportedFormat { format_code: u16 },
    /// Bit depth is unsupported (only 16-bit or 32-bit float supported).
    UnsupportedBitDepth { bit_depth: u16 },
    /// Channel count is unsupported or zero.
    UnsupportedChannelCount { channels: u16 },
    /// Buffer index was out of bounds.
    IndexOutOfBounds { index: usize, count: usize },
    /// Arithmetic overflow occurred during size or timing calculations (CWE-190).
    IntegerOverflow(&'static str),
}

impl fmt::Display for PcmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidParameter { name, reason } => {
                write!(f, "invalid parameter '{name}': {reason}")
            }
            Self::NonFiniteSample { value } => {
                write!(f, "audio sample must be finite, got {value}")
            }
            Self::NonFiniteNumber { name, value } => {
                write!(f, "parameter '{name}' must be finite, got {value}")
            }
            Self::NegativeNumber { name, value } => {
                write!(f, "parameter '{name}' must be non-negative, got {value}")
            }
            Self::IntegerSampleRateRequired { value } => {
                write!(
                    f,
                    "sample_rate_hz must be an integer-valued rate for WAV output, got {value}"
                )
            }
            Self::SampleRateTooHigh { rate, max } => {
                write!(f, "sample rate {rate} Hz exceeds maximum {max} Hz")
            }
            Self::DurationTooLong { duration, max } => {
                write!(f, "duration {duration} s exceeds maximum {max} s")
            }
            Self::SampleCountTooLarge { count, max } => {
                write!(f, "sample count {count} exceeds maximum allowable {max}")
            }
            Self::OversizedWav { size, max } => {
                write!(f, "WAV byte size {size} exceeds maximum allowable {max}")
            }
            Self::MalformedWav(msg) => write!(f, "malformed WAV data: {msg}"),
            Self::UnsupportedFormat { format_code } => {
                write!(f, "unsupported WAV audio format tag 0x{format_code:04X}")
            }
            Self::UnsupportedBitDepth { bit_depth } => {
                write!(f, "unsupported bit depth: {bit_depth}")
            }
            Self::UnsupportedChannelCount { channels } => {
                write!(f, "unsupported channel count: {channels}")
            }
            Self::IndexOutOfBounds { index, count } => {
                write!(f, "index {index} out of bounds for buffer of size {count}")
            }
            Self::IntegerOverflow(op) => {
                write!(f, "integer overflow during {op} calculation")
            }
        }
    }
}

impl Error for PcmError {}

/// Metadata specifying sample rate, channels, bit depth, and voltage scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PcmFormat {
    sample_rate_hz: f64,
    channel_count: u16,
    bit_depth: u16,
    full_scale_voltage: f64,
}

impl Default for PcmFormat {
    fn default() -> Self {
        Self {
            sample_rate_hz: DEFAULT_SAMPLE_RATE_HZ,
            channel_count: DEFAULT_CHANNEL_COUNT,
            bit_depth: DEFAULT_BIT_DEPTH,
            full_scale_voltage: DEFAULT_FULL_SCALE_VOLTAGE,
        }
    }
}

impl PcmFormat {
    /// Construct a new `PcmFormat` with explicit validation.
    pub fn new(
        sample_rate_hz: f64,
        channel_count: u16,
        bit_depth: u16,
        full_scale_voltage: f64,
    ) -> Result<Self, PcmError> {
        if !sample_rate_hz.is_finite() {
            return Err(PcmError::NonFiniteNumber {
                name: "sample_rate_hz",
                value: sample_rate_hz,
            });
        }
        if sample_rate_hz <= 0.0 {
            return Err(PcmError::InvalidParameter {
                name: "sample_rate_hz",
                reason: format!("must be > 0.0, got {sample_rate_hz}"),
            });
        }
        if sample_rate_hz > MAX_SAMPLE_RATE_HZ {
            return Err(PcmError::SampleRateTooHigh {
                rate: sample_rate_hz,
                max: MAX_SAMPLE_RATE_HZ,
            });
        }

        if channel_count == 0 || channel_count > MAX_CHANNELS {
            return Err(PcmError::UnsupportedChannelCount {
                channels: channel_count,
            });
        }

        if bit_depth != 16 && bit_depth != 32 {
            return Err(PcmError::UnsupportedBitDepth { bit_depth });
        }

        if !full_scale_voltage.is_finite() {
            return Err(PcmError::NonFiniteNumber {
                name: "full_scale_voltage",
                value: full_scale_voltage,
            });
        }
        if full_scale_voltage <= 0.0 {
            return Err(PcmError::InvalidParameter {
                name: "full_scale_voltage",
                reason: format!("must be > 0.0, got {full_scale_voltage}"),
            });
        }

        Ok(Self {
            sample_rate_hz,
            channel_count,
            bit_depth,
            full_scale_voltage,
        })
    }

    /// Construct a mono 16-bit format with the given sample rate.
    pub fn mono_16bit(sample_rate_hz: f64) -> Result<Self, PcmError> {
        Self::new(sample_rate_hz, 1, 16, DEFAULT_FULL_SCALE_VOLTAGE)
    }

    pub fn sample_rate_hz(&self) -> f64 {
        self.sample_rate_hz
    }

    pub fn channel_count(&self) -> u16 {
        self.channel_count
    }

    pub fn bit_depth(&self) -> u16 {
        self.bit_depth
    }

    pub fn full_scale_voltage(&self) -> f64 {
        self.full_scale_voltage
    }

    pub fn minimum_integer(&self) -> i32 {
        PCM16_MIN as i32
    }

    pub fn maximum_integer(&self) -> i32 {
        PCM16_MAX as i32
    }

    pub fn sample_width_bytes(&self) -> usize {
        (self.bit_depth / 8) as usize
    }

    /// Return an integer sample rate required for WAV file headers.
    pub fn integer_sample_rate(&self) -> Result<u32, PcmError> {
        let rounded = self.sample_rate_hz.round();
        if rounded < 1.0 || (self.sample_rate_hz - rounded).abs() > INTEGER_TOLERANCE {
            return Err(PcmError::IntegerSampleRateRequired {
                value: self.sample_rate_hz,
            });
        }
        Ok(rounded as u32)
    }
}
