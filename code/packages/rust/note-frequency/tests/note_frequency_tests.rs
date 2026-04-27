use coding_adventures_note_frequency::{note_to_frequency, parse_note, Note};

fn approx_equal(left: f64, right: f64) {
    assert!(
        (left - right).abs() < 1e-12,
        "expected {left} to equal {right}"
    );
}

#[test]
fn parse_note_extracts_components() {
    let note = parse_note("C#5").unwrap();
    assert_eq!(note.letter, "C");
    assert_eq!(note.accidental, "#");
    assert_eq!(note.octave, 5);
}

#[test]
fn lowercase_note_is_normalized() {
    assert_eq!(parse_note("g4").unwrap().to_string(), "G4");
}

#[test]
fn malformed_notes_are_rejected() {
    for value in ["", "A", "H4", "#4", "4A", "A##4", "Bb", "A+4", "A 4"] {
        assert!(parse_note(value).is_err(), "expected {value:?} to fail");
    }
}

#[test]
fn unsupported_spelling_is_rejected() {
    assert!(Note::new("E", "#", 4).is_err());
}

#[test]
fn semitone_offsets_match_reference_examples() {
    assert_eq!(parse_note("A4").unwrap().semitones_from_a4(), 0);
    assert_eq!(parse_note("A5").unwrap().semitones_from_a4(), 12);
    assert_eq!(parse_note("A3").unwrap().semitones_from_a4(), -12);
    assert_eq!(parse_note("C4").unwrap().semitones_from_a4(), -9);
}

#[test]
fn frequency_mapping_matches_reference_examples() {
    approx_equal(parse_note("A4").unwrap().frequency(), 440.0);
    approx_equal(parse_note("A5").unwrap().frequency(), 880.0);
    approx_equal(parse_note("A3").unwrap().frequency(), 220.0);
}

#[test]
fn middle_c_matches_equal_temperament() {
    approx_equal(note_to_frequency("C4").unwrap(), 261.6255653005986);
}

#[test]
fn enharmonic_spellings_match() {
    approx_equal(
        note_to_frequency("C#4").unwrap(),
        note_to_frequency("Db4").unwrap(),
    );
}
