//! Unit tests for frequency_to_note conversion (MUS08, MUS00).

use coding_adventures_music_analysis::{frequency_to_note, frequency_to_note_a440, AnalysisError};
use note_frequency::note_to_frequency;

#[test]
fn test_frequency_to_note_standard_tuning() {
    let res = frequency_to_note_a440(440.0).expect("A440 should convert");
    assert_eq!(res.note, "A4");
    assert_eq!(res.midi, 69);
    assert!((res.cents - 0.0).abs() < 0.1);
}

#[test]
fn test_frequency_to_note_c4_middle_c() {
    // C4 frequency in equal temperament
    let c4_hz = 440.0 * 2.0f64.powf(-9.0 / 12.0);
    let res = frequency_to_note_a440(c4_hz).expect("C4 should convert");
    assert_eq!(res.note, "C4");
    assert_eq!(res.midi, 60);
    assert!((res.cents - 0.0).abs() < 0.1);
}

#[test]
fn test_frequency_to_note_octave_extremes() {
    // A0 (lowest piano note): 27.5 Hz
    let res_a0 = frequency_to_note_a440(27.5).expect("A0 should convert");
    assert_eq!(res_a0.note, "A0");
    assert_eq!(res_a0.midi, 21);

    // C8 (highest piano note): ~4186 Hz
    let c8_hz = 440.0 * 2.0f64.powf((108.0 - 69.0) / 12.0);
    let res_c8 = frequency_to_note_a440(c8_hz).expect("C8 should convert");
    assert_eq!(res_c8.note, "C8");
    assert_eq!(res_c8.midi, 108);
}

#[test]
fn test_frequency_to_note_alternate_reference() {
    // In European orchestral tuning A4 = 442.0 Hz
    let res = frequency_to_note(442.0, 442.0).expect("A4 at 442 Hz reference should convert");
    assert_eq!(res.note, "A4");
    assert_eq!(res.midi, 69);
    assert!((res.cents - 0.0).abs() < 0.1);
}

#[test]
fn test_frequency_to_note_cents_deviation() {
    // 445 Hz is sharp relative to 440 Hz (~19.56 cents)
    let res = frequency_to_note_a440(445.0).expect("445 Hz should convert");
    assert_eq!(res.note, "A4");
    assert_eq!(res.midi, 69);
    assert!(res.cents > 15.0 && res.cents < 25.0);

    // 435 Hz is flat relative to 440 Hz (~ -19.78 cents)
    let res_flat = frequency_to_note_a440(435.0).expect("435 Hz should convert");
    assert_eq!(res_flat.note, "A4");
    assert_eq!(res_flat.midi, 69);
    assert!(res_flat.cents < -15.0 && res_flat.cents > -25.0);
}

#[test]
fn test_frequency_to_note_round_trip_with_note_frequency() {
    let test_notes = ["C3", "D#4", "F#5", "A4", "B2", "G#3"];
    for &note_str in &test_notes {
        let hz = note_to_frequency(note_str).expect("valid note string");
        let matched = frequency_to_note_a440(hz).expect("should convert back to note");
        assert_eq!(matched.note, note_str);
        assert!((matched.cents - 0.0).abs() < 0.2);
    }
}

#[test]
fn test_frequency_to_note_invalid_inputs() {
    assert!(matches!(
        frequency_to_note_a440(0.0),
        Err(AnalysisError::InvalidInput(_))
    ));
    assert!(matches!(
        frequency_to_note_a440(-440.0),
        Err(AnalysisError::InvalidInput(_))
    ));
    assert!(matches!(
        frequency_to_note_a440(f64::NAN),
        Err(AnalysisError::InvalidInput(_))
    ));
    assert!(matches!(
        frequency_to_note_a440(f64::INFINITY),
        Err(AnalysisError::InvalidInput(_))
    ));
    assert!(matches!(
        frequency_to_note(440.0, 0.0),
        Err(AnalysisError::InvalidInput(_))
    ));
    assert!(matches!(
        frequency_to_note(440.0, -440.0),
        Err(AnalysisError::InvalidInput(_))
    ));
}
