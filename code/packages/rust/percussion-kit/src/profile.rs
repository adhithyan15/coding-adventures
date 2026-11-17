//! Percussion voice profile definition and amplitude normalization (MUS07 §5.2).

use crate::envelope::PercussionEnvelope;
use crate::error::{validate_non_negative, validate_unit_float, PercussionError, MAX_PERCUSSION_MODES};
use crate::mode::PercussionMode;
use crate::noise::PercussionNoiseProfile;

/// A deterministic unpitched percussion voice profile (MUS07 §5.2).
#[derive(Debug, Clone, PartialEq)]
pub struct PercussionVoiceProfile {
    pub id: String,
    pub display_name: String,
    pub family: String,
    pub gain: f64,
    pub modes: Vec<PercussionMode>,
    pub noise_profile: Option<PercussionNoiseProfile>,
    pub envelope: PercussionEnvelope,
    pub click_amplitude: f64,
    pub choke_group: Option<String>,
}

impl PercussionVoiceProfile {
    /// Construct a new voice profile with comprehensive parameter validation.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        display_name: impl Into<String>,
        family: impl Into<String>,
        gain: f64,
        modes: Vec<PercussionMode>,
        noise_profile: Option<PercussionNoiseProfile>,
        envelope: PercussionEnvelope,
        click_amplitude: f64,
        choke_group: Option<String>,
    ) -> Result<Self, PercussionError> {
        let id_str = id.into();
        if id_str.trim().is_empty() {
            return Err(PercussionError::EmptyIdentifier { entity: "voice id" });
        }

        let name_str = display_name.into();
        if name_str.trim().is_empty() {
            return Err(PercussionError::EmptyIdentifier {
                entity: "voice display_name",
            });
        }

        let fam_str = family.into();
        if fam_str.trim().is_empty() {
            return Err(PercussionError::EmptyIdentifier { entity: "family" });
        }

        let valid_gain = validate_unit_float("gain", gain)?;
        let valid_click = validate_non_negative("click_amplitude", click_amplitude)?;

        if modes.len() > MAX_PERCUSSION_MODES {
            return Err(PercussionError::ModeCountTooLarge {
                count: modes.len(),
                max: MAX_PERCUSSION_MODES,
            });
        }

        let choke = choke_group.and_then(|cg| {
            let trimmed = cg.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        });

        Ok(Self {
            id: id_str,
            display_name: name_str,
            family: fam_str,
            gain: valid_gain,
            modes,
            noise_profile,
            envelope,
            click_amplitude: valid_click,
            choke_group: choke,
        })
    }

    /// Compute the sum of peak component amplitudes for gain normalization.
    pub fn amplitude_normalizer(&self) -> f64 {
        let mut total = self.click_amplitude;
        for mode in &self.modes {
            total += mode.amplitude;
        }
        if let Some(ref noise) = self.noise_profile {
            total += noise.amplitude;
        }
        if total > 0.0 {
            total
        } else {
            1.0
        }
    }
}
