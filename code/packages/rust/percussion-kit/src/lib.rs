//! # MUS07: Unpitched Percussion and Drum Kit Events
//!
//! MUS05 defines pitched instruments (note frequency + harmonic profile -> waveform).
//! While that model works for melodic instruments and pitched percussion (glockenspiel, marimba),
//! it does not describe drum kit and percussion sounds.
//!
//! A snare is not naturally "A4". A hi-hat is not "C#5". A shaker is not "one stable
//! frequency plus harmonics".
//!
//! This crate implements the unpitched percussion model specified in MUS07:
//!
//! ```text
//! percussion hit + kit voice profile -> transient + noise + resonances -> shaped waveform -> PCM audio
//! ```
//!
//! ## Core Architecture
//!
//! - **All 15 Portable Starter Kit Voices** (`kick`, `snare`, `closed_hihat`, `open_hihat`,
//!   `pedal_hihat`, `low_tom`, `mid_tom`, `high_tom`, `crash`, `ride`, `rimshot`, `clap`,
//!   `cowbell`, `tambourine`, `shaker`, `woodblock`) + canonical alias resolution.
//! - **Deterministic Noise Synthesis**: Deterministic hash-based pseudo-noise and [`SplitMix64`]
//!   PRNG noise synthesis with zero ambient entropy or external RNG state.
//! - **Hi-Hat Choke Group Dynamics**: 15 ms linear release ramp muting of active open hi-hats
//!   when struck by closed, pedal, or repeated open hi-hat hits.
//! - **Streaming Sequence Rendering**: Bounded memory synthesis mixing events iteratively into
//!   the timeline without buffering separate tracks.
//!
//! ## Verification & Security Guarantees
//!
//! - **Safety**: `#![forbid(unsafe_code)]`, zero third-party dependencies (only `std`, `oscillator`, and `pcm-audio`).
//! - **CWE-400 (Resource Exhaustion)**:
//!   - `MAX_PERCUSSION_MODES = 32`
//!   - `MAX_DURATION_SECONDS = 60.0`
//!   - `MAX_SAMPLE_RATE_HZ = 192_000.0`
//!   - `MAX_SAMPLE_COUNT = 16_777_216` (2^24)
//!   - `MAX_HITS = 65_536`
//!   - Pre-flight budget verification strictly before buffer allocation.
//! - **CWE-190 (Integer Overflow)**: Safe rounding and checked arithmetic on sample counts and timeline indices.
//! - **CWE-20 (Improper Input Validation)**: Finite-float checks rejecting `NaN` and `±Inf`.
//! - **CWE-674 (Uncontrolled Recursion)**: Strictly iterative synthesis routines with provably constant stack depth.

#![forbid(unsafe_code)]

pub mod choke;
pub mod envelope;
pub mod error;
pub mod kit;
pub mod mode;
pub mod noise;
pub mod profile;
pub mod render;
pub mod signal;
pub mod splitmix64;

// Re-exports
pub use choke::{apply_hihat_choke, is_choked_by};
pub use envelope::PercussionEnvelope;
pub use error::{
    safe_round_sample, validate_duration, validate_finite, validate_non_negative,
    validate_positive, validate_sample_rate, validate_unit_float, PercussionError,
    CHOKE_RELEASE_RAMP_SECONDS, DEFAULT_MAX_MODE_COUNT, DEFAULT_MAX_SAMPLE_COUNT,
    DEFAULT_SAMPLE_RATE_HZ, INTEGER_TOLERANCE, MAX_DURATION_SECONDS, MAX_HITS,
    MAX_HIT_DURATION_SECONDS, MAX_PERCUSSION_MODES, MAX_SAMPLE_COUNT, MAX_SAMPLE_RATE_HZ,
    MAX_SEQUENCE_EVENTS,
};
pub use kit::{
    all_drum_kits, all_standard_hit_ids, canonical_hit_id, clap_naive, closed_hihat_naive,
    cowbell_naive, crash_naive, get_drum_kit, get_percussion_voice, high_tom_naive, kick_naive,
    low_tom_naive, mid_tom_naive, open_hihat_naive, pedal_hihat_naive, ride_naive, rimshot_naive,
    shaker_naive, snare_naive, standard_kit, tambourine_naive, woodblock_naive,
    DrumKitProfile, ALL_STANDARD_HIT_IDS,
};
pub use mode::PercussionMode;
pub use noise::{pseudo_noise, NoiseAlgorithm, PercussionNoiseProfile};
pub use profile::PercussionVoiceProfile;
pub use render::{
    render_percussion_hit, render_percussion_hit_pcm, render_percussion_sequence,
    render_percussion_sequence_pcm, HitRenderOptions, PercussionHitEvent, PercussionHitRender,
    PercussionSequenceRender, SequenceRenderOptions,
};
pub use signal::PercussionSignal;
pub use splitmix64::{splitmix64_noise, SplitMix64};

// ============================================================================
// Warrant Operations (matching percussion-kit.warrant specifications)
// ============================================================================

/// Model/operation for warrant specification: calculate sample count for a given duration in milliseconds and sample rate.
pub fn calc_hit_sample_count(dur_ms: i64, rate: i64) -> Option<i64> {
    if dur_ms < 0 || rate <= 0 || rate > (MAX_SAMPLE_RATE_HZ as i64) {
        return None;
    }
    let count = dur_ms.checked_mul(rate)?.checked_div(1000)?;
    if count > (MAX_SAMPLE_COUNT as i64) {
        return None;
    }
    Some(count)
}

/// Model/operation for warrant specification: verify that mode count does not exceed upper bound.
pub fn is_mode_count_safe(modes: i64) -> bool {
    modes >= 0 && (modes as usize) <= MAX_PERCUSSION_MODES
}
