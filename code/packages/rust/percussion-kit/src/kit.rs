//! Drum kit profiles, 15 standard starter voices, and canonical alias resolution (MUS07 §5.3, §6, §11).

use std::collections::BTreeMap;

use crate::envelope::PercussionEnvelope;
use crate::error::PercussionError;
use crate::mode::PercussionMode;
use crate::noise::PercussionNoiseProfile;
use crate::profile::PercussionVoiceProfile;

/// A named percussion kit mapping hit ids to voice profiles (MUS07 §5.3).
#[derive(Debug, Clone, PartialEq)]
pub struct DrumKitProfile {
    pub id: String,
    pub display_name: String,
    pub voices_by_hit_id: BTreeMap<String, PercussionVoiceProfile>,
}

impl DrumKitProfile {
    /// Construct a new drum kit profile.
    pub fn new(
        id: impl Into<String>,
        display_name: impl Into<String>,
        voices_by_hit_id: BTreeMap<String, PercussionVoiceProfile>,
    ) -> Result<Self, PercussionError> {
        let id_str = id.into();
        if id_str.trim().is_empty() {
            return Err(PercussionError::EmptyIdentifier { entity: "kit id" });
        }
        let name_str = display_name.into();
        if name_str.trim().is_empty() {
            return Err(PercussionError::EmptyIdentifier {
                entity: "kit display_name",
            });
        }
        if voices_by_hit_id.is_empty() {
            return Err(PercussionError::InvalidParameter {
                name: "voices_by_hit_id",
                reason: "kit must contain at least one voice profile".to_string(),
            });
        }
        for k in voices_by_hit_id.keys() {
            if k.trim().is_empty() {
                return Err(PercussionError::EmptyIdentifier {
                    entity: "kit voice key",
                });
            }
        }

        Ok(Self {
            id: id_str,
            display_name: name_str,
            voices_by_hit_id,
        })
    }
}

// ============================================================================
// Canonical Alias Resolution
// ============================================================================

/// Resolve a hit identifier string to its canonical starter hit name.
///
/// Supports common drummer / MIDI / music score abbreviations and aliases:
/// - `bd`, `bass_drum` -> `kick`
/// - `sd`, `snare_drum` -> `snare`
/// - `ch`, `closed_hat` -> `closed_hihat`
/// - `oh`, `open_hat` -> `open_hihat`
/// - `ph`, `pedal_hat`, `foot_hihat` -> `pedal_hihat`
/// - `lt`, `floor_tom` -> `low_tom`
/// - `mt` -> `mid_tom`
/// - `ht` -> `high_tom`
/// - `cc`, `crash_cymbal` -> `crash`
/// - `rc`, `ride_cymbal` -> `ride`
/// - `rim`, `side_stick`, `sidestick`, `rs` -> `rimshot`
/// - `cp`, `hand_clap`, `handclap` -> `clap`
/// - `cb` -> `cowbell`
/// - `tb`, `tamb` -> `tambourine`
/// - `sh` -> `shaker`
/// - `wb`, `wood_block` -> `woodblock`
pub fn canonical_hit_id(input: &str) -> Option<&'static str> {
    let trimmed = input.trim().to_ascii_lowercase();
    let normalized = trimmed.replace('-', "_");

    match normalized.as_str() {
        "kick" | "kick_naive" | "bd" | "bass_drum" | "bass" | "kick_drum" => Some("kick"),
        "snare" | "snare_naive" | "sd" | "snare_drum" => Some("snare"),
        "closed_hihat" | "closed_hihat_naive" | "closed_hat" | "hat_closed" | "ch" => {
            Some("closed_hihat")
        }
        "open_hihat" | "open_hihat_naive" | "open_hat" | "hat_open" | "oh" => Some("open_hihat"),
        "pedal_hihat" | "pedal_hihat_naive" | "pedal_hat" | "hat_pedal" | "foot_hihat"
        | "foot_hat" | "ph" => Some("pedal_hihat"),
        "low_tom" | "low_tom_naive" | "floor_tom" | "tom_low" | "lt" => Some("low_tom"),
        "mid_tom" | "mid_tom_naive" | "tom_mid" | "mt" => Some("mid_tom"),
        "high_tom" | "high_tom_naive" | "tom_high" | "ht" => Some("high_tom"),
        "crash" | "crash_naive" | "crash_cymbal" | "cc" => Some("crash"),
        "ride" | "ride_naive" | "ride_cymbal" | "rc" => Some("ride"),
        "rimshot" | "rimshot_naive" | "rim" | "side_stick" | "sidestick" | "rs" => {
            Some("rimshot")
        }
        "clap" | "clap_naive" | "hand_clap" | "handclap" | "cp" => Some("clap"),
        "cowbell" | "cowbell_naive" | "cb" => Some("cowbell"),
        "tambourine" | "tambourine_naive" | "tamb" | "tb" => Some("tambourine"),
        "shaker" | "shaker_naive" | "sh" => Some("shaker"),
        "woodblock" | "woodblock_naive" | "wood_block" | "wb" => Some("woodblock"),
        _ => None,
    }
}

// ============================================================================
// 15 Standard Kit Voice Profiles (MUS07 §4, §6, §11)
// ============================================================================

fn mode(freq: f64, amp: f64, dec: f64) -> PercussionMode {
    PercussionMode::new(freq, amp, dec).expect("valid mode")
}

fn env(att: f64, hld: f64, dec: f64) -> PercussionEnvelope {
    PercussionEnvelope::new(att, hld, dec).expect("valid envelope")
}

fn noise(amp: f64, dec: f64, seed: u64) -> Option<PercussionNoiseProfile> {
    Some(PercussionNoiseProfile::new(amp, dec, seed).expect("valid noise"))
}

/// 1. Bass drum / kick voice profile.
pub fn kick_naive() -> PercussionVoiceProfile {
    PercussionVoiceProfile::new(
        "kick_naive",
        "Naive Kick",
        "kick",
        0.90,
        vec![
            mode(58.0, 1.0, 0.20),
            mode(94.0, 0.35, 0.12),
            mode(142.0, 0.10, 0.06),
        ],
        noise(0.08, 0.015, 11),
        env(0.0, 0.015, 0.22),
        0.55,
        None,
    )
    .expect("valid kick")
}

/// 2. Snare drum voice profile.
pub fn snare_naive() -> PercussionVoiceProfile {
    PercussionVoiceProfile::new(
        "snare_naive",
        "Naive Snare",
        "snare",
        0.75,
        vec![mode(180.0, 0.30, 0.16), mode(330.0, 0.15, 0.09)],
        noise(0.95, 0.11, 23),
        env(0.0, 0.008, 0.18),
        0.28,
        None,
    )
    .expect("valid snare")
}

/// 3. Closed hi-hat voice profile (choke_group = "hihat").
pub fn closed_hihat_naive() -> PercussionVoiceProfile {
    PercussionVoiceProfile::new(
        "closed_hihat_naive",
        "Naive Closed Hi-Hat",
        "hat",
        0.45,
        vec![mode(6200.0, 0.24, 0.05), mode(9100.0, 0.18, 0.04)],
        noise(0.92, 0.05, 31),
        env(0.0, 0.003, 0.06),
        0.10,
        Some("hihat".to_string()),
    )
    .expect("valid closed hihat")
}

/// 4. Open hi-hat voice profile (choke_group = "hihat").
pub fn open_hihat_naive() -> PercussionVoiceProfile {
    PercussionVoiceProfile::new(
        "open_hihat_naive",
        "Naive Open Hi-Hat",
        "hat",
        0.42,
        vec![mode(5900.0, 0.24, 0.20), mode(8700.0, 0.16, 0.24)],
        noise(0.95, 0.26, 37),
        env(0.0, 0.004, 0.32),
        0.08,
        Some("hihat".to_string()),
    )
    .expect("valid open hihat")
}

/// 5. Pedal / foot hi-hat voice profile (choke_group = "hihat").
pub fn pedal_hihat_naive() -> PercussionVoiceProfile {
    PercussionVoiceProfile::new(
        "pedal_hihat_naive",
        "Naive Pedal Hi-Hat",
        "hat",
        0.40,
        vec![mode(5600.0, 0.18, 0.06), mode(8200.0, 0.12, 0.05)],
        noise(0.78, 0.07, 41),
        env(0.0, 0.002, 0.08),
        0.12,
        Some("hihat".to_string()),
    )
    .expect("valid pedal hihat")
}

/// 6. Low tom voice profile.
pub fn low_tom_naive() -> PercussionVoiceProfile {
    PercussionVoiceProfile::new(
        "low_tom_naive",
        "Naive Low Tom",
        "tom",
        0.72,
        vec![
            mode(110.0, 1.0, 0.22),
            mode(170.0, 0.28, 0.16),
            mode(250.0, 0.10, 0.10),
        ],
        noise(0.10, 0.03, 43),
        env(0.0, 0.010, 0.20),
        0.20,
        None,
    )
    .expect("valid low tom")
}

/// 7. Middle tom voice profile.
pub fn mid_tom_naive() -> PercussionVoiceProfile {
    PercussionVoiceProfile::new(
        "mid_tom_naive",
        "Naive Mid Tom",
        "tom",
        0.70,
        vec![
            mode(150.0, 1.0, 0.20),
            mode(225.0, 0.25, 0.14),
            mode(320.0, 0.08, 0.08),
        ],
        noise(0.08, 0.025, 47),
        env(0.0, 0.008, 0.18),
        0.18,
        None,
    )
    .expect("valid mid tom")
}

/// 8. High tom voice profile.
pub fn high_tom_naive() -> PercussionVoiceProfile {
    PercussionVoiceProfile::new(
        "high_tom_naive",
        "Naive High Tom",
        "tom",
        0.68,
        vec![
            mode(205.0, 1.0, 0.17),
            mode(295.0, 0.22, 0.12),
            mode(405.0, 0.07, 0.07),
        ],
        noise(0.06, 0.020, 53),
        env(0.0, 0.007, 0.15),
        0.16,
        None,
    )
    .expect("valid high tom")
}

/// 9. Crash cymbal voice profile.
pub fn crash_naive() -> PercussionVoiceProfile {
    PercussionVoiceProfile::new(
        "crash_naive",
        "Naive Crash",
        "cymbal",
        0.38,
        vec![
            mode(3200.0, 0.18, 0.35),
            mode(5100.0, 0.12, 0.48),
            mode(7900.0, 0.08, 0.55),
        ],
        noise(1.0, 0.60, 59),
        env(0.0, 0.004, 0.65),
        0.12,
        None,
    )
    .expect("valid crash")
}

/// 10. Ride cymbal voice profile.
pub fn ride_naive() -> PercussionVoiceProfile {
    PercussionVoiceProfile::new(
        "ride_naive",
        "Naive Ride",
        "cymbal",
        0.36,
        vec![
            mode(2600.0, 0.14, 0.32),
            mode(4100.0, 0.10, 0.44),
            mode(6500.0, 0.08, 0.50),
        ],
        noise(0.82, 0.42, 61),
        env(0.0, 0.004, 0.45),
        0.10,
        None,
    )
    .expect("valid ride")
}

/// 11. Snare rimshot voice profile.
pub fn rimshot_naive() -> PercussionVoiceProfile {
    PercussionVoiceProfile::new(
        "rimshot_naive",
        "Naive Rimshot",
        "snare",
        0.70,
        vec![
            mode(450.0, 0.40, 0.05),
            mode(900.0, 0.30, 0.04),
            mode(1600.0, 0.15, 0.03),
        ],
        noise(0.60, 0.04, 29),
        env(0.0, 0.003, 0.08),
        0.60,
        None,
    )
    .expect("valid rimshot")
}

/// 12. Hand clap voice profile.
pub fn clap_naive() -> PercussionVoiceProfile {
    PercussionVoiceProfile::new(
        "clap_naive",
        "Naive Clap",
        "hand_percussion",
        0.70,
        vec![
            mode(800.0, 0.40, 0.06),
            mode(1250.0, 0.35, 0.05),
            mode(2500.0, 0.20, 0.03),
        ],
        noise(0.88, 0.08, 67),
        env(0.0, 0.008, 0.14),
        0.35,
        None,
    )
    .expect("valid clap")
}

/// 13. Cowbell voice profile.
pub fn cowbell_naive() -> PercussionVoiceProfile {
    PercussionVoiceProfile::new(
        "cowbell_naive",
        "Naive Cowbell",
        "hand_percussion",
        0.65,
        vec![
            mode(587.0, 1.0, 0.25),
            mode(845.0, 0.70, 0.20),
            mode(1450.0, 0.15, 0.10),
        ],
        noise(0.08, 0.015, 71),
        env(0.0, 0.004, 0.28),
        0.45,
        None,
    )
    .expect("valid cowbell")
}

/// 14. Tambourine voice profile.
pub fn tambourine_naive() -> PercussionVoiceProfile {
    PercussionVoiceProfile::new(
        "tambourine_naive",
        "Naive Tambourine",
        "hand_percussion",
        0.45,
        vec![
            mode(4800.0, 0.25, 0.12),
            mode(7200.0, 0.20, 0.15),
            mode(10500.0, 0.15, 0.18),
        ],
        noise(0.90, 0.16, 73),
        env(0.0, 0.003, 0.22),
        0.22,
        None,
    )
    .expect("valid tambourine")
}

/// 15. Shaker voice profile.
pub fn shaker_naive() -> PercussionVoiceProfile {
    PercussionVoiceProfile::new(
        "shaker_naive",
        "Naive Shaker",
        "hand_percussion",
        0.40,
        vec![mode(3200.0, 0.08, 0.08), mode(6400.0, 0.06, 0.06)],
        noise(0.95, 0.10, 79),
        env(0.010, 0.005, 0.12),
        0.04,
        None,
    )
    .expect("valid shaker")
}

/// 16. Woodblock voice profile.
pub fn woodblock_naive() -> PercussionVoiceProfile {
    PercussionVoiceProfile::new(
        "woodblock_naive",
        "Naive Woodblock",
        "woodblock",
        0.72,
        vec![
            mode(780.0, 1.0, 0.04),
            mode(1650.0, 0.30, 0.025),
            mode(2800.0, 0.10, 0.015),
        ],
        noise(0.05, 0.010, 83),
        env(0.0, 0.002, 0.055),
        0.60,
        None,
    )
    .expect("valid woodblock")
}

/// Build the standard percussion drum kit profile containing all starter voices.
pub fn standard_kit() -> DrumKitProfile {
    let mut voices = BTreeMap::new();
    voices.insert("kick".to_string(), kick_naive());
    voices.insert("snare".to_string(), snare_naive());
    voices.insert("closed_hihat".to_string(), closed_hihat_naive());
    voices.insert("open_hihat".to_string(), open_hihat_naive());
    voices.insert("pedal_hihat".to_string(), pedal_hihat_naive());
    voices.insert("low_tom".to_string(), low_tom_naive());
    voices.insert("mid_tom".to_string(), mid_tom_naive());
    voices.insert("high_tom".to_string(), high_tom_naive());
    voices.insert("crash".to_string(), crash_naive());
    voices.insert("ride".to_string(), ride_naive());
    voices.insert("rimshot".to_string(), rimshot_naive());
    voices.insert("clap".to_string(), clap_naive());
    voices.insert("cowbell".to_string(), cowbell_naive());
    voices.insert("tambourine".to_string(), tambourine_naive());
    voices.insert("shaker".to_string(), shaker_naive());
    voices.insert("woodblock".to_string(), woodblock_naive());

    DrumKitProfile::new("standard_kit", "Standard Kit", voices).expect("valid standard kit")
}

/// Return all built-in percussion kits.
pub fn all_drum_kits() -> Vec<DrumKitProfile> {
    vec![standard_kit()]
}

/// The 15 portable baseline starter hit ids from MUS07 §6 & Python reference.
pub const ALL_STANDARD_HIT_IDS: &[&str] = &[
    "kick",
    "snare",
    "closed_hihat",
    "open_hihat",
    "pedal_hihat",
    "low_tom",
    "mid_tom",
    "high_tom",
    "crash",
    "ride",
    "clap",
    "cowbell",
    "tambourine",
    "shaker",
    "woodblock",
];

/// Return the portable starter hit ids for the built-in standard kit.
pub fn all_standard_hit_ids() -> &'static [&'static str] {
    ALL_STANDARD_HIT_IDS
}

/// Resolve a built-in kit id.
pub fn get_drum_kit(kit_id: &str) -> Result<DrumKitProfile, PercussionError> {
    if kit_id == "standard_kit" || kit_id == "standard" {
        Ok(standard_kit())
    } else {
        Err(PercussionError::UnknownKit {
            kit_id: kit_id.to_string(),
        })
    }
}

/// Resolve one hit id through an optional drum kit (defaults to standard kit) with canonical alias fallback.
pub fn get_percussion_voice(
    hit_id: &str,
    kit: Option<&DrumKitProfile>,
) -> Result<PercussionVoiceProfile, PercussionError> {
    if hit_id.trim().is_empty() {
        return Err(PercussionError::EmptyIdentifier { entity: "hit_id" });
    }

    let default_kit;
    let active_kit = match kit {
        Some(k) => k,
        None => {
            default_kit = standard_kit();
            &default_kit
        }
    };

    // 1. Direct hit id lookup
    if let Some(voice) = active_kit.voices_by_hit_id.get(hit_id) {
        return Ok(voice.clone());
    }

    // 2. Canonical alias lookup
    if let Some(canon) = canonical_hit_id(hit_id) {
        if let Some(voice) = active_kit.voices_by_hit_id.get(canon) {
            return Ok(voice.clone());
        }
    }

    Err(PercussionError::UnknownVoice {
        hit_id: hit_id.to_string(),
        kit_id: active_kit.id.clone(),
    })
}
