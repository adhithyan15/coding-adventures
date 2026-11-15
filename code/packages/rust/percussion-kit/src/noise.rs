//! Deterministic pseudo-random noise burst models (MUS07 §4.1).

use crate::error::{validate_duration, validate_non_negative, PercussionError};
use crate::splitmix64::splitmix64_noise;

/// Selector for the underlying deterministic noise synthesis algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NoiseAlgorithm {
    /// Golden-ratio phase offset and sine fraction folding (Python MUS07 baseline).
    #[default]
    PseudoNoise,
    /// Fast 64-bit SplitMix64 pseudo-random generator.
    SplitMix64,
}

/// Deterministic decaying noise burst profile (MUS07 §4.1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PercussionNoiseProfile {
    pub amplitude: f64,
    pub decay_seconds: f64,
    pub seed: u64,
    pub algorithm: NoiseAlgorithm,
}

impl PercussionNoiseProfile {
    /// Construct a new noise profile using the default [`NoiseAlgorithm::PseudoNoise`].
    pub fn new(
        amplitude: f64,
        decay_seconds: f64,
        seed: u64,
    ) -> Result<Self, PercussionError> {
        Self::with_algorithm(amplitude, decay_seconds, seed, NoiseAlgorithm::PseudoNoise)
    }

    /// Construct a new noise profile with a specific noise synthesis algorithm.
    pub fn with_algorithm(
        amplitude: f64,
        decay_seconds: f64,
        seed: u64,
        algorithm: NoiseAlgorithm,
    ) -> Result<Self, PercussionError> {
        let amp = validate_non_negative("amplitude", amplitude)?;
        let dec = validate_duration("decay_seconds", decay_seconds)?;
        if dec <= 0.0 {
            return Err(PercussionError::InvalidParameter {
                name: "decay_seconds",
                reason: format!("must be > 0.0, got {dec}"),
            });
        }

        Ok(Self {
            amplitude: amp,
            decay_seconds: dec,
            seed,
            algorithm,
        })
    }

    /// Sample the noise source at the given sample index.
    #[inline]
    pub fn sample_noise(&self, sample_index: usize) -> f64 {
        match self.algorithm {
            NoiseAlgorithm::PseudoNoise => pseudo_noise(sample_index, self.seed),
            NoiseAlgorithm::SplitMix64 => splitmix64_noise(sample_index, self.seed),
        }
    }
}

/// Deterministic hash-based pseudo-random noise sample in `[-1.0, 1.0]`.
///
/// Uses the golden ratio phase offset and sine fraction folding to generate
/// statistically uniform noise without allocating state or using external PRNGs.
/// Matches Python `_pseudo_noise` for bit-level / epsilon-level MUS07 parity.
#[inline]
pub fn pseudo_noise(sample_index: usize, seed: u64) -> f64 {
    let phase = (sample_index as f64 + 1.0) * (seed as f64 * 0.161_803_398_874_989_5 + 0.618_033_988_75);
    let val = trig::sin(2.0 * trig::PI * phase) * 43_758.545_312_3;
    let frac = val - val.floor();
    2.0 * frac - 1.0
}
