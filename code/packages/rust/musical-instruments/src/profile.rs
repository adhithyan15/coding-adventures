//! Instrument timbre profiles (MUS05 §4 & §8).

use crate::envelope::{validate_unit, AdsrEnvelope};
use crate::error::InstrumentError;
use crate::partial::HarmonicPartial;
use crate::variation::VariationProfile;

/// Maximum allowable harmonic partials per instrument (CWE-400 guard).
pub const MAX_PARTIALS: usize = 64;

/// Default maximum allowable partial count.
pub const DEFAULT_MAX_PARTIAL_COUNT: usize = MAX_PARTIALS;

/// Allowed V1 synthesis kinds.
pub const SYNTHESIS_KINDS: &[&str] = &["sine", "additive", "silence"];

/// A selectable naive musical instrument timbre profile.
#[derive(Debug, Clone, PartialEq)]
pub struct InstrumentProfile {
    /// Stable machine-readable identifier (e.g. "flute_naive").
    pub id: String,
    /// Friendly display name (e.g. "Naive Flute").
    pub display_name: String,
    /// Synthesis method: "sine", "additive", or "silence".
    pub synthesis_kind: String,
    /// Normalized output gain in [0.0, 1.0].
    pub gain: f64,
    /// Harmonic overtone partials (up to MAX_PARTIALS).
    pub harmonic_profile: Vec<HarmonicPartial>,
    /// ADSR envelope shape.
    pub envelope_profile: AdsrEnvelope,
    /// Deterministic variation profile for subtle humanization.
    pub variation_profile: VariationProfile,
    /// General MIDI melodic program number in [1, 128], if assigned.
    pub gm_program: Option<u8>,
    /// Instrument family classification (e.g. "pipe", "strings").
    pub family: String,
}

impl InstrumentProfile {
    /// Construct and validate an instrument timbre profile.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: &str,
        display_name: &str,
        synthesis_kind: &str,
        gain: f64,
        harmonic_profile: Vec<HarmonicPartial>,
        envelope_profile: AdsrEnvelope,
        variation_profile: VariationProfile,
        gm_program: Option<u8>,
        family: &str,
    ) -> Result<Self, InstrumentError> {
        if id.trim().is_empty() {
            return Err(InstrumentError::InvalidParameter {
                name: "id",
                reason: "id must be non-empty".to_string(),
            });
        }
        if display_name.trim().is_empty() {
            return Err(InstrumentError::InvalidParameter {
                name: "display_name",
                reason: "display_name must be non-empty".to_string(),
            });
        }
        if !SYNTHESIS_KINDS.contains(&synthesis_kind) {
            return Err(InstrumentError::InvalidParameter {
                name: "synthesis_kind",
                reason: format!("unsupported synthesis_kind {synthesis_kind:?}"),
            });
        }
        let valid_gain = validate_unit("gain", gain)?;

        if harmonic_profile.len() > MAX_PARTIALS {
            return Err(InstrumentError::BudgetExceeded {
                reason: format!(
                    "harmonic_profile partial count {} exceeds MAX_PARTIALS={MAX_PARTIALS}",
                    harmonic_profile.len()
                ),
            });
        }
        if synthesis_kind != "silence" && harmonic_profile.is_empty() {
            return Err(InstrumentError::InvalidParameter {
                name: "harmonic_profile",
                reason: "non-silent instruments need at least one partial".to_string(),
            });
        }

        if let Some(prog) = gm_program {
            if !(1..=128).contains(&prog) {
                return Err(InstrumentError::UnknownProgram(prog));
            }
        }

        Ok(Self {
            id: id.to_string(),
            display_name: display_name.to_string(),
            synthesis_kind: synthesis_kind.to_string(),
            gain: valid_gain,
            harmonic_profile,
            envelope_profile,
            variation_profile,
            gm_program,
            family: family.to_string(),
        })
    }

    /// Sum of partial amplitudes for safe unity scaling before gain application.
    pub fn amplitude_normalizer(&self) -> f64 {
        let total: f64 = self.harmonic_profile.iter().map(|p| p.amplitude).sum();
        if total > 0.0 {
            total
        } else {
            1.0
        }
    }
}
