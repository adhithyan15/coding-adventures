//! Equal-tempered frequency to note inversion (MUS08, MUS00).

use crate::types::{
    validate_positive_float, AnalysisError, NotePitch, DEFAULT_A4_HZ, MAX_SAMPLE_RATE_HZ,
};

/// Canonical chromatic pitch class names in ascending semitone order.
pub const PITCH_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

/// Convert a positive frequency in Hz to nearest note name, MIDI index, and cents deviation.
///
/// # Arguments
/// * `hz` - Fundamental frequency in Hertz (> 0.0, finite, <= 96,000 Hz).
/// * `a4_hz` - Tuning reference frequency for A4 in Hertz (default 440.0 Hz, <= 10,000 Hz).
///
/// # Errors
/// Returns [`AnalysisError::InvalidInput`] if `hz` or `a4_hz` is non-positive or non-finite.
/// Returns [`AnalysisError::BudgetExceeded`] if `hz` or `a4_hz` exceeds allowed maximums.
pub fn frequency_to_note(hz: f64, a4_hz: f64) -> Result<NotePitch, AnalysisError> {
    let valid_hz = validate_positive_float(hz, "hz", Some(MAX_SAMPLE_RATE_HZ / 2.0))?;
    let valid_a4 = validate_positive_float(a4_hz, "a4_hz", Some(10_000.0))?;

    // Invert equal-tempered frequency formula: f = a4 * 2^((midi - 69) / 12)
    let semitones_from_a4 = 12.0 * (valid_hz / valid_a4).log2();
    let exact_midi = 69.0 + semitones_from_a4;
    let nearest_midi = (exact_midi + 0.5).floor() as i32;
    let cents = (exact_midi - nearest_midi as f64) * 100.0;

    let pitch_class = PITCH_NAMES[nearest_midi.rem_euclid(12) as usize];
    let octave = nearest_midi.div_euclid(12) - 1;
    let note_name = format!("{pitch_class}{octave}");

    let rounded_cents = (cents * 100.0).round() / 100.0;

    Ok(NotePitch {
        note: note_name,
        midi: nearest_midi,
        cents: rounded_cents,
    })
}

/// Convert frequency to note using the standard 440.0 Hz tuning reference.
pub fn frequency_to_note_a440(hz: f64) -> Result<NotePitch, AnalysisError> {
    frequency_to_note(hz, DEFAULT_A4_HZ)
}

/// Convert frequency to note with an optional A4 tuning reference (defaults to 440.0 Hz).
pub fn frequency_to_note_opt(hz: f64, a4_hz: Option<f64>) -> Result<NotePitch, AnalysisError> {
    frequency_to_note(hz, a4_hz.unwrap_or(DEFAULT_A4_HZ))
}
