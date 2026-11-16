//! Error types for the music-machine-core crate.

use std::error::Error;
use std::fmt;

/// Errors produced during score parsing, validation, rendering, or I/O.
#[derive(Debug, Clone, PartialEq)]
pub enum MusicMachineError {
    /// An input parameter failed domain validation (CWE-20).
    InvalidArgument(String),
    /// A configured resource budget was exceeded (CWE-400 / CWE-190).
    BudgetExceeded(String),
    /// A score text or token failed parsing.
    ParseError(String),
    /// A rendering pipeline stage failed.
    RenderError(String),
    /// An I/O error occurred during file reading or writing.
    Io(String),
}

impl fmt::Display for MusicMachineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArgument(msg) => write!(f, "invalid argument: {msg}"),
            Self::BudgetExceeded(msg) => write!(f, "budget exceeded: {msg}"),
            Self::ParseError(msg) => write!(f, "parse error: {msg}"),
            Self::RenderError(msg) => write!(f, "render error: {msg}"),
            Self::Io(msg) => write!(f, "I/O error: {msg}"),
        }
    }
}

impl Error for MusicMachineError {}

impl From<pcm_audio::PcmError> for MusicMachineError {
    fn from(err: pcm_audio::PcmError) -> Self {
        Self::RenderError(err.to_string())
    }
}

impl From<musical_instruments::InstrumentError> for MusicMachineError {
    fn from(err: musical_instruments::InstrumentError) -> Self {
        Self::RenderError(err.to_string())
    }
}

impl From<percussion_kit::PercussionError> for MusicMachineError {
    fn from(err: percussion_kit::PercussionError) -> Self {
        Self::RenderError(err.to_string())
    }
}

impl From<oscillator::OscillatorError> for MusicMachineError {
    fn from(err: oscillator::OscillatorError) -> Self {
        Self::RenderError(err.to_string())
    }
}

impl From<wav_sink::WavSinkError> for MusicMachineError {
    fn from(err: wav_sink::WavSinkError) -> Self {
        Self::Io(err.to_string())
    }
}
