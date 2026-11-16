//! Deterministic variation profiles for natural instrument humanization (MUS05 §7).

use crate::envelope::validate_non_negative;
use crate::error::InstrumentError;

/// Deterministic variation knobs for subtle micro-variations in pitch, amplitude,
/// attack timing, and harmonic overtone distribution.
#[derive(Debug, Clone, PartialEq)]
pub struct VariationProfile {
    /// Pitch jitter range in cents (100 cents = 1 semitone).
    pub pitch_cents_jitter: f64,
    /// Amplitude multiplier jitter range.
    pub amplitude_jitter: f64,
    /// Attack timing jitter range in milliseconds.
    pub attack_jitter_ms: f64,
    /// Deterministic SplitMix64 PRNG seed. If None, variation is inactive.
    pub seed: Option<u64>,
    /// Harmonic overtone relative jitter range.
    pub harmonic_jitter: f64,
}

impl Default for VariationProfile {
    fn default() -> Self {
        Self {
            pitch_cents_jitter: 0.0,
            amplitude_jitter: 0.0,
            attack_jitter_ms: 0.0,
            seed: None,
            harmonic_jitter: 0.0,
        }
    }
}

impl VariationProfile {
    /// Construct a validated VariationProfile.
    pub fn new(
        pitch_cents_jitter: f64,
        amplitude_jitter: f64,
        attack_jitter_ms: f64,
        seed: Option<u64>,
        harmonic_jitter: f64,
    ) -> Result<Self, InstrumentError> {
        let p_jitter = validate_non_negative("pitch_cents_jitter", pitch_cents_jitter)?;
        let a_jitter = validate_non_negative("amplitude_jitter", amplitude_jitter)?;
        let atk_jitter = validate_non_negative("attack_jitter_ms", attack_jitter_ms)?;
        let h_jitter = validate_non_negative("harmonic_jitter", harmonic_jitter)?;

        Ok(Self {
            pitch_cents_jitter: p_jitter,
            amplitude_jitter: a_jitter,
            attack_jitter_ms: atk_jitter,
            seed,
            harmonic_jitter: h_jitter,
        })
    }

    /// Construct a variation profile with a specific deterministic PRNG seed.
    pub fn with_seed(seed: u64) -> Self {
        Self {
            seed: Some(seed),
            ..Self::default()
        }
    }

    /// Alias for `pitch_cents_jitter`.
    pub fn pitch_jitter_cents(&self) -> f64 {
        self.pitch_cents_jitter
    }

    /// Alias for `attack_jitter_ms` converted to seconds.
    pub fn timing_jitter_seconds(&self) -> f64 {
        self.attack_jitter_ms / 1000.0
    }

    /// Set pitch cents jitter.
    pub fn with_pitch_cents_jitter(mut self, jitter: f64) -> Result<Self, InstrumentError> {
        self.pitch_cents_jitter = validate_non_negative("pitch_cents_jitter", jitter)?;
        Ok(self)
    }

    /// Set amplitude jitter.
    pub fn with_amplitude_jitter(mut self, jitter: f64) -> Result<Self, InstrumentError> {
        self.amplitude_jitter = validate_non_negative("amplitude_jitter", jitter)?;
        Ok(self)
    }

    /// Set attack jitter in milliseconds.
    pub fn with_attack_jitter_ms(mut self, jitter_ms: f64) -> Result<Self, InstrumentError> {
        self.attack_jitter_ms = validate_non_negative("attack_jitter_ms", jitter_ms)?;
        Ok(self)
    }

    /// Set harmonic jitter.
    pub fn with_harmonic_jitter(mut self, jitter: f64) -> Result<Self, InstrumentError> {
        self.harmonic_jitter = validate_non_negative("harmonic_jitter", jitter)?;
        Ok(self)
    }

    /// Set deterministic PRNG seed.
    pub fn set_seed(mut self, seed: Option<u64>) -> Self {
        self.seed = seed;
        self
    }
}
