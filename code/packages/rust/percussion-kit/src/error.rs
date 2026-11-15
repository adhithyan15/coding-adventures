//! Structured error types, validation constants, and security bounds for percussion-kit.
//!
//! Enforces CWE-400 (Resource Exhaustion), CWE-190 (Integer Overflow), and
//! CWE-20 (Improper Input Validation) before any buffer or memory allocation.

use std::error::Error;
use std::fmt;

/// Maximum allowable resonant modes per percussion voice (CWE-400 guard).
pub const MAX_PERCUSSION_MODES: usize = 32;

/// Maximum allowable duration for a single hit in seconds (CWE-400 guard).
pub const MAX_DURATION_SECONDS: f64 = 60.0;

/// Alias for [`MAX_DURATION_SECONDS`] matching Python reference nomenclature.
pub const MAX_HIT_DURATION_SECONDS: f64 = MAX_DURATION_SECONDS;

/// Maximum allowable audio sampling rate in Hertz (CWE-400 guard).
pub const MAX_SAMPLE_RATE_HZ: f64 = 192_000.0;

/// Standard default sample rate in Hertz (CD quality).
pub const DEFAULT_SAMPLE_RATE_HZ: f64 = 44_100.0;

/// Maximum allowable sample count in a single rendered buffer (2^24 samples, ~128 MiB).
pub const MAX_SAMPLE_COUNT: usize = 16_777_216;

/// Default sample count ceiling for render operations.
pub const DEFAULT_MAX_SAMPLE_COUNT: usize = 10_000_000;

/// Default mode count ceiling for render operations.
pub const DEFAULT_MAX_MODE_COUNT: usize = MAX_PERCUSSION_MODES;

/// Maximum allowable events in a single percussion sequence (CWE-400 guard).
pub const MAX_HITS: usize = 65_536;

/// Alias for [`MAX_HITS`] matching sequence rendering nomenclature.
pub const MAX_SEQUENCE_EVENTS: usize = MAX_HITS;

/// Default linear release ramp duration for hi-hat choking in seconds (MUS07 §7).
pub const CHOKE_RELEASE_RAMP_SECONDS: f64 = 0.015;

/// Epsilon tolerance for floating-point index rounding.
pub const INTEGER_TOLERANCE: f64 = 1e-9;

/// Structured error type for unpitched percussion synthesis and sequencing.
#[derive(Debug, Clone, PartialEq)]
pub enum PercussionError {
    /// A parameter failed general validation rules.
    InvalidParameter {
        name: &'static str,
        reason: String,
    },
    /// A float value was not finite (NaN, +Inf, -Inf) (CWE-20).
    NonFiniteNumber {
        name: &'static str,
        value: f64,
    },
    /// A parameter expected to be non-negative was negative.
    NegativeNumber {
        name: &'static str,
        value: f64,
    },
    /// A parameter was outside the closed unit interval [0.0, 1.0].
    UnitFloatOutOfRange {
        name: &'static str,
        value: f64,
    },
    /// Requested sample rate exceeds [`MAX_SAMPLE_RATE_HZ`] (CWE-400).
    SampleRateTooHigh {
        rate: f64,
        max: f64,
    },
    /// Requested duration exceeds [`MAX_DURATION_SECONDS`] (CWE-400).
    DurationTooLong {
        duration: f64,
        max: f64,
    },
    /// Requested sample count exceeds allowable limits (CWE-400).
    SampleCountTooLarge {
        count: usize,
        max: usize,
    },
    /// Requested mode count exceeds [`MAX_PERCUSSION_MODES`] (CWE-400).
    ModeCountTooLarge {
        count: usize,
        max: usize,
    },
    /// Number of sequence events exceeds [`MAX_HITS`] (CWE-400).
    SequenceEventsTooLarge {
        count: usize,
        max: usize,
    },
    /// A hit identifier could not be resolved in the drum kit.
    UnknownVoice {
        hit_id: String,
        kit_id: String,
    },
    /// A drum kit identifier could not be found.
    UnknownKit {
        kit_id: String,
    },
    /// An identifier string was empty.
    EmptyIdentifier {
        entity: &'static str,
    },
    /// Error propagated from oscillator crate.
    Oscillator(oscillator::OscillatorError),
    /// Error propagated from pcm-audio crate.
    Pcm(pcm_audio::PcmError),
}

impl fmt::Display for PercussionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidParameter { name, reason } => {
                write!(f, "invalid parameter '{name}': {reason}")
            }
            Self::NonFiniteNumber { name, value } => {
                write!(f, "parameter '{name}' must be finite, got {value}")
            }
            Self::NegativeNumber { name, value } => {
                write!(f, "parameter '{name}' must be non-negative, got {value}")
            }
            Self::UnitFloatOutOfRange { name, value } => {
                write!(f, "parameter '{name}' must be in [0.0, 1.0], got {value}")
            }
            Self::SampleRateTooHigh { rate, max } => {
                write!(f, "sample rate {rate} Hz exceeds maximum {max} Hz")
            }
            Self::DurationTooLong { duration, max } => {
                write!(f, "duration {duration} s exceeds maximum {max} s")
            }
            Self::SampleCountTooLarge { count, max } => {
                write!(f, "sample count {count} exceeds maximum {max}")
            }
            Self::ModeCountTooLarge { count, max } => {
                write!(f, "mode count {count} exceeds maximum {max}")
            }
            Self::SequenceEventsTooLarge { count, max } => {
                write!(f, "sequence event count {count} exceeds maximum {max}")
            }
            Self::UnknownVoice { hit_id, kit_id } => {
                write!(f, "unknown hit '{hit_id}' in kit '{kit_id}'")
            }
            Self::UnknownKit { kit_id } => {
                write!(f, "unknown drum kit '{kit_id}'")
            }
            Self::EmptyIdentifier { entity } => {
                write!(f, "{entity} identifier must be non-empty")
            }
            Self::Oscillator(err) => write!(f, "oscillator error: {err}"),
            Self::Pcm(err) => write!(f, "pcm audio error: {err}"),
        }
    }
}

impl Error for PercussionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Oscillator(err) => Some(err),
            Self::Pcm(err) => Some(err),
            _ => None,
        }
    }
}

impl From<oscillator::OscillatorError> for PercussionError {
    fn from(err: oscillator::OscillatorError) -> Self {
        Self::Oscillator(err)
    }
}

impl From<pcm_audio::PcmError> for PercussionError {
    fn from(err: pcm_audio::PcmError) -> Self {
        Self::Pcm(err)
    }
}

// ============================================================================
// Defensive Numeric Validation Helpers (CWE-20 / CWE-190 / CWE-400)
// ============================================================================

/// Validate that a floating-point value is finite (not NaN, +Inf, or -Inf).
#[inline]
pub fn validate_finite(name: &'static str, value: f64) -> Result<f64, PercussionError> {
    if !value.is_finite() {
        return Err(PercussionError::NonFiniteNumber { name, value });
    }
    Ok(value)
}

/// Validate that a floating-point value is finite and >= 0.0.
#[inline]
pub fn validate_non_negative(name: &'static str, value: f64) -> Result<f64, PercussionError> {
    let v = validate_finite(name, value)?;
    if v < 0.0 {
        return Err(PercussionError::NegativeNumber { name, value: v });
    }
    Ok(v)
}

/// Validate that a floating-point value is finite and > 0.0.
#[inline]
pub fn validate_positive(name: &'static str, value: f64) -> Result<f64, PercussionError> {
    let v = validate_finite(name, value)?;
    if v <= 0.0 {
        return Err(PercussionError::InvalidParameter {
            name,
            reason: format!("must be > 0.0, got {v}"),
        });
    }
    Ok(v)
}

/// Validate that a floating-point value is finite and within [0.0, 1.0].
#[inline]
pub fn validate_unit_float(name: &'static str, value: f64) -> Result<f64, PercussionError> {
    let v = validate_finite(name, value)?;
    if !(0.0..=1.0).contains(&v) {
        return Err(PercussionError::UnitFloatOutOfRange { name, value: v });
    }
    Ok(v)
}

/// Validate that a sample rate is finite, > 0.0, and <= [`MAX_SAMPLE_RATE_HZ`].
#[inline]
pub fn validate_sample_rate(name: &'static str, value: f64) -> Result<f64, PercussionError> {
    let v = validate_positive(name, value)?;
    if v > MAX_SAMPLE_RATE_HZ {
        return Err(PercussionError::SampleRateTooHigh {
            rate: v,
            max: MAX_SAMPLE_RATE_HZ,
        });
    }
    Ok(v)
}

/// Validate that a duration is non-negative and <= [`MAX_DURATION_SECONDS`].
#[inline]
pub fn validate_duration(name: &'static str, value: f64) -> Result<f64, PercussionError> {
    let v = validate_non_negative(name, value)?;
    if v > MAX_DURATION_SECONDS {
        return Err(PercussionError::DurationTooLong {
            duration: v,
            max: MAX_DURATION_SECONDS,
        });
    }
    Ok(v)
}

/// Safely multiply a time in seconds by a sample rate in Hertz and round to an integer sample index (CWE-190).
#[inline]
pub fn safe_round_sample(time_seconds: f64, sample_rate_hz: f64) -> Result<usize, PercussionError> {
    let t = validate_non_negative("time_seconds", time_seconds)?;
    let rate = validate_sample_rate("sample_rate_hz", sample_rate_hz)?;
    let prod = t * rate;
    if !prod.is_finite() {
        return Err(PercussionError::NonFiniteNumber {
            name: "time * rate product",
            value: prod,
        });
    }
    if prod > (MAX_SAMPLE_COUNT as f64) * 2.0 {
        return Err(PercussionError::SampleCountTooLarge {
            count: usize::MAX,
            max: MAX_SAMPLE_COUNT,
        });
    }
    let count_f64 = prod.round();
    if count_f64 < 0.0 {
        return Err(PercussionError::NegativeNumber {
            name: "sample index",
            value: count_f64,
        });
    }
    Ok(count_f64 as usize)
}
