//! Built-in naive instrument timbre presets (MUS05 §8).

use crate::envelope::AdsrEnvelope;
use crate::error::InstrumentError;
use crate::partial::HarmonicPartial;
use crate::profile::InstrumentProfile;
use crate::variation::VariationProfile;

fn p(mult: f64, amp: f64) -> HarmonicPartial {
    HarmonicPartial::new(mult, amp).expect("valid preset partial")
}

fn env(atk: f64, dec: f64, sus: f64, rel: f64) -> AdsrEnvelope {
    AdsrEnvelope::new(atk, dec, sus, rel).expect("valid preset envelope")
}

/// Pure Sine instrument preset.
pub fn sine() -> InstrumentProfile {
    InstrumentProfile::new(
        "sine",
        "Pure Sine",
        "sine",
        1.0,
        vec![p(1.0, 1.0)],
        env(0.005, 0.010, 0.90, 0.030),
        VariationProfile::default(),
        None,
        "synth",
    )
    .expect("valid preset")
}

/// Silence instrument preset.
pub fn silence() -> InstrumentProfile {
    InstrumentProfile::new(
        "silence",
        "Silence",
        "silence",
        0.0,
        vec![],
        env(0.0, 0.0, 0.0, 0.0),
        VariationProfile::default(),
        None,
        "utility",
    )
    .expect("valid preset")
}

/// Naive Flute instrument preset.
pub fn flute_naive() -> InstrumentProfile {
    InstrumentProfile::new(
        "flute_naive",
        "Naive Flute",
        "additive",
        0.95,
        vec![p(1.0, 1.00), p(2.0, 0.10), p(3.0, 0.03)],
        env(0.080, 0.050, 0.85, 0.080),
        VariationProfile::default(),
        None,
        "pipe",
    )
    .expect("valid preset")
}

/// Naive Clarinet instrument preset.
pub fn clarinet_naive() -> InstrumentProfile {
    InstrumentProfile::new(
        "clarinet_naive",
        "Naive Clarinet",
        "additive",
        0.78,
        vec![p(1.0, 1.00), p(3.0, 0.45), p(5.0, 0.20), p(7.0, 0.08)],
        env(0.040, 0.060, 0.75, 0.070),
        VariationProfile::default(),
        None,
        "reed",
    )
    .expect("valid preset")
}

/// Naive Violin instrument preset.
pub fn violin_naive() -> InstrumentProfile {
    InstrumentProfile::new(
        "violin_naive",
        "Naive Violin",
        "additive",
        0.62,
        vec![
            p(1.0, 1.00),
            p(2.0, 0.55),
            p(3.0, 0.35),
            p(4.0, 0.20),
            p(5.0, 0.12),
        ],
        env(0.070, 0.080, 0.80, 0.120),
        VariationProfile::default(),
        None,
        "strings",
    )
    .expect("valid preset")
}

/// Naive Piano instrument preset.
pub fn piano_naive() -> InstrumentProfile {
    InstrumentProfile::new(
        "piano_naive",
        "Naive Piano",
        "additive",
        0.82,
        vec![
            p(1.0, 1.00),
            p(2.0, 0.45),
            p(3.0, 0.25),
            p(4.0, 0.14),
            p(5.0, 0.08),
        ],
        env(0.005, 0.350, 0.18, 0.180),
        VariationProfile::default(),
        None,
        "piano",
    )
    .expect("valid preset")
}

/// Naive Plucked String instrument preset.
pub fn pluck_naive() -> InstrumentProfile {
    InstrumentProfile::new(
        "pluck_naive",
        "Naive Plucked String",
        "additive",
        0.66,
        vec![
            p(1.0, 1.00),
            p(2.0, 0.60),
            p(3.0, 0.38),
            p(4.0, 0.22),
            p(5.0, 0.13),
            p(6.0, 0.08),
        ],
        env(0.003, 0.450, 0.05, 0.080),
        VariationProfile::default(),
        None,
        "plucked",
    )
    .expect("valid preset")
}

/// Naive Brass instrument preset.
pub fn brass_naive() -> InstrumentProfile {
    InstrumentProfile::new(
        "brass_naive",
        "Naive Brass",
        "additive",
        0.64,
        vec![
            p(1.0, 1.00),
            p(2.0, 0.70),
            p(3.0, 0.42),
            p(4.0, 0.22),
            p(5.0, 0.14),
        ],
        env(0.030, 0.070, 0.78, 0.090),
        VariationProfile::default(),
        None,
        "brass",
    )
    .expect("valid preset")
}

/// Naive Organ instrument preset.
pub fn organ_naive() -> InstrumentProfile {
    InstrumentProfile::new(
        "organ_naive",
        "Naive Organ",
        "additive",
        0.55,
        vec![p(1.0, 1.00), p(2.0, 0.75), p(3.0, 0.45), p(4.0, 0.25)],
        env(0.010, 0.010, 0.95, 0.040),
        VariationProfile::default(),
        None,
        "organ",
    )
    .expect("valid preset")
}

/// Naive Mallet instrument preset.
pub fn mallet_naive() -> InstrumentProfile {
    InstrumentProfile::new(
        "mallet_naive",
        "Naive Mallet",
        "additive",
        0.74,
        vec![p(1.0, 1.00), p(2.7, 0.28), p(5.1, 0.12)],
        env(0.002, 0.600, 0.02, 0.120),
        VariationProfile::default(),
        None,
        "chromatic percussion",
    )
    .expect("valid preset")
}

/// Naive Celesta instrument preset.
pub fn celesta_naive() -> InstrumentProfile {
    InstrumentProfile::new(
        "celesta_naive",
        "Naive Celesta",
        "additive",
        0.60,
        vec![p(1.0, 1.00), p(2.0, 0.35), p(4.0, 0.18), p(6.3, 0.08)],
        env(0.004, 0.900, 0.10, 0.250),
        VariationProfile::default(),
        None,
        "chromatic percussion",
    )
    .expect("valid preset")
}

/// Naive Glockenspiel instrument preset.
pub fn glockenspiel_naive() -> InstrumentProfile {
    InstrumentProfile::new(
        "glockenspiel_naive",
        "Naive Glockenspiel",
        "additive",
        0.50,
        vec![p(1.0, 1.00), p(2.8, 0.55), p(5.4, 0.30), p(8.9, 0.15)],
        env(0.001, 0.900, 0.02, 0.500),
        VariationProfile::default(),
        None,
        "chromatic percussion",
    )
    .expect("valid preset")
}

/// Naive Vibraphone instrument preset.
pub fn vibraphone_naive() -> InstrumentProfile {
    InstrumentProfile::new(
        "vibraphone_naive",
        "Naive Vibraphone",
        "additive",
        0.58,
        vec![p(1.0, 1.00), p(2.0, 0.35), p(3.9, 0.16), p(6.8, 0.06)],
        env(0.003, 0.800, 0.28, 0.450),
        VariationProfile::default(),
        None,
        "chromatic percussion",
    )
    .expect("valid preset")
}

/// Naive Marimba instrument preset.
pub fn marimba_naive() -> InstrumentProfile {
    InstrumentProfile::new(
        "marimba_naive",
        "Naive Marimba",
        "additive",
        0.68,
        vec![p(1.0, 1.00), p(4.0, 0.30), p(10.2, 0.10)],
        env(0.002, 0.550, 0.03, 0.120),
        VariationProfile::default(),
        None,
        "chromatic percussion",
    )
    .expect("valid preset")
}

/// Naive Xylophone instrument preset.
pub fn xylophone_naive() -> InstrumentProfile {
    InstrumentProfile::new(
        "xylophone_naive",
        "Naive Xylophone",
        "additive",
        0.64,
        vec![p(1.0, 1.00), p(3.2, 0.32), p(6.1, 0.14)],
        env(0.001, 0.350, 0.01, 0.080),
        VariationProfile::default(),
        None,
        "chromatic percussion",
    )
    .expect("valid preset")
}

/// Naive Tubular Bells instrument preset.
pub fn tubular_bells_naive() -> InstrumentProfile {
    InstrumentProfile::new(
        "tubular_bells_naive",
        "Naive Tubular Bells",
        "additive",
        0.46,
        vec![
            p(1.0, 1.00),
            p(2.0, 0.20),
            p(3.0, 0.12),
            p(4.2, 0.28),
            p(6.8, 0.14),
        ],
        env(0.003, 1.300, 0.04, 0.800),
        VariationProfile::default(),
        None,
        "chromatic percussion",
    )
    .expect("valid preset")
}

/// Naive Timpani instrument preset.
pub fn timpani_naive() -> InstrumentProfile {
    InstrumentProfile::new(
        "timpani_naive",
        "Naive Timpani",
        "additive",
        0.70,
        vec![
            p(1.0, 1.00),
            p(1.5, 0.25),
            p(2.0, 0.38),
            p(2.44, 0.18),
            p(3.0, 0.08),
        ],
        env(0.003, 0.700, 0.06, 0.200),
        VariationProfile::default(),
        None,
        "pitched percussion",
    )
    .expect("valid preset")
}

/// Naive Kalimba instrument preset.
pub fn kalimba_naive() -> InstrumentProfile {
    InstrumentProfile::new(
        "kalimba_naive",
        "Naive Kalimba",
        "additive",
        0.63,
        vec![p(1.0, 1.00), p(2.0, 0.22), p(3.0, 0.08), p(6.0, 0.18)],
        env(0.002, 0.450, 0.04, 0.150),
        VariationProfile::default(),
        None,
        "pitched percussion",
    )
    .expect("valid preset")
}

/// Naive Synth Lead instrument preset.
pub fn synth_lead_naive() -> InstrumentProfile {
    InstrumentProfile::new(
        "synth_lead_naive",
        "Naive Synth Lead",
        "additive",
        0.58,
        vec![
            p(1.0, 1.00),
            p(2.0, 0.85),
            p(3.0, 0.65),
            p(4.0, 0.48),
            p(5.0, 0.36),
            p(6.0, 0.24),
        ],
        env(0.004, 0.040, 0.80, 0.040),
        VariationProfile::default(),
        None,
        "synth lead",
    )
    .expect("valid preset")
}

/// Naive Synth Pad instrument preset.
pub fn synth_pad_naive() -> InstrumentProfile {
    InstrumentProfile::new(
        "synth_pad_naive",
        "Naive Synth Pad",
        "additive",
        0.46,
        vec![p(1.0, 1.00), p(2.0, 0.35), p(3.0, 0.18), p(5.0, 0.10)],
        env(0.450, 0.400, 0.75, 0.550),
        VariationProfile::default(),
        None,
        "synth pad",
    )
    .expect("valid preset")
}

/// Naive Sound Effect instrument preset.
pub fn effect_naive() -> InstrumentProfile {
    InstrumentProfile::new(
        "effect_naive",
        "Naive Effect",
        "additive",
        0.45,
        vec![p(1.0, 1.00), p(1.5, 0.55), p(2.25, 0.35), p(3.7, 0.18)],
        env(0.020, 0.700, 0.20, 0.250),
        VariationProfile::default(),
        None,
        "sound effects",
    )
    .expect("valid preset")
}

/// Return all 21 built-in naive instrument presets.
pub fn all_instruments() -> Vec<InstrumentProfile> {
    vec![
        sine(),
        silence(),
        flute_naive(),
        clarinet_naive(),
        violin_naive(),
        piano_naive(),
        pluck_naive(),
        brass_naive(),
        organ_naive(),
        mallet_naive(),
        celesta_naive(),
        glockenspiel_naive(),
        vibraphone_naive(),
        marimba_naive(),
        xylophone_naive(),
        tubular_bells_naive(),
        timpani_naive(),
        kalimba_naive(),
        synth_lead_naive(),
        synth_pad_naive(),
        effect_naive(),
    ]
}

/// Look up a built-in instrument preset by id.
pub fn get_instrument(id: &str) -> Result<InstrumentProfile, InstrumentError> {
    match id {
        "sine" => Ok(sine()),
        "silence" => Ok(silence()),
        "flute_naive" => Ok(flute_naive()),
        "clarinet_naive" => Ok(clarinet_naive()),
        "violin_naive" => Ok(violin_naive()),
        "piano_naive" => Ok(piano_naive()),
        "pluck_naive" => Ok(pluck_naive()),
        "brass_naive" => Ok(brass_naive()),
        "organ_naive" => Ok(organ_naive()),
        "mallet_naive" => Ok(mallet_naive()),
        "celesta_naive" => Ok(celesta_naive()),
        "glockenspiel_naive" => Ok(glockenspiel_naive()),
        "vibraphone_naive" => Ok(vibraphone_naive()),
        "marimba_naive" => Ok(marimba_naive()),
        "xylophone_naive" => Ok(xylophone_naive()),
        "tubular_bells_naive" => Ok(tubular_bells_naive()),
        "timpani_naive" => Ok(timpani_naive()),
        "kalimba_naive" => Ok(kalimba_naive()),
        "synth_lead_naive" => Ok(synth_lead_naive()),
        "synth_pad_naive" => Ok(synth_pad_naive()),
        "effect_naive" => Ok(effect_naive()),
        _ => Err(InstrumentError::UnknownInstrument(id.to_string())),
    }
}
