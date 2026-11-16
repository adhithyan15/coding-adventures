//! Harmonic partials for additive synthesis (MUS05 §5).

use crate::error::InstrumentError;

/// One sine component inside an additive instrument profile.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HarmonicPartial {
    /// Frequency multiplier relative to fundamental frequency (must be > 0.0).
    pub frequency_multiplier: f64,
    /// Relative amplitude (must be >= 0.0).
    pub amplitude: f64,
    /// Initial phase offset in cycles [0.0, 1.0) (must be finite).
    pub phase_cycles: f64,
}

impl HarmonicPartial {
    /// Construct a new harmonic partial with default `phase_cycles = 0.0`.
    pub fn new(frequency_multiplier: f64, amplitude: f64) -> Result<Self, InstrumentError> {
        Self::with_phase(frequency_multiplier, amplitude, 0.0)
    }

    /// Construct a harmonic partial with explicit phase offset in cycles.
    pub fn with_phase(
        frequency_multiplier: f64,
        amplitude: f64,
        phase_cycles: f64,
    ) -> Result<Self, InstrumentError> {
        if !frequency_multiplier.is_finite() {
            return Err(InstrumentError::NonFiniteNumber {
                name: "frequency_multiplier",
                value: frequency_multiplier,
            });
        }
        if frequency_multiplier <= 0.0 {
            return Err(InstrumentError::NonPositiveNumber {
                name: "frequency_multiplier",
                value: frequency_multiplier,
            });
        }

        if !amplitude.is_finite() {
            return Err(InstrumentError::NonFiniteNumber {
                name: "amplitude",
                value: amplitude,
            });
        }
        if amplitude < 0.0 {
            return Err(InstrumentError::NegativeNumber {
                name: "amplitude",
                value: amplitude,
            });
        }

        if !phase_cycles.is_finite() {
            return Err(InstrumentError::NonFiniteNumber {
                name: "phase_cycles",
                value: phase_cycles,
            });
        }

        Ok(Self {
            frequency_multiplier,
            amplitude,
            phase_cycles,
        })
    }
}
