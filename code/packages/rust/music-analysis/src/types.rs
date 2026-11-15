//! Types, constants, errors, and pre-flight validation for music analysis (VER15).

use std::fmt;

/// Maximum number of audio samples supported in a single buffer (2^24 = 16,777,216).
pub const MAX_SAMPLES: usize = 16_777_216;

/// Maximum FFT length supported (2^20 = 1,048,576).
pub const MAX_FFT_LEN: usize = 1_048_576;

/// Maximum number of time frames in spectrogram / feature tracking (2^20 = 1,048,576).
pub const MAX_FRAMES: usize = 1_048_576;

/// Maximum number of harmonic partials (up to 64).
pub const MAX_PARTIALS: usize = 64;

/// Maximum number of detectable onsets (up to 65,536).
pub const MAX_ONSETS: usize = 65_536;

/// Maximum musical tempo in beats per minute.
pub const MAX_BPM: f64 = 1000.0;

/// Minimum musical tempo in beats per minute.
pub const MIN_BPM: f64 = 1.0;

/// Maximum sampling rate supported in Hertz (192 kHz).
pub const MAX_SAMPLE_RATE_HZ: f64 = 192_000.0;

/// Minimum sampling rate supported in Hertz (1 kHz).
pub const MIN_SAMPLE_RATE_HZ: f64 = 1000.0;

/// Standard tuning reference frequency for A4 in Hertz (440 Hz).
pub const DEFAULT_A4_HZ: f64 = 440.0;

/// Represents errors during music analysis and transcription.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnalysisError {
    /// Raised when an input parameter is invalid, non-finite, or malformed (CWE-20).
    InvalidInput(String),
    /// Raised when an input exceeds pre-flight resource allocation bounds (CWE-400 / CWE-190).
    BudgetExceeded(String),
}

impl fmt::Display for AnalysisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(msg) => write!(f, "InvalidInput: {msg}"),
            Self::BudgetExceeded(msg) => write!(f, "BudgetExceeded: {msg}"),
        }
    }
}

impl std::error::Error for AnalysisError {}

/// Result of frequency to note conversion.
#[derive(Debug, Clone, PartialEq)]
pub struct NotePitch {
    /// Canonical note name with octave (e.g. "A4", "C#5").
    pub note: String,
    /// Nearest MIDI semitone index (e.g. 69 for A4, 60 for C4).
    pub midi: i32,
    /// Cents deviation from nearest semitone in [-50.0, +50.0].
    pub cents: f64,
}

/// Validate that `val` is a finite float (CWE-20 fail-closed).
pub fn validate_finite_float(val: f64, name: &str) -> Result<f64, AnalysisError> {
    if !val.is_finite() {
        return Err(AnalysisError::InvalidInput(format!(
            "{name} must be finite; got {val}"
        )));
    }
    Ok(val)
}

/// Validate that `val` is a finite positive float > 0.0, and optionally <= `max_val`.
pub fn validate_positive_float(
    val: f64,
    name: &str,
    max_val: Option<f64>,
) -> Result<f64, AnalysisError> {
    let f_val = validate_finite_float(val, name)?;
    if f_val <= 0.0 {
        return Err(AnalysisError::InvalidInput(format!(
            "{name} must be positive (> 0.0); got {f_val}"
        )));
    }
    if let Some(max) = max_val {
        if f_val > max {
            return Err(AnalysisError::BudgetExceeded(format!(
                "{name} {f_val} exceeds maximum allowed {max}"
            )));
        }
    }
    Ok(f_val)
}

/// Validate sample buffer before processing (CWE-400, CWE-20).
pub fn validate_samples(samples: &[f64], name: &str) -> Result<(), AnalysisError> {
    let n = samples.len();
    if n == 0 {
        return Err(AnalysisError::InvalidInput(format!("{name} must be non-empty")));
    }
    if n > MAX_SAMPLES {
        return Err(AnalysisError::BudgetExceeded(format!(
            "{name} length {n} exceeds MAX_SAMPLES={MAX_SAMPLES}"
        )));
    }
    for (i, &s) in samples.iter().enumerate() {
        if !s.is_finite() {
            return Err(AnalysisError::InvalidInput(format!(
                "{name}[{i}] must be finite; got {s}"
            )));
        }
    }
    Ok(())
}

/// Validate frequency-domain magnitude or power spectrum (CWE-400, CWE-20).
pub fn validate_spectrum(spectrum: &[f64], name: &str) -> Result<(), AnalysisError> {
    let n = spectrum.len();
    if n == 0 {
        return Err(AnalysisError::InvalidInput(format!("{name} must be non-empty")));
    }
    if n > MAX_FFT_LEN {
        return Err(AnalysisError::BudgetExceeded(format!(
            "{name} length {n} exceeds MAX_FFT_LEN={MAX_FFT_LEN}"
        )));
    }
    for (i, &s) in spectrum.iter().enumerate() {
        if !s.is_finite() {
            return Err(AnalysisError::InvalidInput(format!(
                "{name}[{i}] must be finite; got {s}"
            )));
        }
    }
    Ok(())
}
