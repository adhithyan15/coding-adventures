//! Decaying resonant mode model for percussion voices (MUS07 §4.2).

use crate::error::{validate_duration, validate_finite, validate_non_negative, validate_positive, PercussionError};

/// One decaying resonant mode inside a percussion voice (MUS07 §4.2).
///
/// Represents a physical resonance of a drum shell, membrane, zil, or wooden body.
/// Unlike pitched harmonics, percussion modes need not be integer multiples of
/// a fundamental frequency.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PercussionMode {
    pub frequency_hz: f64,
    pub amplitude: f64,
    pub decay_seconds: f64,
    pub phase_cycles: f64,
}

impl PercussionMode {
    /// Construct a new resonant mode with zero initial phase.
    pub fn new(
        frequency_hz: f64,
        amplitude: f64,
        decay_seconds: f64,
    ) -> Result<Self, PercussionError> {
        Self::with_phase(frequency_hz, amplitude, decay_seconds, 0.0)
    }

    /// Construct a new resonant mode with custom initial phase offset.
    pub fn with_phase(
        frequency_hz: f64,
        amplitude: f64,
        decay_seconds: f64,
        phase_cycles: f64,
    ) -> Result<Self, PercussionError> {
        let freq = validate_positive("frequency_hz", frequency_hz)?;
        let amp = validate_non_negative("amplitude", amplitude)?;
        let dec = validate_duration("decay_seconds", decay_seconds)?;
        if dec <= 0.0 {
            return Err(PercussionError::InvalidParameter {
                name: "decay_seconds",
                reason: format!("must be > 0.0, got {dec}"),
            });
        }
        let phase = validate_finite("phase_cycles", phase_cycles)?;

        Ok(Self {
            frequency_hz: freq,
            amplitude: amp,
            decay_seconds: dec,
            phase_cycles: phase,
        })
    }
}
