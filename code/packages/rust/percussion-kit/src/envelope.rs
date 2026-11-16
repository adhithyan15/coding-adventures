//! Fast percussion amplitude shape models: attack, hold, then decay (MUS07 §4.3).

use crate::error::{validate_duration, validate_non_negative, PercussionError};

/// Fast percussion amplitude shape: attack, hold, then decay (MUS07 §4.3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PercussionEnvelope {
    pub attack_seconds: f64,
    pub hold_seconds: f64,
    pub decay_seconds: f64,
}

impl PercussionEnvelope {
    /// Construct a new percussion envelope with strict boundary verification.
    pub fn new(
        attack_seconds: f64,
        hold_seconds: f64,
        decay_seconds: f64,
    ) -> Result<Self, PercussionError> {
        let att = validate_duration("attack_seconds", attack_seconds)?;
        let hld = validate_duration("hold_seconds", hold_seconds)?;
        let dec = validate_duration("decay_seconds", decay_seconds)?;
        if dec <= 0.0 {
            return Err(PercussionError::InvalidParameter {
                name: "decay_seconds",
                reason: format!("must be > 0.0, got {dec}"),
            });
        }

        Ok(Self {
            attack_seconds: att,
            hold_seconds: hld,
            decay_seconds: dec,
        })
    }

    /// Compute the total physical decay duration in seconds.
    pub fn rendered_duration_seconds(
        &self,
        hit_duration_seconds: f64,
    ) -> Result<f64, PercussionError> {
        let hit_dur = validate_duration("hit_duration_seconds", hit_duration_seconds)?;
        let total = hit_dur + self.decay_seconds;
        if !total.is_finite() {
            return Err(PercussionError::NonFiniteNumber {
                name: "rendered_duration",
                value: total,
            });
        }
        Ok(total)
    }

    /// Evaluate envelope gain in `[0.0, 1.0]` at a given elapsed time.
    pub fn value_at(
        &self,
        time_seconds: f64,
        hit_duration_seconds: f64,
    ) -> Result<f64, PercussionError> {
        let time = validate_non_negative("time_seconds", time_seconds)?;
        let hit_dur = validate_non_negative("hit_duration_seconds", hit_duration_seconds)?;

        if time < self.attack_seconds {
            if self.attack_seconds <= 0.0 {
                return Ok(1.0);
            }
            return Ok(time / self.attack_seconds);
        }

        let sustain_end = self.attack_seconds + self.hold_seconds.max(hit_dur);
        if time < sustain_end {
            return Ok(1.0);
        }

        let decay_time = time - sustain_end;
        if decay_time >= self.decay_seconds {
            return Ok(0.0);
        }

        let frac = decay_time / self.decay_seconds;
        Ok((1.0 - frac).clamp(0.0, 1.0))
    }
}
