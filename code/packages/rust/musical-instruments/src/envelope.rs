//! Attack, decay, sustain, and release (ADSR) loudness envelopes (MUS05 §6).

use crate::error::InstrumentError;

/// Maximum allowable note duration in seconds (CWE-400 guard).
pub const MAX_NOTE_DURATION_SECONDS: f64 = 60.0;

/// Attack, decay, sustain, and release loudness shape.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdsrEnvelope {
    /// Attack duration in seconds (fade from 0.0 to 1.0).
    pub attack_seconds: f64,
    /// Decay duration in seconds (fade from 1.0 to sustain_level).
    pub decay_seconds: f64,
    /// Sustain level in [0.0, 1.0].
    pub sustain_level: f64,
    /// Release duration in seconds (fade from sustain_level to 0.0).
    pub release_seconds: f64,
}

/// Type alias for Python/spec naming parity.
pub type ADSREnvelope = AdsrEnvelope;

impl AdsrEnvelope {
    /// Construct a validated ADSR envelope.
    pub fn new(
        attack_seconds: f64,
        decay_seconds: f64,
        sustain_level: f64,
        release_seconds: f64,
    ) -> Result<Self, InstrumentError> {
        let atk = validate_duration("attack_seconds", attack_seconds)?;
        let dec = validate_duration("decay_seconds", decay_seconds)?;
        let sus = validate_unit("sustain_level", sustain_level)?;
        let rel = validate_duration("release_seconds", release_seconds)?;

        Ok(Self {
            attack_seconds: atk,
            decay_seconds: dec,
            sustain_level: sus,
            release_seconds: rel,
        })
    }

    /// Compute total rendered duration including the post-note release tail.
    pub fn rendered_duration_seconds(
        &self,
        note_duration_seconds: f64,
    ) -> Result<f64, InstrumentError> {
        let note_dur = validate_duration("note_duration_seconds", note_duration_seconds)?;
        Ok(note_dur + self.release_seconds)
    }

    /// Return the envelope amplitude gain in [0.0, 1.0] at note-local `time_seconds`.
    pub fn value_at(
        &self,
        time_seconds: f64,
        note_duration_seconds: f64,
    ) -> Result<f64, InstrumentError> {
        let time = validate_non_negative("time_seconds", time_seconds)?;
        let note_duration = validate_duration("note_duration_seconds", note_duration_seconds)?;

        if self.attack_seconds > 0.0 && time < self.attack_seconds {
            return Ok(time / self.attack_seconds);
        }

        let decay_end = self.attack_seconds + self.decay_seconds;
        if time < decay_end {
            let progress = if self.decay_seconds > 0.0 {
                (time - self.attack_seconds) / self.decay_seconds
            } else {
                1.0
            };
            return Ok(1.0 + (self.sustain_level - 1.0) * progress);
        }

        if time < note_duration {
            return Ok(self.sustain_level);
        }

        let release_time = time - note_duration;
        if self.release_seconds == 0.0 || release_time >= self.release_seconds {
            return Ok(0.0);
        }

        Ok(self.sustain_level * (1.0 - release_time / self.release_seconds))
    }
}

pub(crate) fn validate_non_negative(name: &'static str, value: f64) -> Result<f64, InstrumentError> {
    if !value.is_finite() {
        return Err(InstrumentError::NonFiniteNumber { name, value });
    }
    if value < 0.0 {
        return Err(InstrumentError::NegativeNumber { name, value });
    }
    Ok(value)
}

pub(crate) fn validate_positive(name: &'static str, value: f64) -> Result<f64, InstrumentError> {
    if !value.is_finite() {
        return Err(InstrumentError::NonFiniteNumber { name, value });
    }
    if value <= 0.0 {
        return Err(InstrumentError::NonPositiveNumber { name, value });
    }
    Ok(value)
}

pub(crate) fn validate_unit(name: &'static str, value: f64) -> Result<f64, InstrumentError> {
    if !value.is_finite() {
        return Err(InstrumentError::NonFiniteNumber { name, value });
    }
    if !(0.0..=1.0).contains(&value) {
        return Err(InstrumentError::ValueOutOfRange {
            name,
            min: 0.0,
            max: 1.0,
            actual: value,
        });
    }
    Ok(value)
}

pub(crate) fn validate_duration(name: &'static str, value: f64) -> Result<f64, InstrumentError> {
    let non_neg = validate_non_negative(name, value)?;
    if non_neg > MAX_NOTE_DURATION_SECONDS {
        return Err(InstrumentError::BudgetExceeded {
            reason: format!(
                "{name} {non_neg} exceeds MAX_NOTE_DURATION_SECONDS={MAX_NOTE_DURATION_SECONDS}"
            ),
        });
    }
    Ok(non_neg)
}
