//! Structured error types for musical-instruments.

use std::error::Error;
use std::fmt;

/// Structured error type for validation and synthesis failures in musical-instruments.
#[derive(Debug, Clone, PartialEq)]
pub enum InstrumentError {
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
    /// A parameter expected to be positive was <= 0.0.
    NonPositiveNumber {
        name: &'static str,
        value: f64,
    },
    /// A parameter was outside its valid range.
    ValueOutOfRange {
        name: &'static str,
        min: f64,
        max: f64,
        actual: f64,
    },
    /// Requested resource budget was exceeded (CWE-400).
    BudgetExceeded {
        reason: String,
    },
    /// An unknown instrument preset was requested.
    UnknownInstrument(String),
    /// An invalid General MIDI program number was requested (must be in 1..=128).
    UnknownProgram(u8),
    /// Note text parsing error.
    NoteParseError(String),
    /// Error originating from the oscillator crate.
    Oscillator(String),
    /// Error originating from the pcm-audio crate.
    Pcm(String),
}

impl fmt::Display for InstrumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidParameter { name, reason } => {
                write!(f, "invalid parameter {name}: {reason}")
            }
            Self::NonFiniteNumber { name, value } => {
                write!(f, "{name} must be finite, got {value}")
            }
            Self::NegativeNumber { name, value } => {
                write!(f, "{name} must be >= 0.0, got {value}")
            }
            Self::NonPositiveNumber { name, value } => {
                write!(f, "{name} must be > 0.0, got {value}")
            }
            Self::ValueOutOfRange {
                name,
                min,
                max,
                actual,
            } => {
                write!(f, "{name} must be in [{min}, {max}], got {actual}")
            }
            Self::BudgetExceeded { reason } => {
                write!(f, "resource budget exceeded: {reason}")
            }
            Self::UnknownInstrument(name) => {
                write!(f, "unknown instrument {name:?}")
            }
            Self::UnknownProgram(prog) => {
                write!(f, "program must be in [1, 128], got {prog}")
            }
            Self::NoteParseError(msg) => {
                write!(f, "note parse error: {msg}")
            }
            Self::Oscillator(msg) => {
                write!(f, "oscillator error: {msg}")
            }
            Self::Pcm(msg) => {
                write!(f, "pcm-audio error: {msg}")
            }
        }
    }
}

impl Error for InstrumentError {}

impl From<oscillator::OscillatorError> for InstrumentError {
    fn from(err: oscillator::OscillatorError) -> Self {
        Self::Oscillator(err.to_string())
    }
}

impl From<pcm_audio::PcmError> for InstrumentError {
    fn from(err: pcm_audio::PcmError) -> Self {
        Self::Pcm(err.to_string())
    }
}
